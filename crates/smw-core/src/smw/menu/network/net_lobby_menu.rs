//! The server lobby: room list, search, refresh, new room and the join and password dialogs.
use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::{NET_MAX_ROOM_NAME_LENGTH, NET_MAX_ROOM_PASSWORD_LENGTH};
use crate::globals::Ptr;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;

#[derive(Default)]
pub struct UI_NetLobbyMenu {
    pub ui_menu: UI_Menu,

    /// Room list; the netplay client fills it and its selection writes `netplay.selectedRoomIndex`.
    miNetLobbyScroll: Ctl<MI_NetworkListScroll>,
    miNetLobbyNewRoomButton: Ctl<MI_Button>,
    miNetLobbyFilterField: Ctl<MI_TextField>,
    miNetLobbyRefreshButton: Ctl<MI_Button>,
    miNetLobbyBackButton: Ctl<MI_Button>,

    miNetLobbyJoiningDialogImage: Ctl<MI_Image>,
    miNetLobbyJoiningDialogText: Ctl<MI_Text>,

    miNetLobbyPasswordTitle: Ctl<MI_Text>,
    miNetLobbyPasswordField: Ctl<MI_TextField>,
    miNetLobbyPasswordOK: Ctl<MI_Button>,
    miNetLobbyPasswordCancel: Ctl<MI_Button>,
    /// Backing text of the password field.
    passwordData: String,
    /// Room the password dialog is for, kept by ID since the room list can refresh meanwhile.
    passwordRoomID: u32,

    miNetLobbyLeftHeaderBar: Ctl<MI_Image>,
    miNetLobbyRightHeaderBar: Ctl<MI_Image>,
    miNetLobbyHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_NetLobbyMenu => ui_menu: UI_Menu);

