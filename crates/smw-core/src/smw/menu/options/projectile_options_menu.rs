//! Weapons and projectiles options: lifetimes, hammer delay and power, boomerang style, feather jumps.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::gameplay_styles::BoomerangStyle;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;

/// Lifetimes in frames, 60 per labeled second.
const LIFE_1_TO_10: &[(&str, i16)] = &[
    ("1 Second", 60),
    ("2 Seconds", 120),
    ("3 Seconds", 180),
    ("4 Seconds", 240),
    ("5 Seconds", 300),
    ("6 Seconds", 360),
    ("7 Seconds", 420),
    ("8 Seconds", 480),
    ("9 Seconds", 540),
    ("10 Seconds", 600),
];

/// Lifetimes in frames, 60 per labeled second; 0 is unlimited.
const LIFE_UNLIMITED_TO_30: &[(&str, i16)] = &[
    ("Unlimited", 0),
    ("1 Second", 60),
    ("2 Seconds", 120),
    ("3 Seconds", 180),
    ("4 Seconds", 240),
    ("5 Seconds", 300),
    ("6 Seconds", 360),
    ("7 Seconds", 420),
    ("8 Seconds", 480),
    ("9 Seconds", 540),
    ("10 Seconds", 600),
    ("15 Seconds", 900),
    ("20 Seconds", 1200),
    ("25 Seconds", 1500),
    ("30 Seconds", 1800),
];

#[derive(Default)]
/// Per-weapon settings in two columns, each under a weapon name label.
pub struct UI_ProjectileOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miFireballLifeField: Ctl<MI_SelectField<i16>>,
    pub miHammerLifeField: Ctl<MI_SelectField<i16>>,
    pub miHammerDelayField: Ctl<MI_SelectField<i16>>,
    pub miHammerOneKillField: Ctl<MI_SelectField<bool>>,
    pub miShellLifeField: Ctl<MI_SelectField<i16>>,
    pub miWandFreezeTimeField: Ctl<MI_SelectField<i16>>,

    pub miBlueBlockLifeField: Ctl<MI_SelectField<i16>>,
    pub miGrayBlockLifeField: Ctl<MI_SelectField<i16>>,
    pub miRedBlockLifeField: Ctl<MI_SelectField<i16>>,
    pub miBoomerangStyleField: Ctl<MI_SelectField<BoomerangStyle>>,
    pub miBoomerangLifeField: Ctl<MI_SelectField<i16>>,
    pub miFeatherJumpsField: Ctl<MI_SelectField<i16>>,

    pub miProjectileOptionsMenuBackButton: Ctl<MI_Button>,

    pub miFireballText: Ctl<MI_Text>,
    pub miHammerText: Ctl<MI_Text>,
    pub miBoomerangText: Ctl<MI_Text>,
    pub miFeatherText: Ctl<MI_Text>,
    pub miShellText: Ctl<MI_Text>,
    pub miWandText: Ctl<MI_Text>,
    pub miBlueBlockText: Ctl<MI_Text>,

    pub miProjectileOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miProjectileOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miProjectileOptionsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_ProjectileOptionsMenu => ui_menu: UI_Menu);

