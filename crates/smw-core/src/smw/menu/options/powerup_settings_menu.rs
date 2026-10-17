//! Port of src/smw/menu/options/PowerupSettingsMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;
use crate::smw::ui::mi_stored_powerup_reset_button::MI_StoredPowerupResetButton;

const RESPAWN_TIMES: &[(&str, i16)] = &[
    ("Off", 0),
    ("5 Seconds", 310),
    ("10 Seconds", 620),
    ("15 Seconds", 930),
    ("20 Seconds", 1240),
    ("25 Seconds", 1550),
    ("30 Seconds", 1860),
    ("35 Seconds", 2170),
    ("40 Seconds", 2480),
    ("45 Seconds", 2790),
    ("50 Seconds", 3100),
    ("55 Seconds", 3410),
    ("60 Seconds", 3720),
];

/*
    In this menu, you can change some powerup settings,
    like fire rate, reloading of [?] blocks and bonus items.
*/
#[derive(Default)]
pub struct UI_PowerupSettingsMenu {
    pub ui_menu: UI_Menu,

    pub miStoredPowerupDelayField: Ctl<MI_SelectField<i16>>,
    pub miItemRespawnField: Ctl<MI_SelectField<i16>>,
    pub miSwapStyleField: Ctl<MI_SelectField<i16>>,
    pub miBonusWheelField: Ctl<MI_SelectField<i16>>,
    pub miKeepPowerupField: Ctl<MI_SelectField<bool>>,
    pub miHiddenBlockRespawnField: Ctl<MI_SelectField<i16>>,
    pub miStoredPowerupResetButton: Ctl<MI_StoredPowerupResetButton>,
    pub miPowerupSettingsMenuBackButton: Ctl<MI_Button>,

    pub miPowerupSettingsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miPowerupSettingsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miPowerupSettingsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_PowerupSettingsMenu => ui_menu: UI_Menu);

impl UI_PowerupSettingsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miStoredPowerupDelayField = select_field(spr_selectfield, 70, 100, "Item Use Speed", 500, 220,
                &[("Very Slow", 2), ("Slow", 3), ("Moderate", 4), ("Fast", 5), ("Very Fast", 6)],
                &mut g.game_values.storedpowerupdelay, g);

            this.miItemRespawnField = select_field(spr_selectfield, 70, 140, "Item Spawn", 500, 220, RESPAWN_TIMES, &mut g.game_values.itemrespawntime, g);

            this.miHiddenBlockRespawnField = select_field(spr_selectfield, 70, 180, "Hidden Block Hide", 500, 220, RESPAWN_TIMES, &mut g.game_values.hiddenblockrespawn, g);

            this.miSwapStyleField = select_field(spr_selectfield, 70, 220, "Swap Style", 500, 220,
                &[("Walk", 0), ("Blink", 1), ("Instant", 2)],
                &mut g.game_values.swapstyle, g);

            this.miBonusWheelField = select_field(spr_selectfield, 70, 260, "Bonus Wheel", 500, 220,
                &[("Off", 0), ("Tournament Win", 1), ("Every Game", 2)],
                &mut g.game_values.bonuswheel, g);

            this.miKeepPowerupField = select_field(spr_selectfield, 70, 300, "Bonus Item", 500, 220,
                &[("Until Next Spin", false), ("Keep Always", true)],
                &mut g.game_values.keeppowerup, g);
            g.ctl(this.miKeepPowerupField).set_auto_advance(true);

            this.miStoredPowerupResetButton = Ctl::new(MI_StoredPowerupResetButton::new(g, spr_selectfield, 70, 340, "Reset Stored Items", 500, TextAlign::LEFT), g);
            g.ctl(this.miStoredPowerupResetButton).set_code(MENU_CODE_RESET_STORED_POWERUPS);

            this.miPowerupSettingsMenuBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miPowerupSettingsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miPowerupSettingsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miPowerupSettingsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miPowerupSettingsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Item Settings Menu", 320, 5), g);
        }

        let delay = this.miStoredPowerupDelayField.id();
        let itemrespawn = this.miItemRespawnField.id();
        let hiddenblock = this.miHiddenBlockRespawnField.id();
        let swapstyle = this.miSwapStyleField.id();
        let bonuswheel = this.miBonusWheelField.id();
        let keep = this.miKeepPowerupField.id();
        let reset = this.miStoredPowerupResetButton.id();
        let back = this.miPowerupSettingsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, delay, back, itemrespawn, null, back);
        this.add_control(g, itemrespawn, delay, hiddenblock, null, back);
        this.add_control(g, hiddenblock, itemrespawn, swapstyle, null, back);
        this.add_control(g, swapstyle, hiddenblock, bonuswheel, null, back);
        this.add_control(g, bonuswheel, swapstyle, keep, null, back);
        this.add_control(g, keep, bonuswheel, reset, null, back);
        this.add_control(g, reset, keep, back, null, back);

        this.add_control(g, back, reset, delay, reset, null);

        let c = this.miPowerupSettingsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miPowerupSettingsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miPowerupSettingsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, delay);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
