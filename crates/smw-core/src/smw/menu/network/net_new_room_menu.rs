//! Room name and password entry, then room creation.
use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::common_netplay::protocol_definitions::{NET_MAX_ROOM_NAME_LENGTH, NET_MAX_ROOM_PASSWORD_LENGTH};
use crate::globals::Ptr;
use crate::smw::net::net_save_server_list;

#[derive(Default)]
/// Room name and optional password fields, and Create.
pub struct UI_NetNewRoomMenu {
    pub ui_menu: UI_Menu,

    miNetNewRoomNameField: Ctl<MI_TextField>,
    miNetNewRoomPasswordField: Ctl<MI_TextField>,
    miNetNewRoomCreateButton: Ctl<MI_Button>,
    miNetNewRoomBackButton: Ctl<MI_Button>,

    miNetNewRoomCreatingDialogImage: Ctl<MI_Image>,
    miNetNewRoomCreatingDialogText: Ctl<MI_Text>,

    miNetNewRoomLeftHeaderBar: Ctl<MI_Image>,
    miNetNewRoomRightHeaderBar: Ctl<MI_Image>,
    miNetNewRoomHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_NetNewRoomMenu => ui_menu: UI_Menu);

impl UI_NetNewRoomMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        let selectfield = g.rm.spr_selectfield;
        let plain = g.rm.menu_plain_field;

        this.miNetNewRoomNameField = Ctl::new(MI_TextField::new(plain, 70, 160, "Room name", 640 - 2 * 70, 230), g);
        g.ctl(this.miNetNewRoomNameField).set_data(g, &mut gp.netplay.newroom_name, NET_MAX_ROOM_NAME_LENGTH as i16);

        this.miNetNewRoomPasswordField = Ctl::new(MI_TextField::new(plain, 70, 200, "Password (optional)", 640 - 2 * 70, 230), g);
        g.ctl(this.miNetNewRoomPasswordField).set_data(g, &mut gp.netplay.newroom_password, NET_MAX_ROOM_PASSWORD_LENGTH as i16);

        this.miNetNewRoomCreateButton = Ctl::new(MI_Button::new(g, selectfield, 70, 240, "Create!", 640 - 2 * 70, TextAlign::CENTER), g);
        g.ctl(this.miNetNewRoomCreateButton).set_code(MENU_CODE_TO_NET_NEW_ROOM_CREATE_IN_PROGRESS);

        this.miNetNewRoomBackButton = Ctl::new(MI_Button::new(g, selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miNetNewRoomBackButton).set_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);

        let name = this.miNetNewRoomNameField.id();
        let pass = this.miNetNewRoomPasswordField.id();
        let create = this.miNetNewRoomCreateButton.id();
        let back = this.miNetNewRoomBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, name, back, pass, null, null);
        this.add_control(g, pass, name, create, null, null);
        this.add_control(g, create, pass, back, null, null);
        this.add_control(g, back, create, name, null, null);

        this.miNetNewRoomCreatingDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miNetNewRoomCreatingDialogText = Ctl::new(MI_HeaderText::new(g, "Creating...", 320, 240 - 12), g);

        g.ctl(this.miNetNewRoomCreatingDialogImage).set_visible(false);
        g.ctl(this.miNetNewRoomCreatingDialogText).set_visible(false);

        let (di, dt) = (this.miNetNewRoomCreatingDialogImage.id(), this.miNetNewRoomCreatingDialogText.id());
        this.add_non_control(di);
        this.add_non_control(dt);

        let plain = g.rm.menu_plain_field;
        this.miNetNewRoomLeftHeaderBar = Ctl::new(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miNetNewRoomRightHeaderBar = Ctl::new(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miNetNewRoomHeaderText = Ctl::new(MI_HeaderText::new(g, "New Room Settings Menu", 320, 5), g);

        let (l, r, h) = (this.miNetNewRoomLeftHeaderBar.id(), this.miNetNewRoomRightHeaderBar.id(), this.miNetNewRoomHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, name);
        this.set_cancel_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);
        this
    }

    /// Sends the create room request and shows the Creating dialog; names of 2 characters or fewer are refused.
    pub fn create_in_progress(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.netplay.newroom_name.len() > 2 {
            gp.netplay.client.send_create_room_message(g);
            g.netplay.operationInProgress = true;
            net_save_server_list(g);

            g.ctl(self.miNetNewRoomCreatingDialogImage).set_visible(true);
            g.ctl(self.miNetNewRoomCreatingDialogText).set_visible(true);
            self.remember_current();

            let t = self.miNetNewRoomCreatingDialogText.id();
            self.set_initial_focus(g, t);
            self.set_cancel_code(MENU_CODE_TO_NET_NEW_ROOM_CREATE_ABORT);
            self.reset_menu(g);
        } else {
            println!("[net] Room name is too short!");
        }
    }

    /// Stops waiting for the create answer and closes the dialog.
    pub fn abort_create(&mut self, g: &mut Game) {
        g.netplay.operationInProgress = false;
        self.restore(g);
    }

    /// Hides the Creating dialog and focuses the room name.
    pub fn restore(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.ctl(self.miNetNewRoomCreatingDialogImage).set_visible(false);
        g.ctl(self.miNetNewRoomCreatingDialogText).set_visible(false);

        let n = self.miNetNewRoomNameField.id();
        self.set_initial_focus(gp.get(), n);
        self.set_cancel_code(MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU);
    }
}
