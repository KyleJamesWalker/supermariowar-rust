//! A netplay room: players with their skins and teams, this client's player count, chat, map preview, and Start for
//! the host.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;

use crate::common::path::{file_exists, get_home_directory};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_map_preview::MI_MapPreview;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::{NET_MAX_CHAT_MSG_LENGTH, NET_MAX_LOCAL_PLAYERS};
use crate::globals::Ptr;
use crate::smw::ui::mi_chat_message_box::MI_ChatMessageBox;
use crate::smw::ui::mi_net_room_team_select::MI_NetRoomTeamSelect;
use std::path::Path;

/// Player slots, chat, map preview and Start for a joined room.
#[derive(Default)]
pub struct UI_NetRoomMenu {
    pub ui_menu: UI_Menu,

    miNetRoomName: Ctl<MI_Text>,
    miNetRoomPlayerName: [Ctl<MI_Text>; 4],
    miNetRoomPlayerTeam: [Ctl<MI_Text>; 4],
    /// One per player slot; a skin change sends that local player's skin to the room, left and right its team.
    miSkinSelector: [Ctl<MI_NetRoomTeamSelect>; 4],
    /// How many players on this machine's controllers join the room.
    miLocalPlayersField: Ctl<MI_SelectField<i16>>,

    miNetRoomMessages: Ctl<MI_ChatMessageBox>,
    miNetRoomMessageField: Ctl<MI_TextField>,
    /// Map and mode settings; visible to the host only.
    miNetRoomSettingsButton: Ctl<MI_Button>,
    miNetRoomMapPreview: Ctl<MI_MapPreview>,

    /// Reads Start for the host once another player joined, else "(waiting)".
    miNetRoomStartButton: Ctl<MI_Button>,
    miNetRoomSendButton: Ctl<MI_Button>,
    miNetRoomBackButton: Ctl<MI_Button>,

    miNetRoomStartingDialogImage: Ctl<MI_Image>,
    miNetRoomStartingDialogText: Ctl<MI_Text>,

    miNetRoomLeftHeaderBar: Ctl<MI_Image>,
    miNetRoomRightHeaderBar: Ctl<MI_Image>,
    miNetRoomHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_NetRoomMenu => ui_menu: UI_Menu);

