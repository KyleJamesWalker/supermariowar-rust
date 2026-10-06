//! Port of src/smw/menu/BonusWheelMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::uicontrol::ctl_ptr;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_bonus_wheel::MI_BonusWheel;

#[derive(Default)]
pub struct UI_BonusWheelMenu {
    pub ui_menu: UI_Menu,

    pub miBonusWheel: Ptr<MI_BonusWheel>,
}
crate::impl_base!(UI_BonusWheelMenu => ui_menu: UI_Menu);

impl UI_BonusWheelMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();

        this.miBonusWheel = Ptr::new_box(MI_BonusWheel::new(144, 38));

        let w = ctl_ptr(this.miBonusWheel);
        this.add_control(w, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());
        this.set_initial_focus(w);
        this.set_cancel_code(MENU_CODE_BONUS_DONE);
        this
    }
}
