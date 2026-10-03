//! Port of src/smw/menu/options/EyeCandyOptionsMenu.cpp

use crate::common::eyecandy_styles::{AwardStyle, ScoreboardStyle, SpawnStyle};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::{select_field, OFF_ON};

#[derive(Default)]
pub struct UI_EyeCandyOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miSpawnStyleField: Ptr<MI_SelectField<SpawnStyle>>,
    pub miAwardStyleField: Ptr<MI_SelectField<AwardStyle>>,
    pub miScoreStyleField: Ptr<MI_SelectField<ScoreboardStyle>>,
    pub miCrunchField: Ptr<MI_SelectField<bool>>,
    pub miWinningCrownField: Ptr<MI_SelectField<bool>>,
    pub miStartCountDownField: Ptr<MI_SelectField<bool>>,
    pub miStartModeDisplayField: Ptr<MI_SelectField<bool>>,
    pub miDeadTeamNoticeField: Ptr<MI_SelectField<bool>>,

    pub miEyeCandyOptionsMenuBackButton: Ptr<MI_Button>,

    pub miEyeCandyOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miEyeCandyOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miEyeCandyOptionsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_EyeCandyOptionsMenu => ui_menu: UI_Menu);

impl UI_EyeCandyOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miSpawnStyleField = select_field(spr_selectfield, 70, 80, "Spawn Style", 500, 220,
                &[("Instant", SpawnStyle::Instant), ("Door", SpawnStyle::Door), ("Swirl", SpawnStyle::Swirl)],
                &mut game_values.spawnstyle);

            this.miAwardStyleField = select_field(spr_selectfield, 70, 120, "Award Style", 500, 220,
                &[
                    ("None", AwardStyle::None),
                    ("Fireworks", AwardStyle::Fireworks),
                    ("Spiral", AwardStyle::Swirl),
                    ("Ring", AwardStyle::Halo),
                    ("Souls", AwardStyle::Souls),
                    ("Text", AwardStyle::Text),
                ],
                &mut game_values.awardstyle);

            this.miScoreStyleField = select_field(spr_selectfield, 70, 160, "Score Location", 500, 220,
                &[("Top", ScoreboardStyle::Top), ("Bottom", ScoreboardStyle::Bottom), ("Corners", ScoreboardStyle::Corners)],
                &mut game_values.scoreboardstyle);

            this.miCrunchField = select_field(spr_selectfield, 70, 200, "Screen Crunch", 500, 220, OFF_ON, &mut game_values.screencrunch);
            this.miCrunchField.set_auto_advance(true);

            this.miWinningCrownField = select_field(spr_selectfield, 70, 240, "Leader Crown", 500, 220, OFF_ON, &mut game_values.showwinningcrown);
            this.miWinningCrownField.set_auto_advance(true);

            this.miStartCountDownField = select_field(spr_selectfield, 70, 280, "Start Countdown", 500, 220, OFF_ON, &mut game_values.startgamecountdown);
            this.miStartCountDownField.set_auto_advance(true);

            this.miStartModeDisplayField = select_field(spr_selectfield, 70, 320, "Show Mode", 500, 220, OFF_ON, &mut game_values.startmodedisplay);
            this.miStartModeDisplayField.set_auto_advance(true);

            this.miDeadTeamNoticeField = select_field(spr_selectfield, 70, 360, "Dead Team Notice", 500, 220, OFF_ON, &mut game_values.deadteamnotice);
            this.miDeadTeamNoticeField.set_auto_advance(true);

            this.miEyeCandyOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miEyeCandyOptionsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miEyeCandyOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miEyeCandyOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miEyeCandyOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Eye Candy Options Menu", 320, 5));
        }

        let spawn = ctl_ptr(this.miSpawnStyleField);
        let award = ctl_ptr(this.miAwardStyleField);
        let score = ctl_ptr(this.miScoreStyleField);
        let crunch = ctl_ptr(this.miCrunchField);
        let crown = ctl_ptr(this.miWinningCrownField);
        let countdown = ctl_ptr(this.miStartCountDownField);
        let modedisplay = ctl_ptr(this.miStartModeDisplayField);
        let deadteam = ctl_ptr(this.miDeadTeamNoticeField);
        let back = ctl_ptr(this.miEyeCandyOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(spawn, back, award, null, back);
        this.add_control(award, spawn, score, null, back);
        this.add_control(score, award, crunch, null, back);
        this.add_control(crunch, score, crown, null, back);
        this.add_control(crown, crunch, countdown, null, back);
        this.add_control(countdown, crown, modedisplay, null, back);
        this.add_control(modedisplay, countdown, deadteam, null, back);
        this.add_control(deadteam, modedisplay, back, null, back);

        this.add_control(back, deadteam, spawn, deadteam, null);

        let c = ctl_ptr(this.miEyeCandyOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miEyeCandyOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miEyeCandyOptionsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(spawn);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
