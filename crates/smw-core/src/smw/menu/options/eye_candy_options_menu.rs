//! Eye candy options: spawn, award and scoreboard styles, crunch, crown, countdown and notices.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::eyecandy_styles::{AwardStyle, ScoreboardStyle, SpawnStyle};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::{select_field, OFF_ON};

#[derive(Default)]
/// Visual options: spawn, award and scoreboard styles and on-screen notices.
pub struct UI_EyeCandyOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miSpawnStyleField: Ctl<MI_SelectField<SpawnStyle>>,
    pub miAwardStyleField: Ctl<MI_SelectField<AwardStyle>>,
    pub miScoreStyleField: Ctl<MI_SelectField<ScoreboardStyle>>,
    pub miCrunchField: Ctl<MI_SelectField<bool>>,
    pub miWinningCrownField: Ctl<MI_SelectField<bool>>,
    pub miStartCountDownField: Ctl<MI_SelectField<bool>>,
    pub miStartModeDisplayField: Ctl<MI_SelectField<bool>>,
    pub miDeadTeamNoticeField: Ctl<MI_SelectField<bool>>,

    pub miEyeCandyOptionsMenuBackButton: Ctl<MI_Button>,

    pub miEyeCandyOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miEyeCandyOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miEyeCandyOptionsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_EyeCandyOptionsMenu => ui_menu: UI_Menu);

impl UI_EyeCandyOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miSpawnStyleField = select_field(spr_selectfield, 70, 80, "Spawn Style", 500, 220,
                &[("Instant", SpawnStyle::Instant), ("Door", SpawnStyle::Door), ("Swirl", SpawnStyle::Swirl)],
                &mut g.game_values.spawnstyle, g);

            this.miAwardStyleField = select_field(spr_selectfield, 70, 120, "Award Style", 500, 220,
                &[
                    ("None", AwardStyle::None),
                    ("Fireworks", AwardStyle::Fireworks),
                    ("Spiral", AwardStyle::Swirl),
                    ("Ring", AwardStyle::Halo),
                    ("Souls", AwardStyle::Souls),
                    ("Text", AwardStyle::Text),
                ],
                &mut g.game_values.awardstyle, g);

            this.miScoreStyleField = select_field(spr_selectfield, 70, 160, "Score Location", 500, 220,
                &[("Top", ScoreboardStyle::Top), ("Bottom", ScoreboardStyle::Bottom), ("Corners", ScoreboardStyle::Corners)],
                &mut g.game_values.scoreboardstyle, g);

            this.miCrunchField = select_field(spr_selectfield, 70, 200, "Screen Crunch", 500, 220, OFF_ON, &mut g.game_values.screencrunch, g);
            g.ctl(this.miCrunchField).set_auto_advance(true);

            this.miWinningCrownField = select_field(spr_selectfield, 70, 240, "Leader Crown", 500, 220, OFF_ON, &mut g.game_values.showwinningcrown, g);
            g.ctl(this.miWinningCrownField).set_auto_advance(true);

            this.miStartCountDownField = select_field(spr_selectfield, 70, 280, "Start Countdown", 500, 220, OFF_ON, &mut g.game_values.startgamecountdown, g);
            g.ctl(this.miStartCountDownField).set_auto_advance(true);

            this.miStartModeDisplayField = select_field(spr_selectfield, 70, 320, "Show Mode", 500, 220, OFF_ON, &mut g.game_values.startmodedisplay, g);
            g.ctl(this.miStartModeDisplayField).set_auto_advance(true);

            this.miDeadTeamNoticeField = select_field(spr_selectfield, 70, 360, "Dead Team Notice", 500, 220, OFF_ON, &mut g.game_values.deadteamnotice, g);
            g.ctl(this.miDeadTeamNoticeField).set_auto_advance(true);

            this.miEyeCandyOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miEyeCandyOptionsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miEyeCandyOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miEyeCandyOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miEyeCandyOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Eye Candy Options Menu", 320, 5), g);
        }

        let spawn = this.miSpawnStyleField.id();
        let award = this.miAwardStyleField.id();
        let score = this.miScoreStyleField.id();
        let crunch = this.miCrunchField.id();
        let crown = this.miWinningCrownField.id();
        let countdown = this.miStartCountDownField.id();
        let modedisplay = this.miStartModeDisplayField.id();
        let deadteam = this.miDeadTeamNoticeField.id();
        let back = this.miEyeCandyOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, spawn, back, award, null, back);
        this.add_control(g, award, spawn, score, null, back);
        this.add_control(g, score, award, crunch, null, back);
        this.add_control(g, crunch, score, crown, null, back);
        this.add_control(g, crown, crunch, countdown, null, back);
        this.add_control(g, countdown, crown, modedisplay, null, back);
        this.add_control(g, modedisplay, countdown, deadteam, null, back);
        this.add_control(g, deadteam, modedisplay, back, null, back);

        this.add_control(g, back, deadteam, spawn, deadteam, null);

        let c = this.miEyeCandyOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miEyeCandyOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miEyeCandyOptionsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, spawn);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