impl UI_NetRoomMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        let selectfield = g.rm.spr_selectfield;
        let plain = g.rm.menu_plain_field;

        this.miNetRoomName = Ctl::new(MI_Text::new(g, "", 40, 50, 0, true, TextAlign::LEFT), g);

        this.miNetRoomStartButton = Ctl::new(MI_Button::new(g, selectfield, 300, 310, "(waiting)", 331, TextAlign::CENTER), g);
        g.ctl(this.miNetRoomStartButton).set_code(MENU_CODE_TO_NET_ROOM_START_IN_PROGRESS);

        this.miLocalPlayersField = Ctl::new(MI_SelectField::<i16>::new(selectfield, 16, 310, "Players", 272, 160), g);
        for count in 1..=NET_MAX_LOCAL_PLAYERS as i16 {
            g.ctl(this.miLocalPlayersField).add(count.to_string(), count);
        }
        g.ctl(this.miLocalPlayersField).set_output_ptr(&mut g.netplay.localPlayers as *mut _);
        g.ctl(this.miLocalPlayersField).set_item_changed_code(MENU_CODE_NET_ROOM_LOCAL_PLAYERS);

        this.miNetRoomSettingsButton = Ctl::new(MI_Button::new(g, selectfield, 300, 270, "Map & Mode", 331, TextAlign::CENTER), g);
        g.ctl(this.miNetRoomSettingsButton).set_code(MENU_CODE_NET_ROOM_SETTINGS);
        g.ctl(this.miNetRoomSettingsButton).set_visible(false);

        this.miNetRoomMessages = Ctl::new(MI_ChatMessageBox::new(20, 350, 640 - 2 * 26, 1), g);
        this.miNetRoomMapPreview = Ctl::new(MI_MapPreview::new(g, selectfield, 264, 2, 400, 120), g);

        this.miNetRoomMessageField = Ctl::new(MI_TextField::new(plain, 26, 432, "Say", 464 - 36, 60), g);
        g.ctl(this.miNetRoomMessageField).set_data(g, &mut gp.netplay.mychatmessage, NET_MAX_CHAT_MSG_LENGTH as i16);

        this.miNetRoomSendButton = Ctl::new(MI_Button::new(g, selectfield, 464, 432, "Send", 80, TextAlign::CENTER), g);
        g.ctl(this.miNetRoomSendButton).set_code(MENU_CODE_NET_CHAT_SEND);

        this.miNetRoomBackButton = Ctl::new(MI_Button::new(g, selectfield, 544, 432, "Leave", 80, TextAlign::CENTER), g);
        g.ctl(this.miNetRoomBackButton).set_code(MENU_CODE_TO_NET_LOBBY_MENU);

        let field = this.miNetRoomMessageField.id();
        let start = this.miNetRoomStartButton.id();
        let null = ControlId::NONE;
        for p in 0..4i16 {
            this.miNetRoomPlayerName[p as usize] = Ctl::new(MI_Text::new(g, "", 60, 80 + p * 60, 0, true, TextAlign::LEFT), g);
            let name = this.miNetRoomPlayerName[p as usize].id();
            this.add_non_control(name);
            this.miNetRoomPlayerTeam[p as usize] = Ctl::new(MI_Text::new(g, "", 200, 84 + p * 60, 0, false, TextAlign::LEFT), g);
            let team = this.miNetRoomPlayerTeam[p as usize].id();
            this.add_non_control(team);

            this.miSkinSelector[p as usize] = Ctl::new(MI_NetRoomTeamSelect::new(
                g,
                16,
                72 + p * 60,
                p,
                Box::new(move |g: &mut Game| {
                    let mut gp = g.ptr();
                    if let Some(local) = g.netplay.local_index(p as usize) {
                        gp.netplay.client.send_skin_change(g, local)
                    }
                }),
                Box::new(move |g: &mut Game, step: i16| {
                    let mut gp = g.ptr();
                    if let Some(local) = g.netplay.local_index(p as usize) {
                        gp.netplay.client.send_team_step(g, local, step)
                    }
                }),
            ), g);
            let sel = this.miSkinSelector[p as usize].id();
            this.add_control(g, sel, field, start, null, null);
        }

        let (msgs, preview) = (this.miNetRoomMessages.id(), this.miNetRoomMapPreview.id());
        this.add_non_control(msgs);
        this.add_non_control(preview);

        let send = this.miNetRoomSendButton.id();
        let back = this.miNetRoomBackButton.id();
        let players = this.miLocalPlayersField.id();
        this.add_control(g, start, field, field, players, field);
        let settings = this.miNetRoomSettingsButton.id();
        this.add_control(g, settings, null, start, players, null);
        this.add_control(g, players, null, start, null, start);
        this.add_control(g, field, start, start, back, send);
        this.add_control(g, send, start, start, field, back);
        this.add_control(g, back, start, start, send, field);

        this.miNetRoomStartingDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miNetRoomStartingDialogText = Ctl::new(MI_HeaderText::new(g, "Starting...", 320, 240 - 40), g);

        g.ctl(this.miNetRoomStartingDialogImage).set_visible(false);
        g.ctl(this.miNetRoomStartingDialogText).set_visible(false);

        let (di, dt) = (this.miNetRoomStartingDialogImage.id(), this.miNetRoomStartingDialogText.id());
        this.add_non_control(di);
        this.add_non_control(dt);

        let plain = g.rm.menu_plain_field;
        this.miNetRoomLeftHeaderBar = Ctl::new(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miNetRoomRightHeaderBar = Ctl::new(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miNetRoomHeaderText = Ctl::new(MI_HeaderText::new(g, "Multiplayer Room Menu", 320, 5), g);

        let (l, r, h) = (this.miNetRoomLeftHeaderBar.id(), this.miNetRoomRightHeaderBar.id(), this.miNetRoomHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, field);
        this.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
        this
    }

    /// Empties the message field once its message went out.
    pub fn chat_sent(&mut self, g: &mut Game) {
        g.ctl(self.miNetRoomMessageField).clear();
    }

    /// Redraws the room from `netplay.currentRoom`: names, skins, host-only controls and the map preview.
    pub fn refresh(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let name = g.netplay.currentRoom.name.clone();
        g.ctl(self.miNetRoomHeaderText).set_text(name);

        // Down runs from the chat field through this client's skins and Players back to the chat field; Map & Mode and
        // Start, beside Players, sit under the last skin.
        let mine: Vec<ControlId> = g.netplay.localSlots.iter().map(|&slot| self.miSkinSelector[slot as usize].id()).collect();
        let host = g.netplay.theHostIsMe;
        let (players, settings, start, field) =
            (self.miLocalPlayersField.id(), self.miNetRoomSettingsButton.id(), self.miNetRoomStartButton.id(), self.miNetRoomMessageField.id());
        g.ctl(self.miNetRoomSettingsButton).set_visible(host);
        let mut above = field;
        for &sel in mine.iter().chain([&players]) {
            g.control(above).ctl_mut().set_neighbor(MenuNavDirection::Down, sel);
            g.control(sel).ctl_mut().set_neighbor(MenuNavDirection::Up, above);
            above = sel;
        }
        g.ctl(self.miLocalPlayersField).set_neighbor(MenuNavDirection::Down, field);
        let last_skin = *mine.last().unwrap_or(&field);
        g.ctl(self.miNetRoomSettingsButton).set_neighbor(MenuNavDirection::Up, last_skin);
        g.ctl(self.miNetRoomStartButton).set_neighbor(MenuNavDirection::Up, if host { settings } else { last_skin });
        g.ctl(self.miLocalPlayersField).set_current_value(g.netplay.localPlayers);

        for p in 0..4i16 {
            let pname = g.netplay.currentRoom.playerNames[p as usize].clone();
            g.ctl(self.miNetRoomPlayerName[p as usize]).set_text(pname);
            let room = &g.netplay.currentRoom;
            let team = if room.taken(p as usize) { format!("Team {}", room.teams[p as usize] + 1) } else { String::new() };
            g.ctl(self.miNetRoomPlayerTeam[p as usize]).set_text(team);
            let color = crate::smw::net::room_skin_color(g, p as usize);

            // Other players' skins are the files the client wrote when their skin messages arrived.
            let path = format!("{}net_skin{}.bmp", get_home_directory(g), p);
            if g.netplay.is_local_slot(p as usize) {
                gp.get().rm.load_menu_skin(g, p, gp.get().game_values.skinids[p as usize], color, false);
            } else if file_exists(&path) {
                if !gp.rm.load_menu_skin_path(g, p, Path::new(&path), color, false) {
                    gp.get().rm.load_menu_skin(g, p, gp.get().game_values.skinids[p as usize], color, false);
                }
            } else {
                gp.rm.load_menu_skin(g, p, 0, color, false);
            }
        }

        if can_start(g) {
            g.ctl(self.miNetRoomStartButton).set_name(g, "Start");
        } else {
            g.ctl(self.miNetRoomStartButton).set_name(g, "(waiting)");
        }

        let path = g.netplay.mapfilepath.clone();
        self.set_preview_map_path(g, &path);
    }

    /// Host only, with another client in the room: sends the start request and shows the Starting dialog.
    pub fn start_in_progress(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if can_start(g) {
            gp.netplay.client.send_start_room_message(g);
            g.netplay.operationInProgress = true;

            g.ctl(self.miNetRoomStartingDialogImage).set_visible(true);
            g.ctl(self.miNetRoomStartingDialogText).set_visible(true);
            self.remember_current();

            let t = self.miNetRoomStartingDialogText.id();
            self.set_initial_focus(g, t);
            self.set_cancel_code(MENU_CODE_TO_NET_ROOM_MENU);
            self.reset_menu(g);
        }
    }

    /// Hides the Starting dialog and focuses the chat field.
    pub fn restore(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.ctl(self.miNetRoomStartingDialogImage).set_visible(false);
        g.ctl(self.miNetRoomStartingDialogText).set_visible(false);

        let f = self.miNetRoomMessageField.id();
        self.set_initial_focus(gp.get(), f);
        self.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
    }

    /// Loads the map at `path` into the preview.
    pub fn set_preview_map_path(&mut self, g: &mut Game, path: &str) {
        g.ctl(self.miNetRoomMapPreview).load_map(g, path);
    }
}

/// The host can start once another client joined; its own local players alone are no online match.
fn can_start(g: &Game) -> bool {
    g.netplay.theHostIsMe && g.netplay.currentRoom.player_count() as usize > g.netplay.localSlots.len()
}