impl UI_ProjectileOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miFireballLifeField = select_field(spr, 10, 80, "Life", 305, 120, LIFE_1_TO_10, &mut g.game_values.fireballttl, g);

            this.miFeatherJumpsField = select_field(spr, 10, 150, "Jumps", 305, 120,
                &[("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5)],
                &mut g.game_values.featherjumps, g);

            this.miBoomerangStyleField = select_field(spr, 10, 220, "Style", 305, 120,
                &[
                    ("Flat", BoomerangStyle::Flat),
                    ("SMB3", BoomerangStyle::SMB3),
                    ("Zelda", BoomerangStyle::Zelda),
                    ("Random", BoomerangStyle::Random),
                ],
                &mut g.game_values.boomerangstyle, g);

            this.miBoomerangLifeField = select_field(spr, 10, 260, "Life", 305, 120, LIFE_1_TO_10, &mut g.game_values.boomeranglife, g);

            this.miHammerLifeField = select_field(spr, 325, 80, "Life", 305, 120,
                &[
                    ("No Limit", 310),
                    ("0.5 Seconds", 30),
                    ("0.6 Seconds", 36),
                    ("0.7 Seconds", 42),
                    ("0.8 Seconds", 48),
                    ("0.9 Seconds", 54),
                    ("1.0 Seconds", 60),
                    ("1.1 Seconds", 66),
                    ("1.2 Seconds", 72),
                ],
                &mut g.game_values.hammerttl, g);

            this.miHammerDelayField = select_field(spr, 325, 120, "Delay", 305, 120,
                &[
                    ("None", 0),
                    ("0.1 Seconds", 6),
                    ("0.2 Seconds", 12),
                    ("0.3 Seconds", 18),
                    ("0.4 Seconds", 24),
                    ("0.5 Seconds", 30),
                    ("0.6 Seconds", 36),
                    ("0.7 Seconds", 42),
                    ("0.8 Seconds", 48),
                    ("0.9 Seconds", 54),
                    ("1.0 Seconds", 60),
                ],
                &mut g.game_values.hammerdelay, g);

            this.miHammerOneKillField = select_field(spr, 325, 160, "Power", 305, 120,
                &[("One Kill", true), ("Multiple Kills", false)],
                &mut g.game_values.hammerpower, g);
            g.ctl(this.miHammerOneKillField).set_auto_advance(true);

            this.miShellLifeField = select_field(spr, 10, 330, "Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut g.game_values.shellttl, g);

            this.miWandFreezeTimeField = select_field(spr, 10, 400, "Freeze", 305, 120,
                &[
                    ("1 Second", 60),
                    ("2 Seconds", 120),
                    ("3 Seconds", 180),
                    ("4 Seconds", 240),
                    ("5 Seconds", 300),
                    ("6 Seconds", 360),
                    ("7 Seconds", 420),
                    ("8 Seconds", 480),
                    ("9 Seconds", 540),
                    ("10 Seconds", 600),
                    ("12 Seconds", 720),
                    ("15 Seconds", 900),
                    ("18 Seconds", 1080),
                    ("20 Seconds", 1200),
                ],
                &mut g.game_values.wandfreezetime, g);

            this.miBlueBlockLifeField = select_field(spr, 325, 230, "Blue Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut g.game_values.blueblockttl, g);
            this.miGrayBlockLifeField = select_field(spr, 325, 270, "Gray Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut g.game_values.grayblockttl, g);
            this.miRedBlockLifeField = select_field(spr, 325, 310, "Red Life", 305, 120, LIFE_UNLIMITED_TO_30, &mut g.game_values.redblockttl, g);

            this.miProjectileOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miProjectileOptionsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miProjectileOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miProjectileOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miProjectileOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Projectile & Weapon Options Menu", 320, 5), g);

            this.miFireballText = Ctl::new(MI_Text::new(g, "Fireball", 10, 50, 0, true, TextAlign::LEFT), g);
            this.miFeatherText = Ctl::new(MI_Text::new(g, "Feather", 10, 120, 0, true, TextAlign::LEFT), g);
            this.miBoomerangText = Ctl::new(MI_Text::new(g, "Boomerang", 10, 190, 0, true, TextAlign::LEFT), g);
            this.miHammerText = Ctl::new(MI_Text::new(g, "Hammer", 325, 50, 0, true, TextAlign::LEFT), g);
            this.miShellText = Ctl::new(MI_Text::new(g, "Shell", 10, 300, 0, true, TextAlign::LEFT), g);
            this.miWandText = Ctl::new(MI_Text::new(g, "Wand", 10, 370, 0, true, TextAlign::LEFT), g);
            this.miBlueBlockText = Ctl::new(MI_Text::new(g, "Throwable Blocks", 325, 200, 0, true, TextAlign::LEFT), g);
        }

        let fireball = this.miFireballLifeField.id();
        let feather = this.miFeatherJumpsField.id();
        let boomstyle = this.miBoomerangStyleField.id();
        let boomlife = this.miBoomerangLifeField.id();
        let shell = this.miShellLifeField.id();
        let wand = this.miWandFreezeTimeField.id();
        let hammerlife = this.miHammerLifeField.id();
        let hammerdelay = this.miHammerDelayField.id();
        let hammerkill = this.miHammerOneKillField.id();
        let blue = this.miBlueBlockLifeField.id();
        let gray = this.miGrayBlockLifeField.id();
        let red = this.miRedBlockLifeField.id();
        let back = this.miProjectileOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, fireball, back, feather, null, hammerlife);

        this.add_control(g, feather, fireball, boomstyle, null, hammerkill);

        this.add_control(g, boomstyle, feather, boomlife, null, blue);
        this.add_control(g, boomlife, boomstyle, shell, null, gray);

        this.add_control(g, shell, boomlife, wand, null, red);

        this.add_control(g, wand, shell, hammerlife, null, back);

        this.add_control(g, hammerlife, wand, hammerdelay, fireball, null);
        this.add_control(g, hammerdelay, hammerlife, hammerkill, fireball, null);
        this.add_control(g, hammerkill, hammerdelay, blue, feather, null);

        this.add_control(g, blue, hammerkill, gray, boomstyle, null);
        this.add_control(g, gray, blue, red, boomlife, null);
        this.add_control(g, red, gray, back, shell, null);

        this.add_control(g, back, red, fireball, wand, null);

        let c = this.miFireballText.id();
        this.add_non_control(c);
        let c = this.miFeatherText.id();
        this.add_non_control(c);
        let c = this.miBoomerangText.id();
        this.add_non_control(c);
        let c = this.miHammerText.id();
        this.add_non_control(c);
        let c = this.miShellText.id();
        this.add_non_control(c);
        let c = this.miWandText.id();
        this.add_non_control(c);
        let c = this.miBlueBlockText.id();
        this.add_non_control(c);

        let c = this.miProjectileOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miProjectileOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miProjectileOptionsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, fireball);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
