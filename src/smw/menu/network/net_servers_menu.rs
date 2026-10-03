//! Port of src/smw/menu/network/NetServersMenu.cpp

use crate::common::global::rm;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::NET_MAX_PLAYER_NAME_LENGTH;
use crate::globals::Ptr;
use crate::smw::net::netplay;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;

#[derive(Default)]
pub struct UI_NetServersMenu {
    pub ui_menu: UI_Menu,

    miNetServersSelectButton: Ptr<MI_Button>,
    miNetServersSelectedHostText: Ptr<MI_Text>,
    miNetServersConnectButton: Ptr<MI_Button>,
    miNetServersAddRemoveButton: Ptr<MI_Button>,
    miNetServersNicknameField: Ptr<MI_TextField>,
    miNetServersBackButton: Ptr<MI_Button>,

    miNetServersScroll: Ptr<MI_NetworkListScroll>,

    miNetServersConnectingDialogImage: Ptr<MI_Image>,
    miNetServersConnectingDialogText: Ptr<MI_Text>,

    miNetServersLeftHeaderBar: Ptr<MI_Image>,
    miNetServersRightHeaderBar: Ptr<MI_Image>,
    miNetServersHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_NetServersMenu => ui_menu: UI_Menu);

impl UI_NetServersMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let plain = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miNetServersBackButton = Ptr::new_box(MI_Button::new(selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miNetServersBackButton.set_code(MENU_CODE_TO_MAIN_MENU);

            this.miNetServersLeftHeaderBar = Ptr::new_box(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miNetServersRightHeaderBar = Ptr::new_box(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miNetServersHeaderText = Ptr::new_box(MI_HeaderText::new("Multiplayer Servers Menu", 320, 5));

            this.miNetServersScroll = Ptr::new_box(MI_NetworkListScroll::new(
                plain,
                90,
                72,
                640 - 2 * 90,
                9,
                "Saved Servers",
                MENU_CODE_NET_SERVERLIST_EXIT,
                MENU_CODE_NET_SERVERLIST_EXIT,
            ));
            this.miNetServersScroll.set_auto_modify(true);
            this.miNetServersScroll.set_visible(false);
            this.miNetServersScroll.remote_index(&mut netplay.selectedServerIndex);

            this.miNetServersNicknameField = Ptr::new_box(MI_TextField::new(plain, 70, 120, "Your name", 640 - 2 * 70, 150));
            this.miNetServersNicknameField.set_data(&mut netplay.myPlayerName, NET_MAX_PLAYER_NAME_LENGTH as i16);

            for iServer in 0..netplay.savedServers.len() {
                let host = netplay.savedServers[iServer].hostname.clone();
                this.miNetServersScroll.add(&host, "");
            }

            this.miNetServersSelectButton = Ptr::new_box(MI_Button::new_left(selectfield, 70, 200, "Selected Server", 640 - 2 * 70));
            this.miNetServersSelectButton.set_code(MENU_CODE_TO_NET_SERVERLIST);
            if !netplay.savedServers.is_empty() {
                let host = netplay.savedServers[netplay.selectedServerIndex as usize].hostname.clone();
                this.miNetServersSelectedHostText = Ptr::new_box(MI_Text::new(host, 640 - 90, 205, 0, true, TextAlign::RIGHT));
            } else {
                this.miNetServersSelectedHostText = Ptr::new_box(MI_Text::new("(none)", 640 - 90, 205, 0, true, TextAlign::RIGHT));
            }

            this.miNetServersConnectButton = Ptr::new_box(MI_Button::new(selectfield, 70, 240, "Connect", 640 - 2 * 70, TextAlign::CENTER));
            this.miNetServersConnectButton.set_code(MENU_CODE_NET_CONNECT_IN_PROGRESS);

            this.miNetServersAddRemoveButton = Ptr::new_box(MI_Button::new(selectfield, 70, 280, "Add / Remove Server", 640 - 2 * 70, TextAlign::CENTER));
            this.miNetServersAddRemoveButton.set_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);

            this.miNetServersConnectingDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miNetServersConnectingDialogText = Ptr::new_box(MI_HeaderText::new("Connecting...", 640 / 2, 240 - 12));

            this.miNetServersConnectingDialogImage.set_visible(false);
            this.miNetServersConnectingDialogText.set_visible(false);
            this.miNetServersConnectingDialogText.fDisable = true;
        }

        let nick = ctl_ptr(this.miNetServersNicknameField);
        let select = ctl_ptr(this.miNetServersSelectButton);
        let connect = ctl_ptr(this.miNetServersConnectButton);
        let addremove = ctl_ptr(this.miNetServersAddRemoveButton);
        let back = ctl_ptr(this.miNetServersBackButton);
        let null = Ptr::null();

        this.add_control(nick, back, select, null, null);
        this.add_control(select, nick, connect, null, null);
        let t = ctl_ptr(this.miNetServersSelectedHostText);
        this.add_non_control(t);
        this.add_control(connect, select, addremove, null, null);
        this.add_control(addremove, connect, back, null, null);
        this.add_control(back, addremove, nick, null, null);

        let (l, r, h) = (ctl_ptr(this.miNetServersLeftHeaderBar), ctl_ptr(this.miNetServersRightHeaderBar), ctl_ptr(this.miNetServersHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        let (di, dt) = (ctl_ptr(this.miNetServersConnectingDialogImage), ctl_ptr(this.miNetServersConnectingDialogText));
        this.add_non_control(di);
        this.add_non_control(dt);

        let scroll = ctl_ptr(this.miNetServersScroll);
        this.add_control(scroll, null, null, null, null);

        this.set_initial_focus(select);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }

    pub fn refresh(&mut self) {
        unsafe {
            if !netplay.savedServers.is_empty() {
                let nethostname = netplay.savedServers[netplay.selectedServerIndex as usize].hostname.clone();
                self.miNetServersSelectedHostText.set_text(nethostname);
            }
        }
    }

    pub fn refresh_scroll(&mut self) {
        unsafe {
            self.miNetServersScroll.clear();
            for iServer in 0..netplay.savedServers.len() {
                let host = netplay.savedServers[iServer].hostname.clone();
                self.miNetServersScroll.add(&host, "");
            }

            if !netplay.savedServers.is_empty() {
                let host = netplay.savedServers[netplay.selectedServerIndex as usize].hostname.clone();
                self.miNetServersSelectedHostText.set_text(host);
            } else {
                self.miNetServersSelectedHostText.set_text("(none)");
            }
        }
    }

    pub fn connect_in_progress(&mut self) {
        unsafe {
            if netplay.savedServers.is_empty() {
                return;
            }

            netplay.client.send_connect_request_to_selected_server();
            netplay.operationInProgress = true;
        }

        self.miNetServersConnectingDialogImage.set_visible(true);
        self.miNetServersConnectingDialogText.set_visible(true);
        self.remember_current();

        let t = ctl_ptr(self.miNetServersConnectingDialogText);
        self.set_initial_focus(t);
        self.set_cancel_code(MENU_CODE_NET_CONNECT_ABORT);
        self.reset_menu();
    }

    pub fn open_server_list(&mut self) {
        unsafe { netplay.connectSuccessful = false };
        self.miNetServersScroll.set_visible(true);
        self.remember_current();

        let s = ctl_ptr(self.miNetServersScroll);
        self.set_initial_focus(s);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu();
    }

    pub fn restore(&mut self) {
        self.miNetServersScroll.set_visible(false);
        self.miNetServersConnectingDialogImage.set_visible(false);
        self.miNetServersConnectingDialogText.set_visible(false);

        let s = ctl_ptr(self.miNetServersSelectButton);
        self.set_initial_focus(s);
        self.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        self.restore_current();
    }
}
