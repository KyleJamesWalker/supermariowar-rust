//! The multiplayer entry menu: nickname, server choice, connect, edit servers and host on this device.
use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::NET_MAX_PLAYER_NAME_LENGTH;
use crate::globals::Ptr;
use crate::smw::net::net_save_server_list;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;

#[derive(Default)]
pub struct UI_NetServersMenu {
    pub ui_menu: UI_Menu,

    miNetServersSelectButton: Ctl<MI_Button>,
    miNetServersSelectedHostText: Ctl<MI_Text>,
    miNetServersConnectButton: Ctl<MI_Button>,
    miNetServersAddRemoveButton: Ctl<MI_Button>,
    miNetServersHostButton: Ctl<MI_Button>,
    miNetServersNicknameField: Ctl<MI_TextField>,
    miNetServersBackButton: Ctl<MI_Button>,

    /// Saved server picker popup; its selection writes `netplay.selectedServerIndex`.
    miNetServersScroll: Ctl<MI_NetworkListScroll>,

    miNetServersConnectingDialogImage: Ctl<MI_Image>,
    miNetServersConnectingDialogText: Ctl<MI_Text>,

    miNetServersLeftHeaderBar: Ctl<MI_Image>,
    miNetServersRightHeaderBar: Ctl<MI_Image>,
    miNetServersHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_NetServersMenu => ui_menu: UI_Menu);

