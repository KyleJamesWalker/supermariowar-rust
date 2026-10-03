//! Port of src/smw/menu/options/PowerupSettingsMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
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

    pub miStoredPowerupDelayField: Ptr<MI_SelectField<i16>>,
    pub miItemRespawnField: Ptr<MI_SelectField<i16>>,
    pub miSwapStyleField: Ptr<MI_SelectField<i16>>,
    pub miBonusWheelField: Ptr<MI_SelectField<i16>>,
    pub miKeepPowerupField: Ptr<MI_SelectField<bool>>,
    pub miHiddenBlockRespawnField: Ptr<MI_SelectField<i16>>,
    pub miStoredPowerupResetButton: Ptr<MI_StoredPowerupResetButton>,
    pub miPowerupSettingsMenuBackButton: Ptr<MI_Button>,

    pub miPowerupSettingsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miPowerupSettingsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miPowerupSettingsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_PowerupSettingsMenu => ui_menu: UI_Menu);

impl UI_PowerupSettingsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miStoredPowerupDelayField = select_field(spr_selectfield, 70, 100, "Item Use Speed", 500, 220,
                &[("Very Slow", 2), ("Slow", 3), ("Moderate", 4), ("Fast", 5), ("Very Fast", 6)],
                &mut game_values.storedpowerupdelay);

            this.miItemRespawnField = select_field(spr_selectfield, 70, 140, "Item Spawn", 500, 220, RESPAWN_TIMES, &mut game_values.itemrespawntime);

            this.miHiddenBlockRespawnField = select_field(spr_selectfield, 70, 180, "Hidden Block Hide", 500, 220, RESPAWN_TIMES, &mut game_values.hiddenblockrespawn);

            this.miSwapStyleField = select_field(spr_selectfield, 70, 220, "Swap Style", 500, 220,
                &[("Walk", 0), ("Blink", 1), ("Instant", 2)],
                &mut game_values.swapstyle);

            this.miBonusWheelField = select_field(spr_selectfield, 70, 260, "Bonus Wheel", 500, 220,
                &[("Off", 0), ("Tournament Win", 1), ("Every Game", 2)],
                &mut game_values.bonuswheel);

            this.miKeepPowerupField = select_field(spr_selectfield, 70, 300, "Bonus Item", 500, 220,
                &[("Until Next Spin", false), ("Keep Always", true)],
                &mut game_values.keeppowerup);
            this.miKeepPowerupField.set_auto_advance(true);

            this.miStoredPowerupResetButton = Ptr::new_box(MI_StoredPowerupResetButton::new(spr_selectfield, 70, 340, "Reset Stored Items", 500, TextAlign::LEFT));
            this.miStoredPowerupResetButton.set_code(MENU_CODE_RESET_STORED_POWERUPS);

            this.miPowerupSettingsMenuBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miPowerupSettingsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miPowerupSettingsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miPowerupSettingsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miPowerupSettingsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Item Settings Menu", 320, 5));
        }

        let delay = ctl_ptr(this.miStoredPowerupDelayField);
        let itemrespawn = ctl_ptr(this.miItemRespawnField);
        let hiddenblock = ctl_ptr(this.miHiddenBlockRespawnField);
        let swapstyle = ctl_ptr(this.miSwapStyleField);
        let bonuswheel = ctl_ptr(this.miBonusWheelField);
        let keep = ctl_ptr(this.miKeepPowerupField);
        let reset = ctl_ptr(this.miStoredPowerupResetButton);
        let back = ctl_ptr(this.miPowerupSettingsMenuBackButton);
        let null = Ptr::null();

        this.add_control(delay, back, itemrespawn, null, back);
        this.add_control(itemrespawn, delay, hiddenblock, null, back);
        this.add_control(hiddenblock, itemrespawn, swapstyle, null, back);
        this.add_control(swapstyle, hiddenblock, bonuswheel, null, back);
        this.add_control(bonuswheel, swapstyle, keep, null, back);
        this.add_control(keep, bonuswheel, reset, null, back);
        this.add_control(reset, keep, back, null, back);

        this.add_control(back, reset, delay, reset, null);

        let c = ctl_ptr(this.miPowerupSettingsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miPowerupSettingsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miPowerupSettingsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(delay);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
