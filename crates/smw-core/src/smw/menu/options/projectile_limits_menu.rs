//! Weapon use limits: how many times each projectile weapon or special state can be used.

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

/// Use limit choices; 0 is unlimited.
const LIMITS: &[(&str, i16)] = &[
    ("Unlimited", 0),
    ("2", 2),
    ("5", 5),
    ("8", 8),
    ("10", 10),
    ("12", 12),
    ("15", 15),
    ("20", 20),
    ("25", 25),
    ("30", 30),
    ("40", 40),
    ("50", 50),
];

/*
    You can set the maximum number of uses of certain
    projectile weapons and special player states.
*/
#[derive(Default)]
pub struct UI_ProjectileLimitsMenu {
    pub ui_menu: UI_Menu,

    pub miFireballLimitField: Ctl<MI_SelectField<i16>>,
    pub miHammerLimitField: Ctl<MI_SelectField<i16>>,
    pub miBoomerangLimitField: Ctl<MI_SelectField<i16>>,
    pub miFeatherLimitField: Ctl<MI_SelectField<i16>>,
    pub miLeafLimitField: Ctl<MI_SelectField<i16>>,
    pub miPwingsLimitField: Ctl<MI_SelectField<i16>>,
    pub miTanookiLimitField: Ctl<MI_SelectField<i16>>,
    pub miBombLimitField: Ctl<MI_SelectField<i16>>,
    pub miWandLimitField: Ctl<MI_SelectField<i16>>,

    pub miProjectilesLimitsMenuBackButton: Ctl<MI_Button>,

    pub miProjectilesLimitsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miProjectilesLimitsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miProjectilesLimitsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_ProjectileLimitsMenu => ui_menu: UI_Menu);

impl UI_ProjectileLimitsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miFireballLimitField = select_field(spr, 70, 60, "Fireball Limit", 500, 220, LIMITS, &mut g.game_values.fireballlimit, g);
            this.miHammerLimitField = select_field(spr, 70, 100, "Hammer Limit", 500, 220, LIMITS, &mut g.game_values.hammerlimit, g);
            this.miBoomerangLimitField = select_field(spr, 70, 140, "Boomerang Limit", 500, 220, LIMITS, &mut g.game_values.boomeranglimit, g);
            this.miFeatherLimitField = select_field(spr, 70, 180, "Feather Limit", 500, 220, LIMITS, &mut g.game_values.featherlimit, g);
            this.miLeafLimitField = select_field(spr, 70, 220, "Leaf Limit", 500, 220, LIMITS, &mut g.game_values.leaflimit, g);
            this.miPwingsLimitField = select_field(spr, 70, 260, "P-Wings Limit", 500, 220, LIMITS, &mut g.game_values.pwingslimit, g);
            this.miTanookiLimitField = select_field(spr, 70, 300, "Tanooki Limit", 500, 220, LIMITS, &mut g.game_values.tanookilimit, g);
            this.miBombLimitField = select_field(spr, 70, 340, "Bomb Limit", 500, 220, LIMITS, &mut g.game_values.bombslimit, g);
            this.miWandLimitField = select_field(spr, 70, 380, "Wand Limit", 500, 220, LIMITS, &mut g.game_values.wandlimit, g);

            this.miProjectilesLimitsMenuBackButton = Ctl::new(MI_Button::new(g, spr, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miProjectilesLimitsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miProjectilesLimitsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miProjectilesLimitsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miProjectilesLimitsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Weapon Use Limits Menu", 320, 5), g);
        }

        let fireball = this.miFireballLimitField.id();
        let hammer = this.miHammerLimitField.id();
        let boomerang = this.miBoomerangLimitField.id();
        let feather = this.miFeatherLimitField.id();
        let leaf = this.miLeafLimitField.id();
        let pwings = this.miPwingsLimitField.id();
        let tanooki = this.miTanookiLimitField.id();
        let bomb = this.miBombLimitField.id();
        let wand = this.miWandLimitField.id();
        let back = this.miProjectilesLimitsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, fireball, back, hammer, null, back);
        this.add_control(g, hammer, fireball, boomerang, null, back);
        this.add_control(g, boomerang, hammer, feather, null, back);
        this.add_control(g, feather, boomerang, leaf, null, back);
        this.add_control(g, leaf, feather, pwings, null, back);
        this.add_control(g, pwings, leaf, tanooki, null, back);
        this.add_control(g, tanooki, pwings, bomb, null, back);
        this.add_control(g, bomb, tanooki, wand, null, back);
        this.add_control(g, wand, bomb, back, null, back);

        this.add_control(g, back, wand, fireball, wand, null);

        let c = this.miProjectilesLimitsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miProjectilesLimitsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miProjectilesLimitsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, fireball);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
