//! Port of src/smw/menu/options/ProjectileOptionsMenu.cpp

use crate::common::gameplay_styles::BoomerangStyle;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;

const LIFE_1_TO_10: &[(&str, i16)] = &[
    ("1 Second", 62),
    ("2 Seconds", 124),
    ("3 Seconds", 186),
    ("4 Seconds", 248),
    ("5 Seconds", 310),
    ("6 Seconds", 372),
    ("7 Seconds", 434),
    ("8 Seconds", 496),
    ("9 Seconds", 558),
    ("10 Seconds", 620),
];

const LIFE_UNLIMITED_TO_30: &[(&str, i16)] = &[
    ("Unlimited", 0),
    ("1 Second", 62),
    ("2 Seconds", 124),
    ("3 Seconds", 186),
    ("4 Seconds", 248),
    ("5 Seconds", 310),
    ("6 Seconds", 372),
    ("7 Seconds", 434),
    ("8 Seconds", 496),
    ("9 Seconds", 558),
    ("10 Seconds", 620),
    ("15 Seconds", 930),
    ("20 Seconds", 1240),
    ("25 Seconds", 1550),
    ("30 Seconds", 1860),
];

#[derive(Default)]
pub struct UI_ProjectileOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miFireballLifeField: Ptr<MI_SelectField<i16>>,
    pub miHammerLifeField: Ptr<MI_SelectField<i16>>,
    pub miHammerDelayField: Ptr<MI_SelectField<i16>>,
    pub miHammerOneKillField: Ptr<MI_SelectField<bool>>,
    pub miShellLifeField: Ptr<MI_SelectField<i16>>,
    pub miWandFreezeTimeField: Ptr<MI_SelectField<i16>>,

    pub miBlueBlockLifeField: Ptr<MI_SelectField<i16>>,
    pub miGrayBlockLifeField: Ptr<MI_SelectField<i16>>,
    pub miRedBlockLifeField: Ptr<MI_SelectField<i16>>,
    pub miBoomerangStyleField: Ptr<MI_SelectField<BoomerangStyle>>,
    pub miBoomerangLifeField: Ptr<MI_SelectField<i16>>,
    pub miFeatherJumpsField: Ptr<MI_SelectField<i16>>,

    pub miProjectileOptionsMenuBackButton: Ptr<MI_Button>,

    pub miFireballText: Ptr<MI_Text>,
    pub miHammerText: Ptr<MI_Text>,
    pub miBoomerangText: Ptr<MI_Text>,
    pub miFeatherText: Ptr<MI_Text>,
    pub miShellText: Ptr<MI_Text>,
    pub miWandText: Ptr<MI_Text>,
    pub miBlueBlockText: Ptr<MI_Text>,

    pub miProjectileOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miProjectileOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miProjectileOptionsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_ProjectileOptionsMenu => ui_menu: UI_Menu);