impl UI_NetLobbyMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        let selectfield = g.rm.spr_selectfield;
        let plain = g.rm.menu_plain_field;

        this.miNetLobbyNewRoomButton = Ctl::new(MI_Button::new(g, selectfield, 320 + 20, 80, "New room", 320 - 30, TextAlign::CENTER), g);
        g.ctl(this.miNetLobbyNewRoomButton).set_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);

        this.miNetLobbyFilterField = Ctl::new(MI_TextField::new(plain, 320 + 20, 160, "Search", 320 - 30, 90), g);
        g.ctl(this.miNetLobbyFilterField).set_data(g, &mut gp.netplay.roomFilter, NET_MAX_ROOM_NAME_LENGTH as i16);

        this.miNetLobbyRefreshButton = Ctl::new(MI_Button::new(g, selectfield, 320 + 20, 200, "Refresh", 320 - 30, TextAlign::CENTER), g);
        g.ctl(this.miNetLobbyRefreshButton).set_code(MENU_CODE_TO_NET_LOBBY_MENU);

        this.miNetLobbyBackButton = Ctl::new(MI_Button::new(g, selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miNetLobbyBackButton).set_code(MENU_CODE_TO_NET_SERVERS_MENU);

        this.miNetLobbyScroll = Ctl::new(MI_NetworkListScroll::new(
            plain,
            15,
            40,
            320,
            11,
            "Rooms",
            MENU_CODE_NET_JOIN_ROOM_IN_PROGRESS,
            MENU_CODE_TO_NET_SERVERS_MENU,
        ), g);
        g.ctl(this.miNetLobbyScroll).remote_index(&mut g.netplay.selectedRoomIndex);
        g.ctl(this.miNetLobbyScroll).set_auto_modify(true);

        g.netplay.client.set_room_list_ui_control(this.miNetLobbyScroll);

        this.miNetLobbyJoiningDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miNetLobbyJoiningDialogText = Ctl::new(MI_HeaderText::new(g, "Joining...", 320, 240 - 12), g);

        g.ctl(this.miNetLobbyJoiningDialogImage).set_visible(false);
        g.ctl(this.miNetLobbyJoiningDialogText).set_visible(false);

        this.miNetLobbyPasswordTitle = Ctl::new(MI_Text::new(g, "Room password:", 320 + 20, 250, 0, true, TextAlign::LEFT), g);
        this.miNetLobbyPasswordField = Ctl::new(MI_TextField::new(plain, 320 + 20, 280, "Password", 320 - 30, 120), g);
        let data = &mut this.passwordData as *mut String;
        g.ctl(this.miNetLobbyPasswordField).set_data(g, unsafe { &mut *data }, NET_MAX_ROOM_PASSWORD_LENGTH as i16);
        this.miNetLobbyPasswordOK = Ctl::new(MI_Button::new(g, selectfield, 320 + 20, 320, "Join", 140, TextAlign::CENTER), g);
        g.ctl(this.miNetLobbyPasswordOK).set_code(MENU_CODE_NET_JOIN_ROOM_PASSWORD_OK);
        this.miNetLobbyPasswordCancel = Ctl::new(MI_Button::new(g, selectfield, 320 + 170, 320, "Cancel", 140, TextAlign::CENTER), g);
        g.ctl(this.miNetLobbyPasswordCancel).set_code(MENU_CODE_NET_JOIN_ROOM_ABORT);
        this.show_password_dialog(g, false);

        this.miNetLobbyLeftHeaderBar = Ctl::new(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miNetLobbyRightHeaderBar = Ctl::new(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miNetLobbyHeaderText = Ctl::new(MI_HeaderText::new(g, "Multiplayer Lobby Menu", 320, 5), g);

        let scroll = this.miNetLobbyScroll.id();
        let newroom = this.miNetLobbyNewRoomButton.id();
        let filter = this.miNetLobbyFilterField.id();
        let refresh = this.miNetLobbyRefreshButton.id();
        let back = this.miNetLobbyBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, scroll, null, null, null, newroom);
        this.add_control(g, newroom, back, filter, scroll, null);
        this.add_control(g, filter, newroom, refresh, scroll, null);
        this.add_control(g, refresh, filter, back, scroll, null);
        this.add_control(g, back, refresh, newroom, scroll, null);

        let (di, dt) = (this.miNetLobbyJoiningDialogImage.id(), this.miNetLobbyJoiningDialogText.id());
        this.add_non_control(di);
        this.add_non_control(dt);

        let title = this.miNetLobbyPasswordTitle.id();
        this.add_non_control(title);
        let (field, ok, cancel) = (this.miNetLobbyPasswordField.id(), this.miNetLobbyPasswordOK.id(), this.miNetLobbyPasswordCancel.id());
        this.add_control(g, field, ok, ok, null, null);
        this.add_control(g, ok, field, field, cancel, cancel);
        this.add_control(g, cancel, field, field, ok, ok);

        let (l, r, h) = (this.miNetLobbyLeftHeaderBar.id(), this.miNetLobbyRightHeaderBar.id(), this.miNetLobbyHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, newroom);
        this.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
        this
    }

    fn show_password_dialog(&mut self, g: &mut Game, visible: bool) {
        g.ctl(self.miNetLobbyPasswordTitle).set_visible(visible);
        g.ctl(self.miNetLobbyPasswordField).set_visible(visible);
        g.ctl(self.miNetLobbyPasswordOK).set_visible(visible);
        g.ctl(self.miNetLobbyPasswordCancel).set_visible(visible);
    }

    /// Joins the selected room, asking for its password first if it has one.
    pub fn join_in_progress(&mut self, g: &mut Game) {
        let Some(room) = g.netplay.currentRooms.get(g.netplay.selectedRoomIndex as usize) else { return };
        if room.passwordRequired {
            self.passwordRoomID = room.roomID;
            self.passwordData.clear();
            g.ctl(self.miNetLobbyPasswordField).refresh(g);
            self.show_password_dialog(g, true);
            let f = self.miNetLobbyPasswordField.id();
            self.set_initial_focus(g, f);
            self.set_cancel_code(MENU_CODE_NET_JOIN_ROOM_ABORT);
            self.reset_menu(g);
            return;
        }
        self.send_join(g, "");
    }

    /// Sends the join for the password dialog's room, or closes the dialogs if that room is gone.
    pub fn join_with_password(&mut self, g: &mut Game) {
        self.show_password_dialog(g, false);
        let password = std::mem::take(&mut self.passwordData);
        g.ctl(self.miNetLobbyPasswordField).refresh(g);
        match g.netplay.currentRooms.iter().position(|r| r.roomID == self.passwordRoomID) {
            Some(index) => {
                g.netplay.selectedRoomIndex = index as u16;
                self.send_join(g, &password);
            }
            None => self.restore(g),
        }
    }

    /// Sends the join request and shows the Joining dialog until the server answers.
    fn send_join(&mut self, g: &mut Game, password: &str) {
        let mut gp = g.ptr();
        if g.netplay.currentRooms.get(g.netplay.selectedRoomIndex as usize).is_none() {
            self.restore(g);
            return;
        }
        gp.netplay.client.send_join_room_message(g, password);
        g.netplay.operationInProgress = true;

        g.ctl(self.miNetLobbyJoiningDialogImage).set_visible(true);
        g.ctl(self.miNetLobbyJoiningDialogText).set_visible(true);

        let t = self.miNetLobbyJoiningDialogText.id();
        self.set_initial_focus(g, t);
        self.set_cancel_code(MENU_CODE_NET_JOIN_ROOM_ABORT);
        self.reset_menu(g);
    }

    /// Stops waiting for a join answer and closes the dialogs.
    pub fn abort_join(&mut self, g: &mut Game) {
        g.netplay.operationInProgress = false;
        self.restore(g);
    }

    /// Hides the Joining and password dialogs and focuses New room.
    pub fn restore(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.ctl(self.miNetLobbyJoiningDialogImage).set_visible(false);
        g.ctl(self.miNetLobbyJoiningDialogText).set_visible(false);
        self.show_password_dialog(g, false);

        let b = self.miNetLobbyNewRoomButton.id();
        self.set_initial_focus(gp.get(), b);
        self.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
    }
}
