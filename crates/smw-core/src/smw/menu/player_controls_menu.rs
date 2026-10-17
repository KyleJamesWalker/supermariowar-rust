//! Port of src/smw/menu/PlayerControlsMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
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

    pub miInputContainer: Ctl<MI_InputControlContainer>,
}
crate::impl_base!(UI_PlayerControlsMenu => ui_menu: UI_Menu);

impl UI_PlayerControlsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();

        this.miInputContainer = Ctl::new(MI_InputControlContainer::new(g, gp.rm.menu_plain_field, 94, 10, 0), g);
        g.ctl(this.miInputContainer).set_auto_modify(true);

        let c = this.miInputContainer.id();
        this.add_control(g, c, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);
        this.set_initial_focus(g, c);
        this.set_cancel_code(MENU_CODE_BACK_TO_CONTROLS_MENU);
        this
    }

    pub fn set_player(&mut self, g: &mut Game, playerID: i16) {
        g.ctl(self.miInputContainer).set_player(g, playerID);
    }
}
