//! Port of src/smw/menu/network/NetRoomMenu.cpp

use crate::game::Game;

use crate::common::path::{file_exists, get_home_directory};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_map_preview::MI_MapPreview;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::NET_MAX_CHAT_MSG_LENGTH;
use crate::globals::Ptr;
use crate::smw::ui::mi_chat_message_box::MI_ChatMessageBox;
use crate::smw::ui::mi_net_room_team_select::MI_NetRoomTeamSelect;
use std::path::Path;

#[derive(Default)]
pub struct UI_NetRoomMenu {
    pub ui_menu: UI_Menu,

    miNetRoomName: Ptr<MI_Text>,
    miNetRoomPlayerName: [Ptr<MI_Text>; 4],
    miSkinSelector: [Ptr<MI_NetRoomTeamSelect>; 4],

    miNetRoomMessages: Ptr<MI_ChatMessageBox>,
    miNetRoomMessageField: Ptr<MI_TextField>,
    miNetRoomMapPreview: Ptr<MI_MapPreview>,

    miNetRoomStartButton: Ptr<MI_Button>,
    miNetRoomSendButton: Ptr<MI_Button>,
    miNetRoomBackButton: Ptr<MI_Button>,

    miNetRoomStartingDialogImage: Ptr<MI_Image>,
    miNetRoomStartingDialogText: Ptr<MI_Text>,

    miNetRoomLeftHeaderBar: Ptr<MI_Image>,
    miNetRoomRightHeaderBar: Ptr<MI_Image>,
    miNetRoomHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_NetRoomMenu => ui_menu: UI_Menu);

