//! Port of src/smw/menu/network/NetRoomMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;

use crate::common::path::{file_exists, get_home_directory};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_map_preview::MI_MapPreview;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::NET_MAX_CHAT_MSG_LENGTH;
use crate::globals::Ptr;
use crate::smw::ui::mi_chat_message_box::MI_ChatMessageBox;
use crate::smw::ui::mi_net_room_team_select::MI_NetRoomTeamSelect;
use std::path::Path;

#[derive(Default)]
pub struct UI_NetRoomMenu {
    pub ui_menu: UI_Menu,

    miNetRoomName: Ctl<MI_Text>,
    miNetRoomPlayerName: [Ctl<MI_Text>; 4],
    miSkinSelector: [Ctl<MI_NetRoomTeamSelect>; 4],

    miNetRoomMessages: Ctl<MI_ChatMessageBox>,
    miNetRoomMessageField: Ctl<MI_TextField>,
    miNetRoomSettingsButton: Ctl<MI_Button>,
    miNetRoomMapPreview: Ctl<MI_MapPreview>,

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

            this.miSkinSelector[p as usize] = Ctl::new(MI_NetRoomTeamSelect::new(
                g,
                16,
                72 + p * 60,
                p,
                Box::new(|g: &mut Game| {
                    let mut gp = g.ptr();
                    gp.netplay.client.send_skin_change(g)
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
        this.add_control(g, start, field, field, field, field);
        let settings = this.miNetRoomSettingsButton.id();
        this.add_control(g, settings, null, start, null, null);
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

    pub fn refresh(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let name = g.netplay.currentRoom.name.clone();
        g.ctl(self.miNetRoomHeaderText).set_text(name);

        let me = self.miSkinSelector[g.netplay.remotePlayerNumber as usize].id();
        let host = g.netplay.theHostIsMe;
        g.ctl(self.miNetRoomSettingsButton).set_visible(host);
        let above_start = if host { self.miNetRoomSettingsButton.id() } else { me };
        g.ctl(self.miNetRoomStartButton).set_neighbor(MenuNavDirection::Up, above_start);
        g.ctl(self.miNetRoomSettingsButton).set_neighbor(MenuNavDirection::Up, me);
        g.ctl(self.miNetRoomMessageField).set_neighbor(MenuNavDirection::Down, me);

        for p in 0..4i16 {
            let pname = g.netplay.currentRoom.playerNames[p as usize].clone();
            g.ctl(self.miNetRoomPlayerName[p as usize]).set_text(pname);

            let path = format!("{}net_skin{}.bmp", get_home_directory(g), p);
            if p == g.netplay.remotePlayerNumber as i16 {
                println!("  player {} -> local", p);
                gp.get().rm.load_menu_skin(g, p, gp.get().game_values.skinids[0], p, false);
            } else if file_exists(&path) {
                println!("  player {} -> {}", p, path);
                if !gp.rm.load_menu_skin_path(g, p, Path::new(&path), p, false) {
                    gp.get().rm.load_menu_skin(g, p, gp.get().game_values.skinids[p as usize], p, false);
                }
            } else {
                println!("  player {} -> default", p);
                gp.rm.load_menu_skin(g, p, 0, p, false);
            }
        }

        if g.netplay.theHostIsMe && g.netplay.currentRoom.player_count() > 1 {
            g.ctl(self.miNetRoomStartButton).set_name(g, "Start");
        } else {
            g.ctl(self.miNetRoomStartButton).set_name(g, "(waiting)");
        }

        let path = g.netplay.mapfilepath.clone();
        self.set_preview_map_path(g, &path);
    }

    pub fn start_in_progress(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.netplay.theHostIsMe && g.netplay.currentRoom.player_count() > 1 {
            gp.netplay.client.local_gamehost.send_start_room_message(g);
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

    pub fn restore(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.ctl(self.miNetRoomStartingDialogImage).set_visible(false);
        g.ctl(self.miNetRoomStartingDialogText).set_visible(false);

        let f = self.miNetRoomMessageField.id();
        self.set_initial_focus(gp.get(), f);
        self.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
    }

    pub fn set_preview_map_path(&mut self, g: &mut Game, path: &str) {
        g.ctl(self.miNetRoomMapPreview).load_map(g, path);
    }
}