impl UI_ProjectileOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miFireballLifeField = select_field(spr, 10, 80, "Life", 305, 120, LIFE_1_TO_10, &mut game_values.fireballttl);

            this.miFeatherJumpsField = select_field(spr, 10, 150, "Jumps", 305, 120,
                &[("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5)],
                &mut game_values.featherjumps);

            this.miBoomerangStyleField = select_field(spr, 10, 220, "Style", 305, 120,
                &[
                    ("Flat", BoomerangStyle::Flat),
                    ("SMB3", BoomerangStyle::SMB3),
                    ("Zelda", BoomerangStyle::Zelda),
                    ("Random", BoomerangStyle::Random),
                ],
                &mut game_values.boomerangstyle);

            this.miBoomerangLifeField = select_field(spr, 10, 260, "Life", 305, 120, LIFE_1_TO_10, &mut game_values.boomeranglife);

            this.miHammerLifeField = select_field(spr, 325, 80, "Life", 305, 120,
                &[
                    ("No Limit", 310),
                    ("0.5 Seconds", 31),
                    ("0.6 Seconds", 37),
                    ("0.7 Seconds", 43),
                    ("0.8 Seconds", 49),
                    ("0.9 Seconds", 55),
                    ("1.0 Seconds", 62),
                    ("1.1 Seconds", 68),
                    ("1.2 Seconds", 74),
                ],
                &mut game_values.hammerttl);

            this.miHammerDelayField = select_field(spr, 325, 120, "Delay", 305, 120,
                &[
                    ("None", 0),
                    ("0.1 Seconds", 6),
                    ("0.2 Seconds", 12),
                    ("0.3 Seconds", 19),
                    ("0.4 Seconds", 25),
                    ("0.5 Seconds", 31),
                    ("0.6 Seconds", 37),
                    ("0.7 Seconds", 43),
                    ("0.8 Seconds", 49),
                    ("0.9 Seconds", 55),
                    ("1.0 Seconds", 62),
                ],
                &mut game_values.hammerdelay);

            this.miHammerOneKillField = select_field(spr, 325, 160, "Power", 305, 120,
                &[("One Kill", true), ("Multiple Kills", false)],
                &mut game_values.hammerpower);
            this.miHammerOneKillField.set_auto_advance(true);

            this.miShellLifeField = select_field(spr, 10, 330, "Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut game_values.shellttl);

            this.miWandFreezeTimeField = select_field(spr, 10, 400, "Freeze", 305, 120,
                &[
                    ("1 Second", 62),
                    ("2 Seconds", 124),
                    ("3 Seconds", 186),
                    ("4 Seconds", 248),
                    ("5 Seconds", 310),
                    ("6 Seconds", 372),
                    ("7 Seconds", 434),
                    ("8 Seconds", 496),
                    ("9 Seconds", 558),
                    ("10 Seconds", 620),
                    ("12 Seconds", 744),
                    ("15 Seconds", 930),
                    ("18 Seconds", 1116),
                    ("20 Seconds", 1240),
                ],
                &mut game_values.wandfreezetime);

            this.miBlueBlockLifeField = select_field(spr, 325, 230, "Blue Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut game_values.blueblockttl);
            this.miGrayBlockLifeField = select_field(spr, 325, 270, "Gray Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut game_values.grayblockttl);
            this.miRedBlockLifeField = select_field(spr, 325, 310, "Red Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut game_values.redblockttl);

            this.miProjectileOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miProjectileOptionsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miProjectileOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miProjectileOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miProjectileOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Projectile & Weapon Options Menu", 320, 5));

            this.miFireballText = Ptr::new_box(MI_Text::new("Fireball", 10, 50, 0, true, TextAlign::LEFT));
            this.miFeatherText = Ptr::new_box(MI_Text::new("Feather", 10, 120, 0, true, TextAlign::LEFT));
            this.miBoomerangText = Ptr::new_box(MI_Text::new("Boomerang", 10, 190, 0, true, TextAlign::LEFT));
            this.miHammerText = Ptr::new_box(MI_Text::new("Hammer", 325, 50, 0, true, TextAlign::LEFT));
            this.miShellText = Ptr::new_box(MI_Text::new("Shell", 10, 300, 0, true, TextAlign::LEFT));
            this.miWandText = Ptr::new_box(MI_Text::new("Wand", 10, 370, 0, true, TextAlign::LEFT));
            this.miBlueBlockText = Ptr::new_box(MI_Text::new("Throwable Blocks", 325, 200, 0, true, TextAlign::LEFT));
        }

        let fireball = ctl_ptr(this.miFireballLifeField);
        let feather = ctl_ptr(this.miFeatherJumpsField);
        let boomstyle = ctl_ptr(this.miBoomerangStyleField);
        let boomlife = ctl_ptr(this.miBoomerangLifeField);
        let shell = ctl_ptr(this.miShellLifeField);
        let wand = ctl_ptr(this.miWandFreezeTimeField);
        let hammerlife = ctl_ptr(this.miHammerLifeField);
        let hammerdelay = ctl_ptr(this.miHammerDelayField);
        let hammerkill = ctl_ptr(this.miHammerOneKillField);
        let blue = ctl_ptr(this.miBlueBlockLifeField);
        let gray = ctl_ptr(this.miGrayBlockLifeField);
        let red = ctl_ptr(this.miRedBlockLifeField);
        let back = ctl_ptr(this.miProjectileOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(fireball, back, feather, null, hammerlife);

        this.add_control(feather, fireball, boomstyle, null, hammerkill);

        this.add_control(boomstyle, feather, boomlife, null, blue);
        this.add_control(boomlife, boomstyle, shell, null, gray);

        this.add_control(shell, boomlife, wand, null, red);

        this.add_control(wand, shell, hammerlife, null, back);

        this.add_control(hammerlife, wand, hammerdelay, fireball, null);
        this.add_control(hammerdelay, hammerlife, hammerkill, fireball, null);
        this.add_control(hammerkill, hammerdelay, blue, feather, null);

        this.add_control(blue, hammerkill, gray, boomstyle, null);
        this.add_control(gray, blue, red, boomlife, null);
        this.add_control(red, gray, back, shell, null);

        this.add_control(back, red, fireball, wand, null);

        let c = ctl_ptr(this.miFireballText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miFeatherText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miBoomerangText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miHammerText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miShellText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miWandText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miBlueBlockText);
        this.add_non_control(c);

        let c = ctl_ptr(this.miProjectileOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miProjectileOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miProjectileOptionsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(fireball);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