impl UI_NetRoomMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut gp = g.ptr();
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        unsafe {
            let selectfield = Ptr::from_mut(&mut g.rm.spr_selectfield);
            let plain = Ptr::from_mut(&mut g.rm.menu_plain_field);

            this.miNetRoomName = Ptr::new_box(MI_Text::new(g, "", 40, 50, 0, true, TextAlign::LEFT));

            this.miNetRoomStartButton = Ptr::new_box(MI_Button::new(g, selectfield, 300, 310, "(waiting)", 331, TextAlign::CENTER));
            this.miNetRoomStartButton.set_code(MENU_CODE_TO_NET_ROOM_START_IN_PROGRESS);

            this.miNetRoomMessages = Ptr::new_box(MI_ChatMessageBox::new(20, 350, 640 - 2 * 26, 1));
            this.miNetRoomMapPreview = Ptr::new_box(MI_MapPreview::new(g, selectfield, 264, 2, 400, 120));

            this.miNetRoomMessageField = Ptr::new_box(MI_TextField::new(plain, 26, 432, "Say", 464 - 36, 60));
            this.miNetRoomMessageField.set_data(g, &mut gp.netplay.mychatmessage, NET_MAX_CHAT_MSG_LENGTH as i16);

            this.miNetRoomSendButton = Ptr::new_box(MI_Button::new(g, selectfield, 464, 432, "Send", 80, TextAlign::CENTER));
            this.miNetRoomSendButton.set_code(MENU_CODE_NET_CHAT_SEND);

            this.miNetRoomBackButton = Ptr::new_box(MI_Button::new(g, selectfield, 544, 432, "Leave", 80, TextAlign::CENTER));
            this.miNetRoomBackButton.set_code(MENU_CODE_TO_NET_LOBBY_MENU);
        }

        let field = ctl_ptr(this.miNetRoomMessageField);
        let start = ctl_ptr(this.miNetRoomStartButton);
        let null = Ptr::null();
        for p in 0..4i16 {
            this.miNetRoomPlayerName[p as usize] = Ptr::new_box(MI_Text::new(g, "", 60, 80 + p * 60, 0, true, TextAlign::LEFT));
            let name = ctl_ptr(this.miNetRoomPlayerName[p as usize]);
            this.add_non_control(name);

            this.miSkinSelector[p as usize] = Ptr::new_box(MI_NetRoomTeamSelect::new(
                g,
                16,
                72 + p * 60,
                p,
                Box::new(|g: &mut Game| {
                    let mut gp = g.ptr();
                    gp.netplay.client.send_skin_change(g)
                }),
            ));
            let sel = ctl_ptr(this.miSkinSelector[p as usize]);
            this.add_control(sel, field, start, null, null);
        }

        let (msgs, preview) = (ctl_ptr(this.miNetRoomMessages), ctl_ptr(this.miNetRoomMapPreview));
        this.add_non_control(msgs);
        this.add_non_control(preview);

        let send = ctl_ptr(this.miNetRoomSendButton);
        let back = ctl_ptr(this.miNetRoomBackButton);
        this.add_control(start, field, field, field, field);
        this.add_control(field, start, start, back, send);
        this.add_control(send, start, start, field, back);
        this.add_control(back, start, start, send, field);

        unsafe {
            this.miNetRoomStartingDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut g.rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miNetRoomStartingDialogText = Ptr::new_box(MI_HeaderText::new(g, "Starting...", 320, 240 - 40));
        }

        this.miNetRoomStartingDialogImage.set_visible(false);
        this.miNetRoomStartingDialogText.set_visible(false);

        let (di, dt) = (ctl_ptr(this.miNetRoomStartingDialogImage), ctl_ptr(this.miNetRoomStartingDialogText));
        this.add_non_control(di);
        this.add_non_control(dt);

        unsafe {
            let plain = Ptr::from_mut(&mut g.rm.menu_plain_field);
            this.miNetRoomLeftHeaderBar = Ptr::new_box(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miNetRoomRightHeaderBar = Ptr::new_box(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miNetRoomHeaderText = Ptr::new_box(MI_HeaderText::new(g, "Multiplayer Room Menu", 320, 5));
        }

        let (l, r, h) = (ctl_ptr(this.miNetRoomLeftHeaderBar), ctl_ptr(this.miNetRoomRightHeaderBar), ctl_ptr(this.miNetRoomHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, field);
        this.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
        this
    }

    pub fn refresh(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut gp = g.ptr();
        unsafe {
            let name = g.netplay.currentRoom.name.clone();
            self.miNetRoomHeaderText.set_text(name);

            let me = ctl_ptr(self.miSkinSelector[g.netplay.remotePlayerNumber as usize]);
            self.miNetRoomStartButton.set_neighbor(MenuNavDirection::Up, me);
            self.miNetRoomMessageField.set_neighbor(MenuNavDirection::Down, me);

            for p in 0..4i16 {
                let pname = g.netplay.currentRoom.playerNames[p as usize].clone();
                self.miNetRoomPlayerName[p as usize].set_text(pname);

                let path = format!("{}net_skin{}.bmp", get_home_directory(), p);
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
                self.miNetRoomStartButton.set_name(g, "Start");
            } else {
                self.miNetRoomStartButton.set_name(g, "(waiting)");
            }

            let path = g.netplay.mapfilepath.clone();
            self.set_preview_map_path(g, &path);
        }
    }

    pub fn start_in_progress(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        unsafe {
            if g.netplay.theHostIsMe && g.netplay.currentRoom.player_count() > 1 {
                gp.netplay.client.local_gamehost.send_start_room_message(g);
                g.netplay.operationInProgress = true;

                self.miNetRoomStartingDialogImage.set_visible(true);
                self.miNetRoomStartingDialogText.set_visible(true);
                self.remember_current();

                let t = ctl_ptr(self.miNetRoomStartingDialogText);
                self.set_initial_focus(g, t);
                self.set_cancel_code(MENU_CODE_TO_NET_ROOM_MENU);
                self.reset_menu(g);
            }
        }
    }

    pub fn restore(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.miNetRoomStartingDialogImage.set_visible(false);
        self.miNetRoomStartingDialogText.set_visible(false);

        let f = ctl_ptr(self.miNetRoomMessageField);
        self.set_initial_focus(gp.get(), f);
        self.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
    }

    pub fn set_preview_map_path(&mut self, g: &mut Game, path: &str) {
        self.miNetRoomMapPreview.load_map(g, path);
    }
}
