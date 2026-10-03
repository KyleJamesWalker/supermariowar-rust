//! Port of src/smw/menu/options/PowerupDropRatesMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::ctl_ptr;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_powerup_selection::MI_PowerupSelection;

/*
    In this menu you can set how frequently can various
    powerups pop out of a [?] block.
*/
#[derive(Default)]
pub struct UI_PowerupDropRatesMenu {
    pub ui_menu: UI_Menu,

    pub miPowerupSelection: Ptr<MI_PowerupSelection>,

    // MI_PowerupSlider * miPowerupSlider[NUM_POWERUPS];
    // MI_Button * miPowerupSelectionBackButton;
    // MI_Button * miPowerupSelectionRestoreDefaultsButton;
    pub miPowerupSelectionLeftHeaderBar: Ptr<MI_Image>,
    pub miPowerupSelectionMenuRightHeaderBar: Ptr<MI_Image>,
    pub miPowerupSelectionMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_PowerupDropRatesMenu => ui_menu: UI_Menu);

impl UI_PowerupDropRatesMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miPowerupSelection = Ptr::new_box(MI_PowerupSelection::new(50, 44, 640, 8));
            this.miPowerupSelection.set_auto_modify(true);

            this.miPowerupSelectionLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miPowerupSelectionMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miPowerupSelectionMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Item Selection Menu", 320, 5));
        }

        let sel = ctl_ptr(this.miPowerupSelection);
        this.add_control(sel, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());

        let c = ctl_ptr(this.miPowerupSelectionLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miPowerupSelectionMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miPowerupSelectionMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(sel);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
