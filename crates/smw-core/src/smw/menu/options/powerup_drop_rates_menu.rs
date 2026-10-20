//! Item selection: how often each powerup comes out of a [?] block, with presets.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
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

    pub miPowerupSelection: Ctl<MI_PowerupSelection>,

    pub miPowerupSelectionLeftHeaderBar: Ctl<MI_Image>,
    pub miPowerupSelectionMenuRightHeaderBar: Ctl<MI_Image>,
    pub miPowerupSelectionMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_PowerupDropRatesMenu => ui_menu: UI_Menu);

impl UI_PowerupDropRatesMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let menu_plain_field = g.rm.menu_plain_field;

        this.miPowerupSelection = Ctl::new(MI_PowerupSelection::new(g, 50, 44, 640, 8), g);
        g.ctl(this.miPowerupSelection).set_auto_modify(true);

        this.miPowerupSelectionLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miPowerupSelectionMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miPowerupSelectionMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Item Selection Menu", 320, 5), g);

        let sel = this.miPowerupSelection.id();
        this.add_control(g, sel, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);

        let c = this.miPowerupSelectionLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miPowerupSelectionMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miPowerupSelectionMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, sel);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
