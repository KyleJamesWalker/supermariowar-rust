//! Port of src/smw/menu/options/GameplayOptionsMenu.cpp

use crate::common::gameplay_styles::{ShieldStyle, WarpLockStyle};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
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

    pub miRespawnField: Ptr<MI_SelectField<i16>>,
    pub miShieldStyleField: Ptr<MI_SelectField<ShieldStyle>>,
    pub miShieldTimeField: Ptr<MI_SelectField<i16>>,
    pub miBoundsTimeField: Ptr<MI_SelectField<i16>>,
    pub miSuicideTimeField: Ptr<MI_SelectField<i16>>,
    pub miWarpLockStyleField: Ptr<MI_SelectField<WarpLockStyle>>,
    pub miWarpLockTimeField: Ptr<MI_SelectField<i16>>,
    pub miBotsField: Ptr<MI_SelectField<i16>>,
    pub miPointSpeedField: Ptr<MI_SelectField<i16>>,
    pub miSecretsField: Ptr<MI_SelectField<bool>>,
    pub miGameplayOptionsMenuBackButton: Ptr<MI_Button>,

    pub miGameplayOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miGameplayOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miGameplayOptionsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_GameplayOptionsMenu => ui_menu: UI_Menu);

impl UI_GameplayOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

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
                &mut game_values.respawn);

            this.miShieldStyleField = select_field(spr_selectfield, 70, 80, "Shield Style", 500, 220,
                &[
                    ("No Shield", ShieldStyle::NoShield),
                    ("Soft", ShieldStyle::Soft),
                    ("Soft with Stomp", ShieldStyle::SoftWithStomp),
                    ("Hard", ShieldStyle::Hard),
                ],
                &mut game_values.shieldstyle);

            this.miShieldTimeField = select_field(spr_selectfield, 70, 120, "Shield Time", 500, 220,
                &[
                    ("0.5 Seconds", 31),
                    ("1.0 Seconds", 62),
                    ("1.5 Seconds", 93),
                    ("2.0 Seconds", 124),
                    ("2.5 Seconds", 155),
                    ("3.0 Seconds", 186),
                    ("3.5 Seconds", 217),
                    ("4.0 Seconds", 248),
                    ("4.5 Seconds", 279),
                    ("5.0 Seconds", 310),
                ],
                &mut game_values.shieldtime);

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
                &mut game_values.outofboundstime);

            this.miSuicideTimeField = select_field(spr_selectfield, 70, 200, "Suicide Time", 500, 220,
                &[
                    ("Off", 0),
                    ("3 Seconds", 186),
                    ("5 Seconds", 310),
                    ("8 Seconds", 496),
                    ("10 Seconds", 620),
                    ("15 Seconds", 930),
                    ("20 Seconds", 1240),
                ],
                &mut game_values.suicidetime);

            this.miWarpLockStyleField = select_field(spr_selectfield, 70, 240, "Warp Lock Style", 500, 220,
                &[
                    ("Entrance Only", WarpLockStyle::EntranceOnly),
                    ("Exit Only", WarpLockStyle::ExitOnly),
                    ("Entrance and Exit", WarpLockStyle::EntranceAndExit),
                    ("Entire Connection", WarpLockStyle::EntireConnection),
                    ("All Warps", WarpLockStyle::AllWarps),
                ],
                &mut game_values.warplockstyle);

            this.miWarpLockTimeField = select_field(spr_selectfield, 70, 280, "Warp Lock Time", 500, 220,
                &[
                    ("Off", 0),
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
                ],
                &mut game_values.warplocktime);

            this.miBotsField = select_field(spr_selectfield, 70, 320, "Bot Difficulty", 500, 220,
                &[("Very Easy", 0), ("Easy", 1), ("Moderate", 2), ("Hard", 3), ("Very Hard", 4)],
                &mut game_values.cpudifficulty);

            this.miPointSpeedField = select_field(spr_selectfield, 70, 360, "Point Speed", 500, 220,
                &[("Very Slow", 60), ("Slow", 40), ("Moderate", 20), ("Fast", 10), ("Very Fast", 5)],
                &mut game_values.pointspeed);

            this.miSecretsField = select_field(spr_selectfield, 70, 400, "Secrets", 500, 220, OFF_ON, &mut game_values.secretsenabled);

            this.miGameplayOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miGameplayOptionsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miGameplayOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miGameplayOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miGameplayOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Gameplay Options Menu", 320, 5));
        }

        let respawn = ctl_ptr(this.miRespawnField);
        let shieldstyle = ctl_ptr(this.miShieldStyleField);
        let shieldtime = ctl_ptr(this.miShieldTimeField);
        let bounds = ctl_ptr(this.miBoundsTimeField);
        let suicide = ctl_ptr(this.miSuicideTimeField);
        let warpstyle = ctl_ptr(this.miWarpLockStyleField);
        let warptime = ctl_ptr(this.miWarpLockTimeField);
        let bots = ctl_ptr(this.miBotsField);
        let pointspeed = ctl_ptr(this.miPointSpeedField);
        let secrets = ctl_ptr(this.miSecretsField);
        let back = ctl_ptr(this.miGameplayOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(respawn, back, shieldstyle, null, back);
        this.add_control(shieldstyle, respawn, shieldtime, null, back);
        this.add_control(shieldtime, shieldstyle, bounds, null, back);
        this.add_control(bounds, shieldtime, suicide, null, back);
        this.add_control(suicide, bounds, warpstyle, null, back);
        this.add_control(warpstyle, suicide, warptime, null, back);
        this.add_control(warptime, warpstyle, bots, null, back);
        this.add_control(bots, warptime, pointspeed, null, back);
        this.add_control(pointspeed, bots, secrets, null, back);
        this.add_control(secrets, pointspeed, back, null, back);

        this.add_control(back, secrets, respawn, secrets, null);

        let c = ctl_ptr(this.miGameplayOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGameplayOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGameplayOptionsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(respawn);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
