//! Port of src/smw/menu/network/NetNewRoomMenu.cpp

use crate::common::global::rm;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::{NET_MAX_ROOM_NAME_LENGTH, NET_MAX_ROOM_PASSWORD_LENGTH};
use crate::globals::Ptr;
use crate::smw::net::netplay;

#[derive(Default)]
pub struct UI_NetNewRoomMenu {
    pub ui_menu: UI_Menu,

    miNetNewRoomNameField: Ptr<MI_TextField>,
    miNetNewRoomPasswordField: Ptr<MI_TextField>,
    miNetNewRoomCreateButton: Ptr<MI_Button>,
    miNetNewRoomBackButton: Ptr<MI_Button>,

    miNetNewRoomCreatingDialogImage: Ptr<MI_Image>,
    miNetNewRoomCreatingDialogText: Ptr<MI_Text>,

    miNetNewRoomLeftHeaderBar: Ptr<MI_Image>,
    miNetNewRoomRightHeaderBar: Ptr<MI_Image>,
    miNetNewRoomHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_NetNewRoomMenu => ui_menu: UI_Menu);

impl UI_NetNewRoomMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let plain = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miNetNewRoomNameField = Ptr::new_box(MI_TextField::new(plain, 70, 160, "Room name", 640 - 2 * 70, 230));
            this.miNetNewRoomNameField.set_data(&mut netplay.newroom_name, NET_MAX_ROOM_NAME_LENGTH as i16);

            this.miNetNewRoomPasswordField = Ptr::new_box(MI_TextField::new(plain, 70, 200, "Password (optional)", 640 - 2 * 70, 230));
            this.miNetNewRoomPasswordField.set_data(&mut netplay.newroom_password, NET_MAX_ROOM_PASSWORD_LENGTH as i16);

            this.miNetNewRoomCreateButton = Ptr::new_box(MI_Button::new(selectfield, 70, 240, "Create!", 640 - 2 * 70, TextAlign::CENTER));
            this.miNetNewRoomCreateButton.set_code(MENU_CODE_TO_NET_NEW_ROOM_CREATE_IN_PROGRESS);

            this.miNetNewRoomBackButton = Ptr::new_box(MI_Button::new(selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miNetNewRoomBackButton.set_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);
        }

        let name = ctl_ptr(this.miNetNewRoomNameField);
        let pass = ctl_ptr(this.miNetNewRoomPasswordField);
        let create = ctl_ptr(this.miNetNewRoomCreateButton);
        let back = ctl_ptr(this.miNetNewRoomBackButton);
        let null = Ptr::null();

        this.add_control(name, back, pass, null, null);
        this.add_control(pass, name, create, null, null);
        this.add_control(create, pass, back, null, null);
        this.add_control(back, create, name, null, null);

        unsafe {
            this.miNetNewRoomCreatingDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miNetNewRoomCreatingDialogText = Ptr::new_box(MI_HeaderText::new("Creating...", 320, 240 - 12));
        }

        this.miNetNewRoomCreatingDialogImage.set_visible(false);
        this.miNetNewRoomCreatingDialogText.set_visible(false);

        let (di, dt) = (ctl_ptr(this.miNetNewRoomCreatingDialogImage), ctl_ptr(this.miNetNewRoomCreatingDialogText));
        this.add_non_control(di);
        this.add_non_control(dt);

        unsafe {
            let plain = Ptr::from_mut(&mut rm.menu_plain_field);
            this.miNetNewRoomLeftHeaderBar = Ptr::new_box(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miNetNewRoomRightHeaderBar = Ptr::new_box(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miNetNewRoomHeaderText = Ptr::new_box(MI_HeaderText::new("New Room Settings Menu", 320, 5));
        }

        let (l, r, h) = (ctl_ptr(this.miNetNewRoomLeftHeaderBar), ctl_ptr(this.miNetNewRoomRightHeaderBar), ctl_ptr(this.miNetNewRoomHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(name);
        this.set_cancel_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);
        this
    }

    pub fn create_in_progress(&mut self) {
        unsafe {
            if netplay.newroom_name.len() > 2 {
                netplay.client.send_create_room_message();
                netplay.operationInProgress = true;

                self.miNetNewRoomCreatingDialogImage.set_visible(true);
                self.miNetNewRoomCreatingDialogText.set_visible(true);
                self.remember_current();

                let t = ctl_ptr(self.miNetNewRoomCreatingDialogText);
                self.set_initial_focus(t);
                self.set_cancel_code(MENU_CODE_TO_NET_NEW_ROOM_CREATE_ABORT);
                self.reset_menu();
            } else {
                println!("[net] Room name is too short!");
            }
        }
    }

    pub fn abort_create(&mut self) {
        unsafe { netplay.operationInProgress = false };
        self.restore();
    }

    pub fn restore(&mut self) {
        self.miNetNewRoomCreatingDialogImage.set_visible(false);
        self.miNetNewRoomCreatingDialogText.set_visible(false);

        let n = ctl_ptr(self.miNetNewRoomNameField);
        self.set_initial_focus(n);
        self.set_cancel_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);
    }
}