impl UI_NetServersMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        let selectfield = g.rm.spr_selectfield;
        let plain = g.rm.menu_plain_field;

        this.miNetServersBackButton = Ctl::new(MI_Button::new(g, selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miNetServersBackButton).set_code(MENU_CODE_TO_MAIN_MENU);

        this.miNetServersLeftHeaderBar = Ctl::new(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miNetServersRightHeaderBar = Ctl::new(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miNetServersHeaderText = Ctl::new(MI_HeaderText::new(g, "Multiplayer Servers Menu", 320, 5), g);

        this.miNetServersScroll = Ctl::new(MI_NetworkListScroll::new(
            plain,
            90,
            72,
            640 - 2 * 90,
            9,
            "Saved Servers",
            MENU_CODE_NET_SERVERLIST_EXIT,
            MENU_CODE_NET_SERVERLIST_EXIT,
        ), g);
        g.ctl(this.miNetServersScroll).set_auto_modify(true);
        g.ctl(this.miNetServersScroll).set_visible(false);
        g.ctl(this.miNetServersScroll).remote_index(&mut g.netplay.selectedServerIndex);

        this.miNetServersNicknameField = Ctl::new(MI_TextField::new(plain, 70, 120, "Your name", 640 - 2 * 70, 150), g);
        g.ctl(this.miNetServersNicknameField).set_data(g, &mut gp.netplay.myPlayerName, NET_MAX_PLAYER_NAME_LENGTH as i16);

        for iServer in 0..g.netplay.savedServers.len() {
            let host = g.netplay.savedServers[iServer].hostname.clone();
            g.ctl(this.miNetServersScroll).add(&host, "");
        }

        this.miNetServersSelectButton = Ctl::new(MI_Button::new_left(g, selectfield, 70, 200, "Selected Server", 640 - 2 * 70), g);
        g.ctl(this.miNetServersSelectButton).set_code(MENU_CODE_TO_NET_SERVERLIST);
        if !g.netplay.savedServers.is_empty() {
            let host = g.netplay.savedServers[g.netplay.selectedServerIndex as usize].hostname.clone();
            this.miNetServersSelectedHostText = Ctl::new(MI_Text::new(g, host, 640 - 90, 205, 0, true, TextAlign::RIGHT), g);
        } else {
            this.miNetServersSelectedHostText = Ctl::new(MI_Text::new(g, "(none)", 640 - 90, 205, 0, true, TextAlign::RIGHT), g);
        }

        this.miNetServersConnectButton = Ctl::new(MI_Button::new(g, selectfield, 70, 240, "Connect", 640 - 2 * 70, TextAlign::CENTER), g);
        g.ctl(this.miNetServersConnectButton).set_code(MENU_CODE_NET_CONNECT_IN_PROGRESS);

        this.miNetServersAddRemoveButton = Ctl::new(MI_Button::new(g, selectfield, 70, 280, "Add / Remove Server", 640 - 2 * 70, TextAlign::CENTER), g);
        g.ctl(this.miNetServersAddRemoveButton).set_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);

        this.miNetServersHostButton = Ctl::new(MI_Button::new(g, selectfield, 70, 320, "Host on This Device", 640 - 2 * 70, TextAlign::CENTER), g);
        g.ctl(this.miNetServersHostButton).set_code(MENU_CODE_NET_HOST_LAN);

        this.miNetServersConnectingDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miNetServersConnectingDialogText = Ctl::new(MI_HeaderText::new(g, "Connecting...", 640 / 2, 240 - 12), g);

        g.ctl(this.miNetServersConnectingDialogImage).set_visible(false);
        g.ctl(this.miNetServersConnectingDialogText).set_visible(false);
        g.ctl(this.miNetServersConnectingDialogText).fDisable = true;

        let nick = this.miNetServersNicknameField.id();
        let select = this.miNetServersSelectButton.id();
        let connect = this.miNetServersConnectButton.id();
        let addremove = this.miNetServersAddRemoveButton.id();
        let back = this.miNetServersBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, nick, back, select, null, null);
        this.add_control(g, select, nick, connect, null, null);
        let t = this.miNetServersSelectedHostText.id();
        this.add_non_control(t);
        this.add_control(g, connect, select, addremove, null, null);
        let host = this.miNetServersHostButton.id();
        this.add_control(g, addremove, connect, host, null, null);
        this.add_control(g, host, addremove, back, null, null);
        this.add_control(g, back, host, nick, null, null);

        let (l, r, h) = (this.miNetServersLeftHeaderBar.id(), this.miNetServersRightHeaderBar.id(), this.miNetServersHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        let (di, dt) = (this.miNetServersConnectingDialogImage.id(), this.miNetServersConnectingDialogText.id());
        this.add_non_control(di);
        this.add_non_control(dt);

        let scroll = this.miNetServersScroll.id();
        this.add_control(g, scroll, null, null, null, null);

        this.set_initial_focus(g, select);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }

    /// Shows the selected server's hostname.
    pub fn refresh(&mut self, g: &mut Game) {
        if !g.netplay.savedServers.is_empty() {
            let nethostname = g.netplay.savedServers[g.netplay.selectedServerIndex as usize].hostname.clone();
            g.ctl(self.miNetServersSelectedHostText).set_text(nethostname);
        }
    }

    /// Refills the server list, tagging servers seen on the LAN, and shows the selected one.
    pub fn refresh_scroll(&mut self, g: &mut Game) {
        g.ctl(self.miNetServersScroll).clear();
        for iServer in 0..g.netplay.savedServers.len() {
            let host = g.netplay.savedServers[iServer].hostname.clone();
            let tag = if g.netplay.nearby.contains(&host) { "LAN" } else { "" };
            g.ctl(self.miNetServersScroll).add(&host, tag);
        }

        if !g.netplay.savedServers.is_empty() {
            let host = g.netplay.savedServers[g.netplay.selectedServerIndex as usize].hostname.clone();
            g.ctl(self.miNetServersSelectedHostText).set_text(host);
        } else {
            g.ctl(self.miNetServersSelectedHostText).set_text("(none)");
        }
    }

    /// Connects to the selected server, saves the server list and shows the Connecting dialog.
    pub fn connect_in_progress(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.netplay.savedServers.is_empty() {
            return;
        }

        gp.netplay.client.send_connect_request_to_selected_server(g);
        g.netplay.operationInProgress = true;
        net_save_server_list(g);

        g.ctl(self.miNetServersConnectingDialogImage).set_visible(true);
        g.ctl(self.miNetServersConnectingDialogText).set_visible(true);
        self.remember_current();

        let t = self.miNetServersConnectingDialogText.id();
        self.set_initial_focus(g, t);
        self.set_cancel_code(MENU_CODE_NET_CONNECT_ABORT);
        self.reset_menu(g);
    }

    /// Opens the server picker popup with cancel disabled.
    pub fn open_server_list(&mut self, g: &mut Game) {
        g.netplay.connectSuccessful = false;
        g.ctl(self.miNetServersScroll).set_visible(true);
        self.remember_current();

        let s = self.miNetServersScroll.id();
        self.set_initial_focus(g, s);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu(g);
    }

    /// Hides the picker and Connecting dialog and restores the remembered focus.
    pub fn restore(&mut self, g: &mut Game) {
        g.ctl(self.miNetServersScroll).set_visible(false);
        g.ctl(self.miNetServersConnectingDialogImage).set_visible(false);
        g.ctl(self.miNetServersConnectingDialogText).set_visible(false);

        let s = self.miNetServersSelectButton.id();
        self.set_initial_focus(g, s);
        self.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        self.restore_current(g);
    }
}
