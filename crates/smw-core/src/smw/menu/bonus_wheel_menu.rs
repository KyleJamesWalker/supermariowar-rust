//! The bonus wheel screen where a match, tournament, tour or world winner spins for a powerup.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_bonus_wheel::MI_BonusWheel;

#[derive(Default)]
/// Holds only the bonus wheel; cancel sends `MENU_CODE_BONUS_DONE`.
pub struct UI_BonusWheelMenu {
    pub ui_menu: UI_Menu,

    pub miBonusWheel: Ctl<MI_BonusWheel>,
}
crate::impl_base!(UI_BonusWheelMenu => ui_menu: UI_Menu);

impl UI_BonusWheelMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();

        this.miBonusWheel = Ctl::new(MI_BonusWheel::new(g, 144, 38), g);

        let w = this.miBonusWheel.id();
        this.add_control(g, w, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);
        this.set_initial_focus(g, w);
        this.set_cancel_code(MENU_CODE_BONUS_DONE);
        this
    }
}
