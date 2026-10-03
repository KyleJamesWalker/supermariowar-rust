//! Port of src/smw/menu/network/NetLobbyMenu.cpp

use crate::common::global::rm;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::NET_MAX_ROOM_NAME_LENGTH;
use crate::globals::Ptr;
use crate::smw::net::netplay;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;

#[derive(Default)]
pub struct UI_NetLobbyMenu {
    pub ui_menu: UI_Menu,

    miNetLobbyScroll: Ptr<MI_NetworkListScroll>,
    miNetLobbyNewRoomButton: Ptr<MI_Button>,
    miNetLobbyFilterField: Ptr<MI_TextField>,
    miNetLobbyRefreshButton: Ptr<MI_Button>,
    miNetLobbyBackButton: Ptr<MI_Button>,

    miNetLobbyJoiningDialogImage: Ptr<MI_Image>,
    miNetLobbyJoiningDialogText: Ptr<MI_Text>,

    miNetLobbyLeftHeaderBar: Ptr<MI_Image>,
    miNetLobbyRightHeaderBar: Ptr<MI_Image>,
    miNetLobbyHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_NetLobbyMenu => ui_menu: UI_Menu);

impl UI_NetLobbyMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let plain = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miNetLobbyNewRoomButton = Ptr::new_box(MI_Button::new(selectfield, 320 + 20, 80, "New room", 320 - 30, TextAlign::CENTER));
            this.miNetLobbyNewRoomButton.set_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);

            this.miNetLobbyFilterField = Ptr::new_box(MI_TextField::new(plain, 320 + 20, 160, "Search", 320 - 30, 90));
            this.miNetLobbyFilterField.set_data(&mut netplay.roomFilter, NET_MAX_ROOM_NAME_LENGTH as i16);

            this.miNetLobbyRefreshButton = Ptr::new_box(MI_Button::new(selectfield, 320 + 20, 200, "Refresh", 320 - 30, TextAlign::CENTER));
            this.miNetLobbyRefreshButton.set_code(MENU_CODE_TO_NET_LOBBY_MENU);

            this.miNetLobbyBackButton = Ptr::new_box(MI_Button::new(selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miNetLobbyBackButton.set_code(MENU_CODE_TO_NET_SERVERS_MENU);

            this.miNetLobbyScroll = Ptr::new_box(MI_NetworkListScroll::new(
                plain,
                15,
                40,
                320,
                11,
                "Rooms",
                MENU_CODE_NET_JOIN_ROOM_IN_PROGRESS,
                MENU_CODE_TO_NET_SERVERS_MENU,
            ));
            this.miNetLobbyScroll.remote_index(&mut netplay.selectedRoomIndex);
            this.miNetLobbyScroll.set_auto_modify(true);

            netplay.client.set_room_list_ui_control(this.miNetLobbyScroll);

            this.miNetLobbyJoiningDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miNetLobbyJoiningDialogText = Ptr::new_box(MI_HeaderText::new("Joining...", 320, 240 - 12));

            this.miNetLobbyJoiningDialogImage.set_visible(false);
            this.miNetLobbyJoiningDialogText.set_visible(false);

            this.miNetLobbyLeftHeaderBar = Ptr::new_box(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miNetLobbyRightHeaderBar = Ptr::new_box(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miNetLobbyHeaderText = Ptr::new_box(MI_HeaderText::new("Multiplayer Lobby Menu", 320, 5));
        }

        let scroll = ctl_ptr(this.miNetLobbyScroll);
        let newroom = ctl_ptr(this.miNetLobbyNewRoomButton);
        let filter = ctl_ptr(this.miNetLobbyFilterField);
        let refresh = ctl_ptr(this.miNetLobbyRefreshButton);
        let back = ctl_ptr(this.miNetLobbyBackButton);
        let null = Ptr::null();

        this.add_control(scroll, null, null, null, newroom);
        this.add_control(newroom, back, filter, scroll, null);
        this.add_control(filter, newroom, refresh, scroll, null);
        this.add_control(refresh, filter, back, scroll, null);
        this.add_control(back, refresh, newroom, scroll, null);

        let (di, dt) = (ctl_ptr(this.miNetLobbyJoiningDialogImage), ctl_ptr(this.miNetLobbyJoiningDialogText));
        this.add_non_control(di);
        this.add_non_control(dt);

        let (l, r, h) = (ctl_ptr(this.miNetLobbyLeftHeaderBar), ctl_ptr(this.miNetLobbyRightHeaderBar), ctl_ptr(this.miNetLobbyHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(newroom);
        this.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
        this
    }

    pub fn join_in_progress(&mut self) {
        unsafe {
            netplay.client.send_join_room_message();
            netplay.operationInProgress = true;
        }

        self.miNetLobbyJoiningDialogImage.set_visible(true);
        self.miNetLobbyJoiningDialogText.set_visible(true);

        let t = ctl_ptr(self.miNetLobbyJoiningDialogText);
        self.set_initial_focus(t);
        self.set_cancel_code(MENU_CODE_NET_JOIN_ROOM_ABORT);
        self.reset_menu();
    }

    pub fn abort_join(&mut self) {
        unsafe { netplay.operationInProgress = false };
        self.restore();
    }

    pub fn restore(&mut self) {
        self.miNetLobbyJoiningDialogImage.set_visible(false);
        self.miNetLobbyJoiningDialogText.set_visible(false);

        let b = ctl_ptr(self.miNetLobbyNewRoomButton);
        self.set_initial_focus(b);
        self.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
    }
}
