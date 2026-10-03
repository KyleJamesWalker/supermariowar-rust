//! Port of src/smw/menu/options/ProjectileLimitsMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;

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

    pub miFireballLimitField: Ptr<MI_SelectField<i16>>,
    pub miHammerLimitField: Ptr<MI_SelectField<i16>>,
    pub miBoomerangLimitField: Ptr<MI_SelectField<i16>>,
    pub miFeatherLimitField: Ptr<MI_SelectField<i16>>,
    pub miLeafLimitField: Ptr<MI_SelectField<i16>>,
    pub miPwingsLimitField: Ptr<MI_SelectField<i16>>,
    pub miTanookiLimitField: Ptr<MI_SelectField<i16>>,
    pub miBombLimitField: Ptr<MI_SelectField<i16>>,
    pub miWandLimitField: Ptr<MI_SelectField<i16>>,

    pub miProjectilesLimitsMenuBackButton: Ptr<MI_Button>,

    pub miProjectilesLimitsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miProjectilesLimitsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miProjectilesLimitsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_ProjectileLimitsMenu => ui_menu: UI_Menu);

impl UI_ProjectileLimitsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miFireballLimitField = select_field(spr, 70, 60, "Fireball Limit", 500, 220, LIMITS, &mut game_values.fireballlimit);
            this.miHammerLimitField = select_field(spr, 70, 100, "Hammer Limit", 500, 220, LIMITS, &mut game_values.hammerlimit);
            this.miBoomerangLimitField = select_field(spr, 70, 140, "Boomerang Limit", 500, 220, LIMITS, &mut game_values.boomeranglimit);
            this.miFeatherLimitField = select_field(spr, 70, 180, "Feather Limit", 500, 220, LIMITS, &mut game_values.featherlimit);
            this.miLeafLimitField = select_field(spr, 70, 220, "Leaf Limit", 500, 220, LIMITS, &mut game_values.leaflimit);
            this.miPwingsLimitField = select_field(spr, 70, 260, "P-Wings Limit", 500, 220, LIMITS, &mut game_values.pwingslimit);
            this.miTanookiLimitField = select_field(spr, 70, 300, "Tanooki Limit", 500, 220, LIMITS, &mut game_values.tanookilimit);
            this.miBombLimitField = select_field(spr, 70, 340, "Bomb Limit", 500, 220, LIMITS, &mut game_values.bombslimit);
            this.miWandLimitField = select_field(spr, 70, 380, "Wand Limit", 500, 220, LIMITS, &mut game_values.wandlimit);

            this.miProjectilesLimitsMenuBackButton = Ptr::new_box(MI_Button::new(spr, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miProjectilesLimitsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miProjectilesLimitsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miProjectilesLimitsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miProjectilesLimitsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Weapon Use Limits Menu", 320, 5));
        }

        let fireball = ctl_ptr(this.miFireballLimitField);
        let hammer = ctl_ptr(this.miHammerLimitField);
        let boomerang = ctl_ptr(this.miBoomerangLimitField);
        let feather = ctl_ptr(this.miFeatherLimitField);
        let leaf = ctl_ptr(this.miLeafLimitField);
        let pwings = ctl_ptr(this.miPwingsLimitField);
        let tanooki = ctl_ptr(this.miTanookiLimitField);
        let bomb = ctl_ptr(this.miBombLimitField);
        let wand = ctl_ptr(this.miWandLimitField);
        let back = ctl_ptr(this.miProjectilesLimitsMenuBackButton);
        let null = Ptr::null();

        this.add_control(fireball, back, hammer, null, back);
        this.add_control(hammer, fireball, boomerang, null, back);
        this.add_control(boomerang, hammer, feather, null, back);
        this.add_control(feather, boomerang, leaf, null, back);
        this.add_control(leaf, feather, pwings, null, back);
        this.add_control(pwings, leaf, tanooki, null, back);
        this.add_control(tanooki, pwings, bomb, null, back);
        this.add_control(bomb, tanooki, wand, null, back);
        this.add_control(wand, bomb, back, null, back);

        this.add_control(back, wand, fireball, wand, null);

        let c = ctl_ptr(this.miProjectilesLimitsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miProjectilesLimitsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miProjectilesLimitsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(fireball);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
