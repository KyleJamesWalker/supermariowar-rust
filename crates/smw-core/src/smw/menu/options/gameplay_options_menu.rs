//! Port of src/smw/menu/options/GameplayOptionsMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::gameplay_styles::{ShieldStyle, WarpLockStyle};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::{select_field, OFF_ON};

/*
    This menu is where you can change various gameplay-related
    settings, such as respawn time, bot difficulty or warp locking.
*/
#[derive(Default)]
pub struct UI_GameplayOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miRespawnField: Ctl<MI_SelectField<i16>>,
    pub miShieldStyleField: Ctl<MI_SelectField<ShieldStyle>>,
    pub miShieldTimeField: Ctl<MI_SelectField<i16>>,
    pub miBoundsTimeField: Ctl<MI_SelectField<i16>>,
    pub miSuicideTimeField: Ctl<MI_SelectField<i16>>,
    pub miWarpLockStyleField: Ctl<MI_SelectField<WarpLockStyle>>,
    pub miWarpLockTimeField: Ctl<MI_SelectField<i16>>,
    pub miBotsField: Ctl<MI_SelectField<i16>>,
    pub miPointSpeedField: Ctl<MI_SelectField<i16>>,
    pub miSecretsField: Ctl<MI_SelectField<bool>>,
    pub miGameplayOptionsMenuBackButton: Ctl<MI_Button>,

    pub miGameplayOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miGameplayOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miGameplayOptionsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_GameplayOptionsMenu => ui_menu: UI_Menu);

impl UI_GameplayOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miRespawnField = select_field(spr_selectfield, 70, 40, "Respawn Time", 500, 220,
                &[
                    ("Instant", 0),
                    ("0.5 Seconds", 1),
                    ("1.0 Seconds", 2),
                    ("1.5 Seconds", 3),
                    ("2.0 Seconds", 4),
                    ("2.5 Seconds", 5),
                    ("3.0 Seconds", 6),
                    ("3.5 Seconds", 7),
                    ("4.0 Seconds", 8),
                    ("4.5 Seconds", 9),
                    ("5.0 Seconds", 10),
                    ("5.5 Seconds", 11),
                    ("6.0 Seconds", 12),
                    ("6.5 Seconds", 13),
                    ("7.0 Seconds", 14),
                    ("7.5 Seconds", 15),
                    ("8.0 Seconds", 16),
                    ("8.5 Seconds", 17),
                    ("9.0 Seconds", 18),
                    ("9.5 Seconds", 19),
                    ("10.0 Seconds", 20),
                ],
                &mut g.game_values.respawn, g);

            this.miShieldStyleField = select_field(spr_selectfield, 70, 80, "Shield Style", 500, 220,
                &[
                    ("No Shield", ShieldStyle::NoShield),
                    ("Soft", ShieldStyle::Soft),
                    ("Soft with Stomp", ShieldStyle::SoftWithStomp),
                    ("Hard", ShieldStyle::Hard),
                ],
                &mut g.game_values.shieldstyle, g);

            this.miShieldTimeField = select_field(spr_selectfield, 70, 120, "Shield Time", 500, 220,
                &[
                    ("0.5 Seconds", 30),
                    ("1.0 Seconds", 60),
                    ("1.5 Seconds", 90),
                    ("2.0 Seconds", 120),
                    ("2.5 Seconds", 150),
                    ("3.0 Seconds", 180),
                    ("3.5 Seconds", 210),
                    ("4.0 Seconds", 240),
                    ("4.5 Seconds", 270),
                    ("5.0 Seconds", 300),
                ],
                &mut g.game_values.shieldtime, g);

            this.miBoundsTimeField = select_field(spr_selectfield, 70, 160, "Bounds Time", 500, 220,
                &[
                    ("Infinite", 0),
                    ("1 Second", 1),
                    ("2 Seconds", 2),
                    ("3 Seconds", 3),
                    ("4 Seconds", 4),
                    ("5 Seconds", 5),
                    ("6 Seconds", 6),
                    ("7 Seconds", 7),
                    ("8 Seconds", 8),
                    ("9 Seconds", 9),
                    ("10 Seconds", 10),
                ],
                &mut g.game_values.outofboundstime, g);

            this.miSuicideTimeField = select_field(spr_selectfield, 70, 200, "Suicide Time", 500, 220,
                &[
                    ("Off", 0),
                    ("3 Seconds", 180),
                    ("5 Seconds", 300),
                    ("8 Seconds", 480),
                    ("10 Seconds", 600),
                    ("15 Seconds", 900),
                    ("20 Seconds", 1200),
                ],
                &mut g.game_values.suicidetime, g);

            this.miWarpLockStyleField = select_field(spr_selectfield, 70, 240, "Warp Lock Style", 500, 220,
                &[
                    ("Entrance Only", WarpLockStyle::EntranceOnly),
                    ("Exit Only", WarpLockStyle::ExitOnly),
                    ("Entrance and Exit", WarpLockStyle::EntranceAndExit),
                    ("Entire Connection", WarpLockStyle::EntireConnection),
                    ("All Warps", WarpLockStyle::AllWarps),
                ],
                &mut g.game_values.warplockstyle, g);

            this.miWarpLockTimeField = select_field(spr_selectfield, 70, 280, "Warp Lock Time", 500, 220,
                &[
                    ("Off", 0),
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
                ],
                &mut g.game_values.warplocktime, g);

            this.miBotsField = select_field(spr_selectfield, 70, 320, "Bot Difficulty", 500, 220,
                &[("Very Easy", 0), ("Easy", 1), ("Moderate", 2), ("Hard", 3), ("Very Hard", 4)],
                &mut g.game_values.cpudifficulty, g);

            this.miPointSpeedField = select_field(spr_selectfield, 70, 360, "Point Speed", 500, 220,
                &[("Very Slow", 60), ("Slow", 40), ("Moderate", 20), ("Fast", 10), ("Very Fast", 5)],
                &mut g.game_values.pointspeed, g);

            this.miSecretsField = select_field(spr_selectfield, 70, 400, "Secrets", 500, 220, OFF_ON, &mut g.game_values.secretsenabled, g);

            this.miGameplayOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miGameplayOptionsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miGameplayOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miGameplayOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miGameplayOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Gameplay Options Menu", 320, 5), g);
        }

        let respawn = this.miRespawnField.id();
        let shieldstyle = this.miShieldStyleField.id();
        let shieldtime = this.miShieldTimeField.id();
        let bounds = this.miBoundsTimeField.id();
        let suicide = this.miSuicideTimeField.id();
        let warpstyle = this.miWarpLockStyleField.id();
        let warptime = this.miWarpLockTimeField.id();
        let bots = this.miBotsField.id();
        let pointspeed = this.miPointSpeedField.id();
        let secrets = this.miSecretsField.id();
        let back = this.miGameplayOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, respawn, back, shieldstyle, null, back);
        this.add_control(g, shieldstyle, respawn, shieldtime, null, back);
        this.add_control(g, shieldtime, shieldstyle, bounds, null, back);
        this.add_control(g, bounds, shieldtime, suicide, null, back);
        this.add_control(g, suicide, bounds, warpstyle, null, back);
        this.add_control(g, warpstyle, suicide, warptime, null, back);
        this.add_control(g, warptime, warpstyle, bots, null, back);
        this.add_control(g, bots, warptime, pointspeed, null, back);
        this.add_control(g, pointspeed, bots, secrets, null, back);
        this.add_control(g, secrets, pointspeed, back, null, back);

        this.add_control(g, back, secrets, respawn, secrets, null);

        let c = this.miGameplayOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miGameplayOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miGameplayOptionsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, respawn);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
