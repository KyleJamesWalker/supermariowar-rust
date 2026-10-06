//! Port of src/smw/menu/PlayerControlsMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::uicontrol::ctl_ptr;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_input_control_container::MI_InputControlContainer;

/*
    This is where the selected player (from PlayerControlsSelectMenu)
    can set the preferred control device and keys.
*/
#[derive(Default)]
pub struct UI_PlayerControlsMenu {
    pub ui_menu: UI_Menu,

    pub miInputContainer: Ptr<MI_InputControlContainer>,
}
crate::impl_base!(UI_PlayerControlsMenu => ui_menu: UI_Menu);

impl UI_PlayerControlsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();

        unsafe {
            this.miInputContainer = Ptr::new_box(MI_InputControlContainer::new(Ptr::from_mut(&mut rm.menu_plain_field), 94, 10, 0));
        }
        this.miInputContainer.set_auto_modify(true);

        let c = ctl_ptr(this.miInputContainer);
        this.add_control(c, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());
        this.set_initial_focus(c);
        this.set_cancel_code(MENU_CODE_BACK_TO_CONTROLS_MENU);
        this
    }

    pub fn set_player(&mut self, playerID: i16) {
        self.miInputContainer.set_player(playerID);
    }
}
