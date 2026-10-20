//! The per-mode settings menus opened from game setup's Settings button.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::gameplay_styles::{DeathStyle, JailStyle, ScoringStyle, StarStyle};
use crate::common::global_constants::{MAXRACEGOALS, NUMSTOMPENEMIES, NUMSURVIVALENEMIES};
use crate::common::match_types::Boss;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_slider_field::MI_SliderField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_frenzy_mode_options::MI_FrenzyModeOptions;
use crate::smw::ui::mi_powerup_slider::MI_PowerupSlider;

#[derive(Default)]
/// One settings menu per game mode plus the boss minigame; fields write `game_values.gamemodemenusettings`.
pub struct UI_ModeOptionsMenu {
    pub ui_menu: UI_Menu,

    // Game Mode Settings Menu
    /// Settings menu of each game mode, indexed by mode.
    pub mModeSettingsMenu: [UI_Menu; 22],
    pub mBossSettingsMenu: UI_Menu,
    // Classic
    pub miClassicModeStyleField: Ctl<MI_SelectField<DeathStyle>>,
    pub miClassicModeScoringField: Ctl<MI_SelectField<ScoringStyle>>,
    pub miClassicModeBackButton: Ctl<MI_Button>,
    pub miClassicModeLeftHeaderBar: Ctl<MI_Image>,
    pub miClassicModeRightHeaderBar: Ctl<MI_Image>,
    pub miClassicModeHeaderText: Ctl<MI_Text>,
    // Frag
    pub miFragModeStyleField: Ctl<MI_SelectField<DeathStyle>>,
    pub miFragModeScoringField: Ctl<MI_SelectField<ScoringStyle>>,
    pub miFragModeBackButton: Ctl<MI_Button>,
    pub miFragModeLeftHeaderBar: Ctl<MI_Image>,
    pub miFragModeRightHeaderBar: Ctl<MI_Image>,
    pub miFragModeHeaderText: Ctl<MI_Text>,
    // Time Limit
    pub miTimeLimitModeStyleField: Ctl<MI_SelectField<DeathStyle>>,
    pub miTimeLimitModeScoringField: Ctl<MI_SelectField<ScoringStyle>>,
    pub miTimeLimitModePercentExtraTime: Ctl<MI_SliderField>,
    pub miTimeLimitModeBackButton: Ctl<MI_Button>,
    pub miTimeLimitModeLeftHeaderBar: Ctl<MI_Image>,
    pub miTimeLimitModeRightHeaderBar: Ctl<MI_Image>,
    pub miTimeLimitModeHeaderText: Ctl<MI_Text>,
    // Jail
    pub miJailModeStyleField: Ctl<MI_SelectField<JailStyle>>,
    pub miJailModeTagFreeField: Ctl<MI_SelectField<bool>>,
    pub miJailModeTimeFreeField: Ctl<MI_SelectField<i16>>,
    pub miJailModeJailKeyField: Ctl<MI_SliderField>,
    pub miJailModeBackButton: Ctl<MI_Button>,
    pub miJailModeLeftHeaderBar: Ctl<MI_Image>,
    pub miJailModeRightHeaderBar: Ctl<MI_Image>,
    pub miJailModeHeaderText: Ctl<MI_Text>,
    // Coins
    pub miCoinModePenaltyField: Ctl<MI_SelectField<bool>>,
    pub miCoinModeQuantityField: Ctl<MI_SelectField<i16>>,
    pub miCoinModePercentExtraCoin: Ctl<MI_SliderField>,
    pub miCoinModeBackButton: Ctl<MI_Button>,
    pub miCoinModeLeftHeaderBar: Ctl<MI_Image>,
    pub miCoinModeRightHeaderBar: Ctl<MI_Image>,
    pub miCoinModeHeaderText: Ctl<MI_Text>,
    // Yoshi's Eggs
    pub miEggModeEggQuantityField: [Ctl<MI_PowerupSlider>; 4],
    pub miEggModeYoshiQuantityField: [Ctl<MI_PowerupSlider>; 4],
    pub miEggModeExplosionTimeField: Ctl<MI_SelectField<i16>>,
    pub miEggModeBackButton: Ctl<MI_Button>,
    pub miEggModeLeftHeaderBar: Ctl<MI_Image>,
    pub miEggModeRightHeaderBar: Ctl<MI_Image>,
    pub miEggModeHeaderText: Ctl<MI_Text>,
    // Capture The Flag
    pub miFlagModeSpeedField: Ctl<MI_SliderField>,
    pub miFlagModeTouchReturnField: Ctl<MI_SelectField<bool>>,
    pub miFlagModePointMoveField: Ctl<MI_SelectField<bool>>,
    pub miFlagModeAutoReturnField: Ctl<MI_SelectField<i16>>,
    pub miFlagModeHomeScoreField: Ctl<MI_SelectField<bool>>,
    pub miFlagModeCenterFlagField: Ctl<MI_SelectField<bool>>,
    pub miFlagModeBackButton: Ctl<MI_Button>,
    pub miFlagModeLeftHeaderBar: Ctl<MI_Image>,
    pub miFlagModeRightHeaderBar: Ctl<MI_Image>,
    pub miFlagModeHeaderText: Ctl<MI_Text>,
    // Chicken
    pub miChickenModeShowTargetField: Ctl<MI_SelectField<bool>>,
    pub miChickenModeGlideField: Ctl<MI_SelectField<bool>>,
    pub miChickenModeBackButton: Ctl<MI_Button>,
    pub miChickenModeLeftHeaderBar: Ctl<MI_Image>,
    pub miChickenModeRightHeaderBar: Ctl<MI_Image>,
    pub miChickenModeHeaderText: Ctl<MI_Text>,
    // Tag
    pub miTagModeTagOnTouchField: Ctl<MI_SelectField<bool>>,
    pub miTagModeBackButton: Ctl<MI_Button>,
    pub miTagModeLeftHeaderBar: Ctl<MI_Image>,
    pub miTagModeRightHeaderBar: Ctl<MI_Image>,
    pub miTagModeHeaderText: Ctl<MI_Text>,
    // Star
    pub miStarModeTimeField: Ctl<MI_SelectField<i16>>,
    pub miStarModeShineField: Ctl<MI_SelectField<StarStyle>>,
    pub miStarModePercentExtraTime: Ctl<MI_SliderField>,
    pub miStarModeBackButton: Ctl<MI_Button>,
    pub miStarModeLeftHeaderBar: Ctl<MI_Image>,
    pub miStarModeRightHeaderBar: Ctl<MI_Image>,
    pub miStarModeHeaderText: Ctl<MI_Text>,
    // Domination
    pub miDominationModeQuantityField: Ctl<MI_SelectField<i16>>,
    pub miDominationModeLoseOnDeathField: Ctl<MI_SelectField<bool>>,
    pub miDominationModeRelocateOnDeathField: Ctl<MI_SelectField<bool>>,
    pub miDominationModeStealOnDeathField: Ctl<MI_SelectField<bool>>,
    pub miDominationModeRelocateFrequencyField: Ctl<MI_SelectField<i16>>,
    pub miDominationModeDeathText: Ctl<MI_Text>,
    pub miDominationModeBackButton: Ctl<MI_Button>,
    pub miDominationModeLeftHeaderBar: Ctl<MI_Image>,
    pub miDominationModeRightHeaderBar: Ctl<MI_Image>,
    pub miDominationModeHeaderText: Ctl<MI_Text>,
    // King of the Hill
    pub miKingOfTheHillModeSizeField: Ctl<MI_SelectField<i16>>,
    pub miKingOfTheHillModeRelocateFrequencyField: Ctl<MI_SelectField<i16>>,
    pub miKingOfTheHillModeMultiplierField: Ctl<MI_SelectField<i16>>,
    pub miKingOfTheHillModeBackButton: Ctl<MI_Button>,
    pub miKingOfTheHillModeLeftHeaderBar: Ctl<MI_Image>,
    pub miKingOfTheHillModeRightHeaderBar: Ctl<MI_Image>,
    pub miKingOfTheHillModeHeaderText: Ctl<MI_Text>,
    // Race
    pub miRaceModeQuantityField: Ctl<MI_SelectField<i16>>,
    pub miRaceModeSpeedField: Ctl<MI_SelectField<i16>>,
    pub miRaceModePenaltyField: Ctl<MI_SelectField<i16>>,
    pub miRaceModeBackButton: Ctl<MI_Button>,
    pub miRaceModeLeftHeaderBar: Ctl<MI_Image>,
    pub miRaceModeRightHeaderBar: Ctl<MI_Image>,
    pub miRaceModeHeaderText: Ctl<MI_Text>,
    // Stomp
    pub miStompModeRateField: Ctl<MI_SelectField<i16>>,
    pub miStompModeEnemySlider: [Ctl<MI_PowerupSlider>; NUMSTOMPENEMIES as usize],
    pub miStompModeBackButton: Ctl<MI_Button>,
    pub miStompModeLeftHeaderBar: Ctl<MI_Image>,
    pub miStompModeRightHeaderBar: Ctl<MI_Image>,
    pub miStompModeHeaderText: Ctl<MI_Text>,
    // Frenzy
    pub miFrenzyModeOptions: Ctl<MI_FrenzyModeOptions>,
    pub miFrenzyModeLeftHeaderBar: Ctl<MI_Image>,
    pub miFrenzyModeRightHeaderBar: Ctl<MI_Image>,
    pub miFrenzyModeHeaderText: Ctl<MI_Text>,
    // Survival
    pub miSurvivalModeEnemySlider: [Ctl<MI_PowerupSlider>; NUMSURVIVALENEMIES as usize],
    pub miSurvivalModeDensityField: Ctl<MI_SelectField<i16>>,
    pub miSurvivalModeSpeedField: Ctl<MI_SelectField<i16>>,
    pub miSurvivalModeShieldField: Ctl<MI_SelectField<bool>>,
    pub miSurvivalModeBackButton: Ctl<MI_Button>,
    pub miSurvivalModeLeftHeaderBar: Ctl<MI_Image>,
    pub miSurvivalModeRightHeaderBar: Ctl<MI_Image>,
    pub miSurvivalModeHeaderText: Ctl<MI_Text>,
    // Greed
    pub miGreedModeCoinLife: Ctl<MI_SelectField<i16>>,
    pub miGreedModeOwnCoins: Ctl<MI_SelectField<bool>>,
    pub miGreedModeMultiplier: Ctl<MI_SelectField<i16>>,
    pub miGreedModePercentExtraCoin: Ctl<MI_SliderField>,
    pub miGreedModeBackButton: Ctl<MI_Button>,
    pub miGreedModeLeftHeaderBar: Ctl<MI_Image>,
    pub miGreedModeRightHeaderBar: Ctl<MI_Image>,
    pub miGreedModeHeaderText: Ctl<MI_Text>,
    // Health
    pub miHealthModeStartLife: Ctl<MI_SelectField<i16>>,
    pub miHealthModeMaxLife: Ctl<MI_SelectField<i16>>,
    pub miHealthModePercentExtraLife: Ctl<MI_SliderField>,
    pub miHealthModeBackButton: Ctl<MI_Button>,
    pub miHealthModeLeftHeaderBar: Ctl<MI_Image>,
    pub miHealthModeRightHeaderBar: Ctl<MI_Image>,
    pub miHealthModeHeaderText: Ctl<MI_Text>,
    // Card Collection
    pub miCollectionModeQuantityField: Ctl<MI_SelectField<i16>>,
    pub miCollectionModeRateField: Ctl<MI_SelectField<i16>>,
    pub miCollectionModeBankTimeField: Ctl<MI_SelectField<i16>>,
    pub miCollectionModeCardLifeField: Ctl<MI_SelectField<i16>>,
    pub miCollectionModeBackButton: Ctl<MI_Button>,
    pub miCollectionModeLeftHeaderBar: Ctl<MI_Image>,
    pub miCollectionModeRightHeaderBar: Ctl<MI_Image>,
    pub miCollectionModeHeaderText: Ctl<MI_Text>,
    // Phanto Chase
    pub miChaseModeSpeedField: Ctl<MI_SelectField<i16>>,
    pub miChaseModeQuantitySlider: [Ctl<MI_PowerupSlider>; 3],
    pub miChaseModeBackButton: Ctl<MI_Button>,
    pub miChaseModeLeftHeaderBar: Ctl<MI_Image>,
    pub miChaseModeRightHeaderBar: Ctl<MI_Image>,
    pub miChaseModeHeaderText: Ctl<MI_Text>,
    // Shyguy Tag
    pub miShyGuyTagModeTagOnSuicideField: Ctl<MI_SelectField<bool>>,
    pub miShyGuyTagModeTagOnStompField: Ctl<MI_SelectField<i16>>,
    pub miShyGuyTagModeFreeTimeField: Ctl<MI_SelectField<i16>>,
    pub miShyGuyTagModeBackButton: Ctl<MI_Button>,
    pub miShyGuyTagModeLeftHeaderBar: Ctl<MI_Image>,
    pub miShyGuyTagModeRightHeaderBar: Ctl<MI_Image>,
    pub miShyGuyTagModeHeaderText: Ctl<MI_Text>,
    // Boss
    pub miBossModeTypeField: Ctl<MI_SelectField<Boss>>,
    pub miBossModeDifficultyField: Ctl<MI_SelectField<i16>>,
    pub miBossModeHitPointsField: Ctl<MI_SelectField<i16>>,
    pub miBossModeBackButton: Ctl<MI_Button>,
    pub miBossModeLeftHeaderBar: Ctl<MI_Image>,
    pub miBossModeRightHeaderBar: Ctl<MI_Image>,
    pub miBossModeHeaderText: Ctl<MI_Text>,
}

crate::impl_base!(UI_ModeOptionsMenu => ui_menu: UI_Menu);

impl UI_ModeOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        //***********************
        // Classic Mode Settings
        //***********************

        this.miClassicModeStyleField = Ctl::new(MI_SelectField::<DeathStyle>::new(g.rm.spr_selectfield, 120, 200, "On Kill", 400, 180), g);
        g.ctl(this.miClassicModeStyleField).add("Respawn", DeathStyle::Respawn);
        g.ctl(this.miClassicModeStyleField).add("Shield", DeathStyle::Shield);
        g.ctl(this.miClassicModeStyleField).set_output_ptr(&mut g.game_values.gamemodemenusettings.classic.style as *mut _);
        g.ctl(this.miClassicModeStyleField).set_current_value(g.game_values.gamemodemenusettings.classic.style);

        this.miClassicModeScoringField = Ctl::new(MI_SelectField::<ScoringStyle>::new(g.rm.spr_selectfield, 120, 240, "Scoring", 400, 180), g);
        g.ctl(this.miClassicModeScoringField).add("All Kills", ScoringStyle::AllKills);
        g.ctl(this.miClassicModeScoringField).add_random("Push Kills Only", ScoringStyle::PushOnly, false);
        g.ctl(this.miClassicModeScoringField).set_output_ptr(&mut g.game_values.gamemodemenusettings.classic.scoring as *mut _);
        g.ctl(this.miClassicModeScoringField).set_current_value(g.game_values.gamemodemenusettings.classic.scoring);

        this.miClassicModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miClassicModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miClassicModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miClassicModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miClassicModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Classic Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[0].add_control(g, this.miClassicModeStyleField.id(), this.miClassicModeBackButton.id(), this.miClassicModeScoringField.id(), Default::default(), this.miClassicModeBackButton.id());
        this.mModeSettingsMenu[0].add_control(g, this.miClassicModeScoringField.id(), this.miClassicModeStyleField.id(), this.miClassicModeBackButton.id(), Default::default(), this.miClassicModeBackButton.id());
        this.mModeSettingsMenu[0].add_control(g, this.miClassicModeBackButton.id(), this.miClassicModeScoringField.id(), this.miClassicModeStyleField.id(), this.miClassicModeScoringField.id(), Default::default());

        this.mModeSettingsMenu[0].add_non_control(this.miClassicModeLeftHeaderBar.id());
        this.mModeSettingsMenu[0].add_non_control(this.miClassicModeRightHeaderBar.id());
        this.mModeSettingsMenu[0].add_non_control(this.miClassicModeHeaderText.id());

        this.mModeSettingsMenu[0].set_initial_focus(g, this.miClassicModeStyleField.id());
        this.mModeSettingsMenu[0].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Frag Mode Settings
        //***********************

        this.miFragModeStyleField = Ctl::new(MI_SelectField::<DeathStyle>::new(g.rm.spr_selectfield, 120, 200, "On Kill", 400, 180), g);
        g.ctl(this.miFragModeStyleField).add("Respawn", DeathStyle::Respawn);
        g.ctl(this.miFragModeStyleField).add("Shield", DeathStyle::Shield);
        g.ctl(this.miFragModeStyleField).set_output_ptr(&mut g.game_values.gamemodemenusettings.frag.style as *mut _);
        g.ctl(this.miFragModeStyleField).set_current_value(g.game_values.gamemodemenusettings.frag.style);

        this.miFragModeScoringField = Ctl::new(MI_SelectField::<ScoringStyle>::new(g.rm.spr_selectfield, 120, 240, "Scoring", 400, 180), g);
        g.ctl(this.miFragModeScoringField).add("All Kills", ScoringStyle::AllKills);
        g.ctl(this.miFragModeScoringField).add_random("Push Kills Only", ScoringStyle::PushOnly, false);
        g.ctl(this.miFragModeScoringField).set_output_ptr(&mut g.game_values.gamemodemenusettings.frag.scoring as *mut _);
        g.ctl(this.miFragModeScoringField).set_current_value(g.game_values.gamemodemenusettings.frag.scoring);

        this.miFragModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miFragModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miFragModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miFragModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miFragModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Frag Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[1].add_control(g, this.miFragModeStyleField.id(), this.miFragModeBackButton.id(), this.miFragModeScoringField.id(), Default::default(), this.miFragModeBackButton.id());
        this.mModeSettingsMenu[1].add_control(g, this.miFragModeScoringField.id(), this.miFragModeStyleField.id(), this.miFragModeBackButton.id(), Default::default(), this.miFragModeBackButton.id());
        this.mModeSettingsMenu[1].add_control(g, this.miFragModeBackButton.id(), this.miFragModeScoringField.id(), this.miFragModeStyleField.id(), this.miFragModeScoringField.id(), Default::default());

        this.mModeSettingsMenu[1].add_non_control(this.miFragModeLeftHeaderBar.id());
        this.mModeSettingsMenu[1].add_non_control(this.miFragModeRightHeaderBar.id());
        this.mModeSettingsMenu[1].add_non_control(this.miFragModeHeaderText.id());

        this.mModeSettingsMenu[1].set_initial_focus(g, this.miFragModeStyleField.id());
        this.mModeSettingsMenu[1].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Time Limit Mode Settings
        //***********************

        this.miTimeLimitModeStyleField = Ctl::new(MI_SelectField::<DeathStyle>::new(g.rm.spr_selectfield, 120, 180, "On Kill", 400, 150), g);
        g.ctl(this.miTimeLimitModeStyleField).add("Respawn", DeathStyle::Respawn);
        g.ctl(this.miTimeLimitModeStyleField).add("Shield", DeathStyle::Shield);
        g.ctl(this.miTimeLimitModeStyleField).set_output_ptr(&mut g.game_values.gamemodemenusettings.time.style as *mut _);
        g.ctl(this.miTimeLimitModeStyleField).set_current_value(g.game_values.gamemodemenusettings.time.style);

        this.miTimeLimitModeScoringField = Ctl::new(MI_SelectField::<ScoringStyle>::new(g.rm.spr_selectfield, 120, 220, "Scoring", 400, 150), g);
        g.ctl(this.miTimeLimitModeScoringField).add("All Kills", ScoringStyle::AllKills);
        g.ctl(this.miTimeLimitModeScoringField).add_random("Push Kills Only", ScoringStyle::PushOnly, false);
        g.ctl(this.miTimeLimitModeScoringField).set_output_ptr(&mut g.game_values.gamemodemenusettings.time.scoring as *mut _);
        g.ctl(this.miTimeLimitModeScoringField).set_current_value(g.game_values.gamemodemenusettings.time.scoring);

        this.miTimeLimitModePercentExtraTime = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 260, "Extra Time", 400, 150, 384), g);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("0", 0, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("5", 5, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("10", 10);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("15", 15);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("20", 20);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("25", 25);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("30", 30);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("35", 35);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("40", 40);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("45", 45);
        g.ctl(this.miTimeLimitModePercentExtraTime).add("50", 50);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("55", 55, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("60", 60, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("65", 65, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("70", 70, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("75", 75, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("80", 80, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("85", 85, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("90", 90, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("95", 95, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).add_random("100", 100, false);
        g.ctl(this.miTimeLimitModePercentExtraTime).set_output_ptr(&mut g.game_values.gamemodemenusettings.time.percentextratime as *mut _);
        g.ctl(this.miTimeLimitModePercentExtraTime).set_current_value(g.game_values.gamemodemenusettings.time.percentextratime);
        g.ctl(this.miTimeLimitModePercentExtraTime).allow_wrap(false);

        this.miTimeLimitModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miTimeLimitModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miTimeLimitModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miTimeLimitModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miTimeLimitModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Time Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[2].add_control(g, this.miTimeLimitModeStyleField.id(), this.miTimeLimitModeBackButton.id(), this.miTimeLimitModeScoringField.id(), Default::default(), this.miTimeLimitModeBackButton.id());
        this.mModeSettingsMenu[2].add_control(g, this.miTimeLimitModeScoringField.id(), this.miTimeLimitModeStyleField.id(), this.miTimeLimitModePercentExtraTime.id(), Default::default(), this.miTimeLimitModeBackButton.id());
        this.mModeSettingsMenu[2].add_control(g, this.miTimeLimitModePercentExtraTime.id(), this.miTimeLimitModeScoringField.id(), this.miTimeLimitModeBackButton.id(), Default::default(), this.miTimeLimitModeBackButton.id());
        this.mModeSettingsMenu[2].add_control(g, this.miTimeLimitModeBackButton.id(), this.miTimeLimitModePercentExtraTime.id(), this.miTimeLimitModeStyleField.id(), this.miTimeLimitModePercentExtraTime.id(), Default::default());

        this.mModeSettingsMenu[2].add_non_control(this.miTimeLimitModeLeftHeaderBar.id());
        this.mModeSettingsMenu[2].add_non_control(this.miTimeLimitModeRightHeaderBar.id());
        this.mModeSettingsMenu[2].add_non_control(this.miTimeLimitModeHeaderText.id());

        this.mModeSettingsMenu[2].set_initial_focus(g, this.miTimeLimitModeStyleField.id());
        this.mModeSettingsMenu[2].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Jail Mode Settings
        //***********************

        this.miJailModeStyleField = Ctl::new(MI_SelectField::<JailStyle>::new(g.rm.spr_selectfield, 120, 160, "Style", 400, 150), g);
        g.ctl(this.miJailModeStyleField).add("Classic", JailStyle::Classic);
        g.ctl(this.miJailModeStyleField).add("Owned", JailStyle::Owned);
        g.ctl(this.miJailModeStyleField).add("Free For All", JailStyle::FreeForAll);
        g.ctl(this.miJailModeStyleField).set_output_ptr(&mut g.game_values.gamemodemenusettings.jail.style as *mut _);
        g.ctl(this.miJailModeStyleField).set_current_value(g.game_values.gamemodemenusettings.jail.style);

        this.miJailModeTimeFreeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 200, "Free Timer", 400, 150), g);
        g.ctl(this.miJailModeTimeFreeField).add("None", 1);
        g.ctl(this.miJailModeTimeFreeField).add_random("5 Seconds", 300, false);
        g.ctl(this.miJailModeTimeFreeField).add("10 Seconds", 600);
        g.ctl(this.miJailModeTimeFreeField).add("15 Seconds", 900);
        g.ctl(this.miJailModeTimeFreeField).add("20 Seconds", 1200);
        g.ctl(this.miJailModeTimeFreeField).add("25 Seconds", 1500);
        g.ctl(this.miJailModeTimeFreeField).add("30 Seconds", 1800);
        g.ctl(this.miJailModeTimeFreeField).add("35 Seconds", 2100);
        g.ctl(this.miJailModeTimeFreeField).add("40 Seconds", 2400);
        g.ctl(this.miJailModeTimeFreeField).add_random("45 Seconds", 2700, false);
        g.ctl(this.miJailModeTimeFreeField).add_random("50 Seconds", 3000, false);
        g.ctl(this.miJailModeTimeFreeField).add_random("55 Seconds", 3300, false);
        g.ctl(this.miJailModeTimeFreeField).add_random("60 Seconds", 3600, false);
        g.ctl(this.miJailModeTimeFreeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.jail.timetofree as *mut _);
        g.ctl(this.miJailModeTimeFreeField).set_current_value(g.game_values.gamemodemenusettings.jail.timetofree);

        this.miJailModeTagFreeField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 240, "Tag Free", 400, 150), g);
        g.ctl(this.miJailModeTagFreeField).add("Off", false);
        g.ctl(this.miJailModeTagFreeField).add("On", true);
        g.ctl(this.miJailModeTagFreeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.jail.tagfree as *mut _);
        g.ctl(this.miJailModeTagFreeField).set_current_value(g.game_values.gamemodemenusettings.jail.tagfree);
        g.ctl(this.miJailModeTagFreeField).set_auto_advance(true);

        this.miJailModeJailKeyField = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 280, "Jail Key", 400, 150, 384), g);
        g.ctl(this.miJailModeJailKeyField).add_random("0", 0, false);
        g.ctl(this.miJailModeJailKeyField).add_random("5", 5, false);
        g.ctl(this.miJailModeJailKeyField).add("10", 10);
        g.ctl(this.miJailModeJailKeyField).add("15", 15);
        g.ctl(this.miJailModeJailKeyField).add("20", 20);
        g.ctl(this.miJailModeJailKeyField).add("25", 25);
        g.ctl(this.miJailModeJailKeyField).add("30", 30);
        g.ctl(this.miJailModeJailKeyField).add("35", 35);
        g.ctl(this.miJailModeJailKeyField).add("40", 40);
        g.ctl(this.miJailModeJailKeyField).add("45", 45);
        g.ctl(this.miJailModeJailKeyField).add("50", 50);
        g.ctl(this.miJailModeJailKeyField).add_random("55", 55, false);
        g.ctl(this.miJailModeJailKeyField).add_random("60", 60, false);
        g.ctl(this.miJailModeJailKeyField).add_random("65", 65, false);
        g.ctl(this.miJailModeJailKeyField).add_random("70", 70, false);
        g.ctl(this.miJailModeJailKeyField).add_random("75", 75, false);
        g.ctl(this.miJailModeJailKeyField).add_random("80", 80, false);
        g.ctl(this.miJailModeJailKeyField).add_random("85", 85, false);
        g.ctl(this.miJailModeJailKeyField).add_random("90", 90, false);
        g.ctl(this.miJailModeJailKeyField).add_random("95", 95, false);
        g.ctl(this.miJailModeJailKeyField).add_random("100", 100, false);
        g.ctl(this.miJailModeJailKeyField).set_output_ptr(&mut g.game_values.gamemodemenusettings.jail.percentkey as *mut _);
        g.ctl(this.miJailModeJailKeyField).set_current_value(g.game_values.gamemodemenusettings.jail.percentkey);
        g.ctl(this.miJailModeJailKeyField).allow_wrap(false);

        this.miJailModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miJailModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miJailModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miJailModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miJailModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Jail Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[3].add_control(g, this.miJailModeStyleField.id(), this.miJailModeBackButton.id(), this.miJailModeTimeFreeField.id(), Default::default(), this.miJailModeBackButton.id());
        this.mModeSettingsMenu[3].add_control(g, this.miJailModeTimeFreeField.id(), this.miJailModeStyleField.id(), this.miJailModeTagFreeField.id(), Default::default(), this.miJailModeBackButton.id());
        this.mModeSettingsMenu[3].add_control(g, this.miJailModeTagFreeField.id(), this.miJailModeTimeFreeField.id(), this.miJailModeJailKeyField.id(), Default::default(), this.miJailModeBackButton.id());
        this.mModeSettingsMenu[3].add_control(g, this.miJailModeJailKeyField.id(), this.miJailModeTagFreeField.id(), this.miJailModeBackButton.id(), Default::default(), this.miJailModeBackButton.id());
        this.mModeSettingsMenu[3].add_control(g, this.miJailModeBackButton.id(), this.miJailModeJailKeyField.id(), this.miJailModeStyleField.id(), this.miJailModeJailKeyField.id(), Default::default());

        this.mModeSettingsMenu[3].add_non_control(this.miJailModeLeftHeaderBar.id());
        this.mModeSettingsMenu[3].add_non_control(this.miJailModeRightHeaderBar.id());
        this.mModeSettingsMenu[3].add_non_control(this.miJailModeHeaderText.id());

        this.mModeSettingsMenu[3].set_initial_focus(g, this.miJailModeStyleField.id());
        this.mModeSettingsMenu[3].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Coins Mode Settings
        //***********************

        this.miCoinModePenaltyField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 180, "Penalty", 400, 150), g);
        g.ctl(this.miCoinModePenaltyField).add("Off", false);
        g.ctl(this.miCoinModePenaltyField).add("On", true);
        g.ctl(this.miCoinModePenaltyField).set_output_ptr(&mut g.game_values.gamemodemenusettings.coins.penalty as *mut _);
        g.ctl(this.miCoinModePenaltyField).set_current_value(g.game_values.gamemodemenusettings.coins.penalty);
        g.ctl(this.miCoinModePenaltyField).set_auto_advance(true);

        this.miCoinModeQuantityField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 220, "Quantity", 400, 150), g);
        g.ctl(this.miCoinModeQuantityField).add("1", 1);
        g.ctl(this.miCoinModeQuantityField).add("2", 2);
        g.ctl(this.miCoinModeQuantityField).add("3", 3);
        g.ctl(this.miCoinModeQuantityField).add("4", 4);
        g.ctl(this.miCoinModeQuantityField).add("5", 5);
        g.ctl(this.miCoinModeQuantityField).add("6", 6);
        g.ctl(this.miCoinModeQuantityField).add("7", 7);
        g.ctl(this.miCoinModeQuantityField).add("8", 8);
        g.ctl(this.miCoinModeQuantityField).add("9", 9);
        g.ctl(this.miCoinModeQuantityField).add("10", 10);
        g.ctl(this.miCoinModeQuantityField).set_output_ptr(&mut g.game_values.gamemodemenusettings.coins.quantity as *mut _);
        g.ctl(this.miCoinModeQuantityField).set_current_value(g.game_values.gamemodemenusettings.coins.quantity);

        this.miCoinModePercentExtraCoin = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 260, "Extra Coins", 400, 150, 384), g);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("0", 0, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("5", 5, false);
        g.ctl(this.miCoinModePercentExtraCoin).add("10", 10);
        g.ctl(this.miCoinModePercentExtraCoin).add("15", 15);
        g.ctl(this.miCoinModePercentExtraCoin).add("20", 20);
        g.ctl(this.miCoinModePercentExtraCoin).add("25", 25);
        g.ctl(this.miCoinModePercentExtraCoin).add("30", 30);
        g.ctl(this.miCoinModePercentExtraCoin).add("35", 35);
        g.ctl(this.miCoinModePercentExtraCoin).add("40", 40);
        g.ctl(this.miCoinModePercentExtraCoin).add("45", 45);
        g.ctl(this.miCoinModePercentExtraCoin).add("50", 50);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("55", 55, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("60", 60, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("65", 65, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("70", 70, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("75", 75, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("80", 80, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("85", 85, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("90", 90, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("95", 95, false);
        g.ctl(this.miCoinModePercentExtraCoin).add_random("100", 100, false);
        g.ctl(this.miCoinModePercentExtraCoin).set_output_ptr(&mut g.game_values.gamemodemenusettings.coins.percentextracoin as *mut _);
        g.ctl(this.miCoinModePercentExtraCoin).set_current_value(g.game_values.gamemodemenusettings.coins.percentextracoin);
        g.ctl(this.miCoinModePercentExtraCoin).allow_wrap(false);

        this.miCoinModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miCoinModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miCoinModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miCoinModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miCoinModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Coin Collection Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[4].add_control(g, this.miCoinModePenaltyField.id(), this.miCoinModeBackButton.id(), this.miCoinModeQuantityField.id(), Default::default(), this.miCoinModeBackButton.id());
        this.mModeSettingsMenu[4].add_control(g, this.miCoinModeQuantityField.id(), this.miCoinModePenaltyField.id(), this.miCoinModePercentExtraCoin.id(), Default::default(), this.miCoinModeBackButton.id());
        this.mModeSettingsMenu[4].add_control(g, this.miCoinModePercentExtraCoin.id(), this.miCoinModeQuantityField.id(), this.miCoinModeBackButton.id(), Default::default(), this.miCoinModeBackButton.id());
        this.mModeSettingsMenu[4].add_control(g, this.miCoinModeBackButton.id(), this.miCoinModePercentExtraCoin.id(), this.miCoinModePenaltyField.id(), this.miCoinModePercentExtraCoin.id(), Default::default());

        this.mModeSettingsMenu[4].add_non_control(this.miCoinModeLeftHeaderBar.id());
        this.mModeSettingsMenu[4].add_non_control(this.miCoinModeRightHeaderBar.id());
        this.mModeSettingsMenu[4].add_non_control(this.miCoinModeHeaderText.id());

        this.mModeSettingsMenu[4].set_initial_focus(g, this.miCoinModePenaltyField.id());
        this.mModeSettingsMenu[4].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Stomp Mode Settings
        //***********************

        this.miStompModeRateField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 40, "Rate", 400, 180), g);
        g.ctl(this.miStompModeRateField).add_random("Very Slow", 150, false);
        g.ctl(this.miStompModeRateField).add("Slow", 120);
        g.ctl(this.miStompModeRateField).add("Moderate", 90);
        g.ctl(this.miStompModeRateField).add("Fast", 60);
        g.ctl(this.miStompModeRateField).add_random("Very Fast", 30, false);
        g.ctl(this.miStompModeRateField).add_random("Extremely Fast", 15, false);
        g.ctl(this.miStompModeRateField).add_random("Insanely Fast", 5, false);
        g.ctl(this.miStompModeRateField).set_output_ptr(&mut g.game_values.gamemodemenusettings.stomp.rate as *mut _);
        g.ctl(this.miStompModeRateField).set_current_value(g.game_values.gamemodemenusettings.stomp.rate);

        for iEnemy in 0..NUMSTOMPENEMIES as i16 {
            this.miStompModeEnemySlider[iEnemy as usize] = Ctl::new(MI_PowerupSlider::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, g.rm.menu_stomp, 120, 80 + 40 * iEnemy, 400, iEnemy), g);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 0);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 1);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 2);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 3);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 4);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 5);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 6);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 7);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 8);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 9);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).add("", 10);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).allow_wrap(false);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).set_output_ptr(&mut g.game_values.gamemodemenusettings.stomp.enemyweight[iEnemy as usize] as *mut _);
            g.ctl(this.miStompModeEnemySlider[iEnemy as usize]).set_current_value(g.game_values.gamemodemenusettings.stomp.enemyweight[iEnemy as usize]);
        }

        this.miStompModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miStompModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miStompModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miStompModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miStompModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Stomp Mode Menu", 320, 5), g);


        this.mModeSettingsMenu[5].add_control(g, this.miStompModeRateField.id(), this.miStompModeBackButton.id(), this.miStompModeEnemySlider[0].id(), Default::default(), this.miStompModeBackButton.id());

        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[0].id(), this.miStompModeRateField.id(), this.miStompModeEnemySlider[1].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[1].id(), this.miStompModeEnemySlider[0].id(), this.miStompModeEnemySlider[2].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[2].id(), this.miStompModeEnemySlider[1].id(), this.miStompModeEnemySlider[3].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[3].id(), this.miStompModeEnemySlider[2].id(), this.miStompModeEnemySlider[4].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[4].id(), this.miStompModeEnemySlider[3].id(), this.miStompModeEnemySlider[5].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[5].id(), this.miStompModeEnemySlider[4].id(), this.miStompModeEnemySlider[6].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[6].id(), this.miStompModeEnemySlider[5].id(), this.miStompModeEnemySlider[7].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[7].id(), this.miStompModeEnemySlider[6].id(), this.miStompModeEnemySlider[8].id(), Default::default(), this.miStompModeBackButton.id());
        this.mModeSettingsMenu[5].add_control(g, this.miStompModeEnemySlider[8].id(), this.miStompModeEnemySlider[7].id(), this.miStompModeBackButton.id(), Default::default(), this.miStompModeBackButton.id());

        this.mModeSettingsMenu[5].add_control(g, this.miStompModeBackButton.id(), this.miStompModeEnemySlider[8].id(), this.miStompModeRateField.id(), this.miStompModeEnemySlider[8].id(), Default::default());

        this.mModeSettingsMenu[5].add_non_control(this.miStompModeLeftHeaderBar.id());
        this.mModeSettingsMenu[5].add_non_control(this.miStompModeRightHeaderBar.id());
        this.mModeSettingsMenu[5].add_non_control(this.miStompModeHeaderText.id());

        this.mModeSettingsMenu[5].set_initial_focus(g, this.miStompModeRateField.id());
        this.mModeSettingsMenu[5].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Yoshi's Eggs Mode Settings
        //***********************

        for iEggField in 0..4 {
            this.miEggModeEggQuantityField[iEggField as usize] = Ctl::new(MI_PowerupSlider::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, g.rm.menu_egg, 170, 60 + 40 * iEggField, 300, iEggField), g);
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).add_random("0", 0, !(iEggField == 0));
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).add_random("1", 1, !(iEggField >= 2));
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).add_random("2", 2, !(iEggField >= 2));
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).add_random("3", 3, !(iEggField >= 1));
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).add("4", 4);
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).set_output_ptr(&mut g.game_values.gamemodemenusettings.egg.eggs[iEggField as usize] as *mut _);
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).set_current_value(g.game_values.gamemodemenusettings.egg.eggs[iEggField as usize]);
            g.ctl(this.miEggModeEggQuantityField[iEggField as usize]).allow_wrap(false);
        }

        for iYoshiField in 0..4 {
            this.miEggModeYoshiQuantityField[iYoshiField as usize] = Ctl::new(MI_PowerupSlider::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, g.rm.menu_egg, 170, 220 + 40 * iYoshiField, 300, iYoshiField + 4), g);
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).add_random("0", 0, !(iYoshiField == 0));
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).add_random("1", 1, !(iYoshiField >= 2));
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).add_random("2", 2, !(iYoshiField >= 1));
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).add("3", 3);
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).add("4", 4);
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).set_output_ptr(&mut g.game_values.gamemodemenusettings.egg.yoshis[iYoshiField as usize] as *mut _);
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).set_current_value(g.game_values.gamemodemenusettings.egg.yoshis[iYoshiField as usize]);
            g.ctl(this.miEggModeYoshiQuantityField[iYoshiField as usize]).allow_wrap(false);
        }

        this.miEggModeExplosionTimeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 380, "Explosion Timer", 400, 220), g);
        g.ctl(this.miEggModeExplosionTimeField).add("Off", 0);
        g.ctl(this.miEggModeExplosionTimeField).add_random("3 Seconds", 3, false);
        g.ctl(this.miEggModeExplosionTimeField).add("5 Seconds", 5);
        g.ctl(this.miEggModeExplosionTimeField).add("8 Seconds", 8);
        g.ctl(this.miEggModeExplosionTimeField).add("10 Seconds", 10);
        g.ctl(this.miEggModeExplosionTimeField).add("15 Seconds", 15);
        g.ctl(this.miEggModeExplosionTimeField).add("20 Seconds", 20);
        g.ctl(this.miEggModeExplosionTimeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.egg.explode as *mut _);
        g.ctl(this.miEggModeExplosionTimeField).set_current_value(g.game_values.gamemodemenusettings.egg.explode);

        this.miEggModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miEggModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miEggModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miEggModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miEggModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Yoshi's Eggs Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[6].add_control(g, this.miEggModeEggQuantityField[0].id(), this.miEggModeBackButton.id(), this.miEggModeEggQuantityField[1].id(), Default::default(), this.miEggModeBackButton.id());
        this.mModeSettingsMenu[6].add_control(g, this.miEggModeEggQuantityField[1].id(), this.miEggModeEggQuantityField[0].id(), this.miEggModeEggQuantityField[2].id(), Default::default(), this.miEggModeBackButton.id());
        this.mModeSettingsMenu[6].add_control(g, this.miEggModeEggQuantityField[2].id(), this.miEggModeEggQuantityField[1].id(), this.miEggModeEggQuantityField[3].id(), Default::default(), this.miEggModeBackButton.id());
        this.mModeSettingsMenu[6].add_control(g, this.miEggModeEggQuantityField[3].id(), this.miEggModeEggQuantityField[2].id(), this.miEggModeYoshiQuantityField[0].id(), Default::default(), this.miEggModeBackButton.id());

        this.mModeSettingsMenu[6].add_control(g, this.miEggModeYoshiQuantityField[0].id(), this.miEggModeEggQuantityField[3].id(), this.miEggModeYoshiQuantityField[1].id(), Default::default(), this.miEggModeBackButton.id());
        this.mModeSettingsMenu[6].add_control(g, this.miEggModeYoshiQuantityField[1].id(), this.miEggModeYoshiQuantityField[0].id(), this.miEggModeYoshiQuantityField[2].id(), Default::default(), this.miEggModeBackButton.id());
        this.mModeSettingsMenu[6].add_control(g, this.miEggModeYoshiQuantityField[2].id(), this.miEggModeYoshiQuantityField[1].id(), this.miEggModeYoshiQuantityField[3].id(), Default::default(), this.miEggModeBackButton.id());
        this.mModeSettingsMenu[6].add_control(g, this.miEggModeYoshiQuantityField[3].id(), this.miEggModeYoshiQuantityField[2].id(), this.miEggModeExplosionTimeField.id(), Default::default(), this.miEggModeBackButton.id());

        this.mModeSettingsMenu[6].add_control(g, this.miEggModeExplosionTimeField.id(), this.miEggModeYoshiQuantityField[3].id(), this.miEggModeBackButton.id(), Default::default(), this.miEggModeBackButton.id());

        this.mModeSettingsMenu[6].add_control(g, this.miEggModeBackButton.id(), this.miEggModeExplosionTimeField.id(), this.miEggModeEggQuantityField[0].id(), this.miEggModeExplosionTimeField.id(), Default::default());

        this.mModeSettingsMenu[6].add_non_control(this.miEggModeLeftHeaderBar.id());
        this.mModeSettingsMenu[6].add_non_control(this.miEggModeRightHeaderBar.id());
        this.mModeSettingsMenu[6].add_non_control(this.miEggModeHeaderText.id());

        this.mModeSettingsMenu[6].set_initial_focus(g, this.miEggModeEggQuantityField[0].id());
        this.mModeSettingsMenu[6].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        //***********************
        // Flag Mode Settings
        //***********************

        this.miFlagModeSpeedField = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 120, "Speed", 400, 180, 380), g);
        g.ctl(this.miFlagModeSpeedField).add("-", -1);
        g.ctl(this.miFlagModeSpeedField).add("0", 0);
        g.ctl(this.miFlagModeSpeedField).add("1", 1);
        g.ctl(this.miFlagModeSpeedField).add("2", 2);
        g.ctl(this.miFlagModeSpeedField).add("3", 3);
        g.ctl(this.miFlagModeSpeedField).add("4", 4);
        g.ctl(this.miFlagModeSpeedField).add("5", 5);
        g.ctl(this.miFlagModeSpeedField).add_random("6", 6, false);
        g.ctl(this.miFlagModeSpeedField).add_random("7", 7, false);
        g.ctl(this.miFlagModeSpeedField).add_random("8", 8, false);
        g.ctl(this.miFlagModeSpeedField).set_output_ptr(&mut g.game_values.gamemodemenusettings.flag.speed as *mut _);
        g.ctl(this.miFlagModeSpeedField).set_current_value(g.game_values.gamemodemenusettings.flag.speed);
        g.ctl(this.miFlagModeSpeedField).allow_wrap(false);

        this.miFlagModeTouchReturnField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 160, "Touch Return", 400, 180), g);
        g.ctl(this.miFlagModeTouchReturnField).add("Off", false);
        g.ctl(this.miFlagModeTouchReturnField).add("On", true);
        g.ctl(this.miFlagModeTouchReturnField).set_output_ptr(&mut g.game_values.gamemodemenusettings.flag.touchreturn as *mut _);
        g.ctl(this.miFlagModeTouchReturnField).set_current_value(g.game_values.gamemodemenusettings.flag.touchreturn);
        g.ctl(this.miFlagModeTouchReturnField).set_auto_advance(true);

        this.miFlagModePointMoveField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 200, "Point Move", 400, 180), g);
        g.ctl(this.miFlagModePointMoveField).add("Off", false);
        g.ctl(this.miFlagModePointMoveField).add("On", true);
        g.ctl(this.miFlagModePointMoveField).set_output_ptr(&mut g.game_values.gamemodemenusettings.flag.pointmove as *mut _);
        g.ctl(this.miFlagModePointMoveField).set_current_value(g.game_values.gamemodemenusettings.flag.pointmove);
        g.ctl(this.miFlagModePointMoveField).set_auto_advance(true);

        this.miFlagModeAutoReturnField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 240, "Auto Return", 400, 180), g);
        g.ctl(this.miFlagModeAutoReturnField).add("None", 0);
        g.ctl(this.miFlagModeAutoReturnField).add_random("5 Seconds", 300, false);
        g.ctl(this.miFlagModeAutoReturnField).add("10 Seconds", 600);
        g.ctl(this.miFlagModeAutoReturnField).add_random("15 Seconds", 900, false);
        g.ctl(this.miFlagModeAutoReturnField).add("20 Seconds", 1200);
        g.ctl(this.miFlagModeAutoReturnField).add_random("25 Seconds", 1500, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("30 Seconds", 1800, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("35 Seconds", 2100, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("40 Seconds", 2400, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("45 Seconds", 2700, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("50 Seconds", 3000, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("55 Seconds", 3300, false);
        g.ctl(this.miFlagModeAutoReturnField).add_random("60 Seconds", 3600, false);
        g.ctl(this.miFlagModeAutoReturnField).set_output_ptr(&mut g.game_values.gamemodemenusettings.flag.autoreturn as *mut _);
        g.ctl(this.miFlagModeAutoReturnField).set_current_value(g.game_values.gamemodemenusettings.flag.autoreturn);

        this.miFlagModeHomeScoreField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 280, "Need Home", 400, 180), g);
        g.ctl(this.miFlagModeHomeScoreField).add("Off", false);
        g.ctl(this.miFlagModeHomeScoreField).add("On", true);
        g.ctl(this.miFlagModeHomeScoreField).set_output_ptr(&mut g.game_values.gamemodemenusettings.flag.homescore as *mut _);
        g.ctl(this.miFlagModeHomeScoreField).set_current_value(g.game_values.gamemodemenusettings.flag.homescore);
        g.ctl(this.miFlagModeHomeScoreField).set_auto_advance(true);

        this.miFlagModeCenterFlagField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 320, "Center Flag", 400, 180), g);
        g.ctl(this.miFlagModeCenterFlagField).add("Off", false);
        g.ctl(this.miFlagModeCenterFlagField).add("On", true);
        g.ctl(this.miFlagModeCenterFlagField).set_output_ptr(&mut g.game_values.gamemodemenusettings.flag.centerflag as *mut _);
        g.ctl(this.miFlagModeCenterFlagField).set_current_value(g.game_values.gamemodemenusettings.flag.centerflag);
        g.ctl(this.miFlagModeCenterFlagField).set_auto_advance(true);

        this.miFlagModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miFlagModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miFlagModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miFlagModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miFlagModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Capture The Flag Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[7].add_control(g, this.miFlagModeSpeedField.id(), this.miFlagModeBackButton.id(), this.miFlagModeTouchReturnField.id(), Default::default(), this.miFlagModeBackButton.id());
        this.mModeSettingsMenu[7].add_control(g, this.miFlagModeTouchReturnField.id(), this.miFlagModeSpeedField.id(), this.miFlagModePointMoveField.id(), Default::default(), this.miFlagModeBackButton.id());
        this.mModeSettingsMenu[7].add_control(g, this.miFlagModePointMoveField.id(), this.miFlagModeTouchReturnField.id(), this.miFlagModeAutoReturnField.id(), Default::default(), this.miFlagModeBackButton.id());
        this.mModeSettingsMenu[7].add_control(g, this.miFlagModeAutoReturnField.id(), this.miFlagModePointMoveField.id(), this.miFlagModeHomeScoreField.id(), Default::default(), this.miFlagModeBackButton.id());
        this.mModeSettingsMenu[7].add_control(g, this.miFlagModeHomeScoreField.id(), this.miFlagModeAutoReturnField.id(), this.miFlagModeCenterFlagField.id(), Default::default(), this.miFlagModeBackButton.id());
        this.mModeSettingsMenu[7].add_control(g, this.miFlagModeCenterFlagField.id(), this.miFlagModeHomeScoreField.id(), this.miFlagModeBackButton.id(), Default::default(), this.miFlagModeBackButton.id());

        this.mModeSettingsMenu[7].add_control(g, this.miFlagModeBackButton.id(), this.miFlagModeCenterFlagField.id(), this.miFlagModeSpeedField.id(), this.miFlagModeCenterFlagField.id(), Default::default());

        this.mModeSettingsMenu[7].add_non_control(this.miFlagModeLeftHeaderBar.id());
        this.mModeSettingsMenu[7].add_non_control(this.miFlagModeRightHeaderBar.id());
        this.mModeSettingsMenu[7].add_non_control(this.miFlagModeHeaderText.id());

        this.mModeSettingsMenu[7].set_initial_focus(g, this.miFlagModeSpeedField.id());
        this.mModeSettingsMenu[7].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Chicken Mode Settings
        //***********************

        this.miChickenModeShowTargetField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 200, "Show Target", 400, 180), g);
        g.ctl(this.miChickenModeShowTargetField).add("Off", false);
        g.ctl(this.miChickenModeShowTargetField).add("On", true);
        g.ctl(this.miChickenModeShowTargetField).set_output_ptr(&mut g.game_values.gamemodemenusettings.chicken.usetarget as *mut _);
        g.ctl(this.miChickenModeShowTargetField).set_current_value(g.game_values.gamemodemenusettings.chicken.usetarget);
        g.ctl(this.miChickenModeShowTargetField).set_auto_advance(true);

        this.miChickenModeGlideField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 240, "Chicken Glide", 400, 180), g);
        g.ctl(this.miChickenModeGlideField).add("Off", false);
        g.ctl(this.miChickenModeGlideField).add("On", true);
        g.ctl(this.miChickenModeGlideField).set_output_ptr(&mut g.game_values.gamemodemenusettings.chicken.glide as *mut _);
        g.ctl(this.miChickenModeGlideField).set_current_value(g.game_values.gamemodemenusettings.chicken.glide);
        g.ctl(this.miChickenModeGlideField).set_auto_advance(true);

        this.miChickenModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miChickenModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miChickenModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miChickenModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miChickenModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Chicken Mode Menu", 320, 5), g);


        this.mModeSettingsMenu[8].add_control(g, this.miChickenModeShowTargetField.id(), this.miChickenModeBackButton.id(), this.miChickenModeGlideField.id(), Default::default(), this.miChickenModeBackButton.id());
        this.mModeSettingsMenu[8].add_control(g, this.miChickenModeGlideField.id(), this.miChickenModeShowTargetField.id(), this.miChickenModeBackButton.id(), Default::default(), this.miChickenModeBackButton.id());
        this.mModeSettingsMenu[8].add_control(g, this.miChickenModeBackButton.id(), this.miChickenModeGlideField.id(), this.miChickenModeShowTargetField.id(), this.miChickenModeGlideField.id(), Default::default());

        this.mModeSettingsMenu[8].add_non_control(this.miChickenModeLeftHeaderBar.id());
        this.mModeSettingsMenu[8].add_non_control(this.miChickenModeRightHeaderBar.id());
        this.mModeSettingsMenu[8].add_non_control(this.miChickenModeHeaderText.id());

        this.mModeSettingsMenu[8].set_initial_focus(g, this.miChickenModeShowTargetField.id());
        this.mModeSettingsMenu[8].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Tag Mode Settings
        //***********************

        this.miTagModeTagOnTouchField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 220, "Touch Tag", 400, 180), g);
        g.ctl(this.miTagModeTagOnTouchField).add("Off", false);
        g.ctl(this.miTagModeTagOnTouchField).add("On", true);
        g.ctl(this.miTagModeTagOnTouchField).set_output_ptr(&mut g.game_values.gamemodemenusettings.tag.tagontouch as *mut _);
        g.ctl(this.miTagModeTagOnTouchField).set_current_value(g.game_values.gamemodemenusettings.tag.tagontouch);
        g.ctl(this.miTagModeTagOnTouchField).set_auto_advance(true);

        this.miTagModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miTagModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miTagModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miTagModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miTagModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Tag Mode Menu", 320, 5), g);


        this.mModeSettingsMenu[9].add_control(g, this.miTagModeTagOnTouchField.id(), this.miTagModeBackButton.id(), this.miTagModeBackButton.id(), Default::default(), this.miTagModeBackButton.id());
        this.mModeSettingsMenu[9].add_control(g, this.miTagModeBackButton.id(), this.miTagModeTagOnTouchField.id(), this.miTagModeTagOnTouchField.id(), this.miTagModeTagOnTouchField.id(), Default::default());

        this.mModeSettingsMenu[9].add_non_control(this.miTagModeLeftHeaderBar.id());
        this.mModeSettingsMenu[9].add_non_control(this.miTagModeRightHeaderBar.id());
        this.mModeSettingsMenu[9].add_non_control(this.miTagModeHeaderText.id());

        this.mModeSettingsMenu[9].set_initial_focus(g, this.miTagModeTagOnTouchField.id());
        this.mModeSettingsMenu[9].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Star Mode Settings
        //***********************

        this.miStarModeTimeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 180, "Time", 400, 150), g);
        g.ctl(this.miStarModeTimeField).add_random("5 Seconds", 5, false);
        g.ctl(this.miStarModeTimeField).add_random("10 Seconds", 10, false);
        g.ctl(this.miStarModeTimeField).add("15 Seconds", 15);
        g.ctl(this.miStarModeTimeField).add("20 Seconds", 20);
        g.ctl(this.miStarModeTimeField).add("25 Seconds", 25);
        g.ctl(this.miStarModeTimeField).add("30 Seconds", 30);
        g.ctl(this.miStarModeTimeField).add("35 Seconds", 35);
        g.ctl(this.miStarModeTimeField).add("40 Seconds", 40);
        g.ctl(this.miStarModeTimeField).add_random("45 Seconds", 45, false);
        g.ctl(this.miStarModeTimeField).add_random("50 Seconds", 50, false);
        g.ctl(this.miStarModeTimeField).add_random("55 Seconds", 55, false);
        g.ctl(this.miStarModeTimeField).add_random("60 Seconds", 60, false);
        g.ctl(this.miStarModeTimeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.star.time as *mut _);
        g.ctl(this.miStarModeTimeField).set_current_value(g.game_values.gamemodemenusettings.star.time);

        this.miStarModeShineField = Ctl::new(MI_SelectField::<StarStyle>::new(g.rm.spr_selectfield, 120, 220, "Star Type", 400, 150), g);
        g.ctl(this.miStarModeShineField).add("Ztar", StarStyle::Ztar);
        g.ctl(this.miStarModeShineField).add("Shine", StarStyle::Shine);
        g.ctl(this.miStarModeShineField).add("Multi Star", StarStyle::Multi);
        g.ctl(this.miStarModeShineField).add("Random", StarStyle::Random);
        g.ctl(this.miStarModeShineField).set_output_ptr(&mut g.game_values.gamemodemenusettings.star.shine as *mut _);
        g.ctl(this.miStarModeShineField).set_current_value(g.game_values.gamemodemenusettings.star.shine);

        this.miStarModePercentExtraTime = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 260, "Extra Time", 400, 150, 384), g);
        g.ctl(this.miStarModePercentExtraTime).add_random("0", 0, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("5", 5, false);
        g.ctl(this.miStarModePercentExtraTime).add("10", 10);
        g.ctl(this.miStarModePercentExtraTime).add("15", 15);
        g.ctl(this.miStarModePercentExtraTime).add("20", 20);
        g.ctl(this.miStarModePercentExtraTime).add("25", 25);
        g.ctl(this.miStarModePercentExtraTime).add("30", 30);
        g.ctl(this.miStarModePercentExtraTime).add("35", 35);
        g.ctl(this.miStarModePercentExtraTime).add("40", 40);
        g.ctl(this.miStarModePercentExtraTime).add("45", 45);
        g.ctl(this.miStarModePercentExtraTime).add("50", 50);
        g.ctl(this.miStarModePercentExtraTime).add_random("55", 55, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("60", 60, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("65", 65, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("70", 70, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("75", 75, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("80", 80, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("85", 85, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("90", 90, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("95", 95, false);
        g.ctl(this.miStarModePercentExtraTime).add_random("100", 100, false);
        g.ctl(this.miStarModePercentExtraTime).set_output_ptr(&mut g.game_values.gamemodemenusettings.star.percentextratime as *mut _);
        g.ctl(this.miStarModePercentExtraTime).set_current_value(g.game_values.gamemodemenusettings.star.percentextratime);
        g.ctl(this.miStarModePercentExtraTime).allow_wrap(false);

        this.miStarModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miStarModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miStarModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miStarModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miStarModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Star Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[10].add_control(g, this.miStarModeTimeField.id(), this.miStarModeBackButton.id(), this.miStarModeShineField.id(), Default::default(), this.miStarModeBackButton.id());
        this.mModeSettingsMenu[10].add_control(g, this.miStarModeShineField.id(), this.miStarModeTimeField.id(), this.miStarModePercentExtraTime.id(), Default::default(), this.miStarModeBackButton.id());
        this.mModeSettingsMenu[10].add_control(g, this.miStarModePercentExtraTime.id(), this.miStarModeShineField.id(), this.miStarModeBackButton.id(), Default::default(), this.miStarModeBackButton.id());
        this.mModeSettingsMenu[10].add_control(g, this.miStarModeBackButton.id(), this.miStarModePercentExtraTime.id(), this.miStarModeTimeField.id(), this.miStarModePercentExtraTime.id(), Default::default());

        this.mModeSettingsMenu[10].add_non_control(this.miStarModeLeftHeaderBar.id());
        this.mModeSettingsMenu[10].add_non_control(this.miStarModeRightHeaderBar.id());
        this.mModeSettingsMenu[10].add_non_control(this.miStarModeHeaderText.id());

        this.mModeSettingsMenu[10].set_initial_focus(g, this.miStarModeTimeField.id());
        this.mModeSettingsMenu[10].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Domination Mode Settings
        //***********************

        this.miDominationModeQuantityField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 120, "Quantity", 400, 180), g);
        g.ctl(this.miDominationModeQuantityField).add_random("1 Base", 1, false);
        g.ctl(this.miDominationModeQuantityField).add_random("2 Bases", 2, false);
        g.ctl(this.miDominationModeQuantityField).add("3 Bases", 3);
        g.ctl(this.miDominationModeQuantityField).add("4 Bases", 4);
        g.ctl(this.miDominationModeQuantityField).add("5 Bases", 5);
        g.ctl(this.miDominationModeQuantityField).add_random("6 Bases", 6, false);
        g.ctl(this.miDominationModeQuantityField).add_random("7 Bases", 7, false);
        g.ctl(this.miDominationModeQuantityField).add_random("8 Bases", 8, false);
        g.ctl(this.miDominationModeQuantityField).add_random("9 Bases", 9, false);
        g.ctl(this.miDominationModeQuantityField).add_random("10 Bases", 10, false);
        g.ctl(this.miDominationModeQuantityField).add_random("# Players - 1", 11, false);
        g.ctl(this.miDominationModeQuantityField).add_random("# Players", 12, false);
        g.ctl(this.miDominationModeQuantityField).add("# Players + 1", 13);
        g.ctl(this.miDominationModeQuantityField).add("# Players + 2", 14);
        g.ctl(this.miDominationModeQuantityField).add("# Players + 3", 15);
        g.ctl(this.miDominationModeQuantityField).add("# Players + 4", 16);
        g.ctl(this.miDominationModeQuantityField).add("# Players + 5", 17);
        g.ctl(this.miDominationModeQuantityField).add_random("# Players + 6", 18, false);
        g.ctl(this.miDominationModeQuantityField).add_random("2x Players - 3", 19, false);
        g.ctl(this.miDominationModeQuantityField).add_random("2x Players - 2", 20, false);
        g.ctl(this.miDominationModeQuantityField).add("2x Players - 1", 21);
        g.ctl(this.miDominationModeQuantityField).add("2x Players", 22);
        g.ctl(this.miDominationModeQuantityField).add_random("2x Players + 1", 23, false);
        g.ctl(this.miDominationModeQuantityField).add_random("2x Players + 2", 24, false);
        g.ctl(this.miDominationModeQuantityField).set_output_ptr(&mut g.game_values.gamemodemenusettings.domination.quantity as *mut _);
        g.ctl(this.miDominationModeQuantityField).set_current_value(g.game_values.gamemodemenusettings.domination.quantity);

        this.miDominationModeRelocateFrequencyField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 160, "Relocate", 400, 180), g);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("Never", 0);
        g.ctl(this.miDominationModeRelocateFrequencyField).add_random("5 Seconds", 300, false);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("10 Seconds", 600);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("15 Seconds", 900);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("20 Seconds", 1200);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("30 Seconds", 1800);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("45 Seconds", 2700);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("1 Minute", 3600);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("1.5 Minutes", 5400);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("2 Minutes", 7200);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("2.5 Minutes", 9000);
        g.ctl(this.miDominationModeRelocateFrequencyField).add("3 Minutes", 10800);
        g.ctl(this.miDominationModeRelocateFrequencyField).set_output_ptr(&mut g.game_values.gamemodemenusettings.domination.relocationfrequency as *mut _);
        g.ctl(this.miDominationModeRelocateFrequencyField).set_current_value(g.game_values.gamemodemenusettings.domination.relocationfrequency);

        this.miDominationModeDeathText = Ctl::new(MI_Text::new(g, "On Death", 120, 210, 0, true, TextAlign::LEFT), g);

        this.miDominationModeLoseOnDeathField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 240, "Lose Bases", 400, 180), g);
        g.ctl(this.miDominationModeLoseOnDeathField).add("Off", false);
        g.ctl(this.miDominationModeLoseOnDeathField).add("On", true);
        g.ctl(this.miDominationModeLoseOnDeathField).set_output_ptr(&mut g.game_values.gamemodemenusettings.domination.loseondeath as *mut _);
        g.ctl(this.miDominationModeLoseOnDeathField).set_current_value(g.game_values.gamemodemenusettings.domination.loseondeath);
        g.ctl(this.miDominationModeLoseOnDeathField).set_auto_advance(true);

        this.miDominationModeRelocateOnDeathField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 280, "Move Bases", 400, 180), g);
        g.ctl(this.miDominationModeRelocateOnDeathField).add("Off", false);
        g.ctl(this.miDominationModeRelocateOnDeathField).add("On", true);
        g.ctl(this.miDominationModeRelocateOnDeathField).set_output_ptr(&mut g.game_values.gamemodemenusettings.domination.relocateondeath as *mut _);
        g.ctl(this.miDominationModeRelocateOnDeathField).set_current_value(g.game_values.gamemodemenusettings.domination.relocateondeath);
        g.ctl(this.miDominationModeRelocateOnDeathField).set_auto_advance(true);

        this.miDominationModeStealOnDeathField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 320, "Steal Bases", 400, 180), g);
        g.ctl(this.miDominationModeStealOnDeathField).add("Off", false);
        g.ctl(this.miDominationModeStealOnDeathField).add("On", true);
        g.ctl(this.miDominationModeStealOnDeathField).set_output_ptr(&mut g.game_values.gamemodemenusettings.domination.stealondeath as *mut _);
        g.ctl(this.miDominationModeStealOnDeathField).set_current_value(g.game_values.gamemodemenusettings.domination.stealondeath);
        g.ctl(this.miDominationModeStealOnDeathField).set_auto_advance(true);


        this.miDominationModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miDominationModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miDominationModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miDominationModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miDominationModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Domination Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[11].add_control(g, this.miDominationModeQuantityField.id(), this.miDominationModeBackButton.id(), this.miDominationModeRelocateFrequencyField.id(), Default::default(), this.miDominationModeBackButton.id());
        this.mModeSettingsMenu[11].add_control(g, this.miDominationModeRelocateFrequencyField.id(), this.miDominationModeQuantityField.id(), this.miDominationModeLoseOnDeathField.id(), Default::default(), this.miDominationModeBackButton.id());
        this.mModeSettingsMenu[11].add_control(g, this.miDominationModeLoseOnDeathField.id(), this.miDominationModeRelocateFrequencyField.id(), this.miDominationModeRelocateOnDeathField.id(), Default::default(), this.miDominationModeBackButton.id());
        this.mModeSettingsMenu[11].add_control(g, this.miDominationModeRelocateOnDeathField.id(), this.miDominationModeLoseOnDeathField.id(), this.miDominationModeStealOnDeathField.id(), Default::default(), this.miDominationModeBackButton.id());
        this.mModeSettingsMenu[11].add_control(g, this.miDominationModeStealOnDeathField.id(), this.miDominationModeRelocateOnDeathField.id(), this.miDominationModeBackButton.id(), Default::default(), this.miDominationModeBackButton.id());

        this.mModeSettingsMenu[11].add_control(g, this.miDominationModeBackButton.id(), this.miDominationModeStealOnDeathField.id(), this.miDominationModeQuantityField.id(), this.miDominationModeStealOnDeathField.id(), Default::default());

        this.mModeSettingsMenu[11].add_non_control(this.miDominationModeLeftHeaderBar.id());
        this.mModeSettingsMenu[11].add_non_control(this.miDominationModeRightHeaderBar.id());
        this.mModeSettingsMenu[11].add_non_control(this.miDominationModeHeaderText.id());
        this.mModeSettingsMenu[11].add_non_control(this.miDominationModeDeathText.id());

        this.mModeSettingsMenu[11].set_initial_focus(g, this.miDominationModeQuantityField.id());
        this.mModeSettingsMenu[11].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // King of the Hill Mode Settings
        //***********************

        this.miKingOfTheHillModeSizeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 180, "Size", 400, 180), g);
        g.ctl(this.miKingOfTheHillModeSizeField).add("2 x 2", 2);
        g.ctl(this.miKingOfTheHillModeSizeField).add("3 x 3", 3);
        g.ctl(this.miKingOfTheHillModeSizeField).add("4 x 4", 4);
        g.ctl(this.miKingOfTheHillModeSizeField).add("5 x 5", 5);
        g.ctl(this.miKingOfTheHillModeSizeField).add("6 x 6", 6);
        g.ctl(this.miKingOfTheHillModeSizeField).add("7 x 7", 7);
        g.ctl(this.miKingOfTheHillModeSizeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.kingofthehill.areasize as *mut _);
        g.ctl(this.miKingOfTheHillModeSizeField).set_current_value(g.game_values.gamemodemenusettings.kingofthehill.areasize);

        this.miKingOfTheHillModeRelocateFrequencyField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 220, "Relocate", 400, 180), g);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("Never", 0);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add_random("5 Seconds", 300, false);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("10 Seconds", 600);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("15 Seconds", 900);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("20 Seconds", 1200);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("30 Seconds", 1800);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("45 Seconds", 2700);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("1 Minute", 3600);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("1.5 Minutes", 5400);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("2 Minutes", 7200);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("2.5 Minutes", 9000);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).add("3 Minutes", 10800);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).set_output_ptr(&mut g.game_values.gamemodemenusettings.kingofthehill.relocationfrequency as *mut _);
        g.ctl(this.miKingOfTheHillModeRelocateFrequencyField).set_current_value(g.game_values.gamemodemenusettings.kingofthehill.relocationfrequency);

        this.miKingOfTheHillModeMultiplierField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 260, "Max Multiplier", 400, 180), g);
        g.ctl(this.miKingOfTheHillModeMultiplierField).add("None", 1);
        g.ctl(this.miKingOfTheHillModeMultiplierField).add("2", 2);
        g.ctl(this.miKingOfTheHillModeMultiplierField).add("3", 3);
        g.ctl(this.miKingOfTheHillModeMultiplierField).add("4", 4);
        g.ctl(this.miKingOfTheHillModeMultiplierField).add("5", 5);
        g.ctl(this.miKingOfTheHillModeMultiplierField).set_output_ptr(&mut g.game_values.gamemodemenusettings.kingofthehill.maxmultiplier as *mut _);
        g.ctl(this.miKingOfTheHillModeMultiplierField).set_current_value(g.game_values.gamemodemenusettings.kingofthehill.maxmultiplier);

        this.miKingOfTheHillModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miKingOfTheHillModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miKingOfTheHillModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miKingOfTheHillModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miKingOfTheHillModeHeaderText = Ctl::new(MI_HeaderText::new(g, "King of the Hill Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[12].add_control(g, this.miKingOfTheHillModeSizeField.id(), this.miKingOfTheHillModeBackButton.id(), this.miKingOfTheHillModeRelocateFrequencyField.id(), Default::default(), this.miKingOfTheHillModeBackButton.id());
        this.mModeSettingsMenu[12].add_control(g, this.miKingOfTheHillModeRelocateFrequencyField.id(), this.miKingOfTheHillModeSizeField.id(), this.miKingOfTheHillModeMultiplierField.id(), Default::default(), this.miKingOfTheHillModeBackButton.id());
        this.mModeSettingsMenu[12].add_control(g, this.miKingOfTheHillModeMultiplierField.id(), this.miKingOfTheHillModeRelocateFrequencyField.id(), this.miKingOfTheHillModeBackButton.id(), Default::default(), this.miKingOfTheHillModeBackButton.id());

        this.mModeSettingsMenu[12].add_control(g, this.miKingOfTheHillModeBackButton.id(), this.miKingOfTheHillModeMultiplierField.id(), this.miKingOfTheHillModeSizeField.id(), this.miKingOfTheHillModeMultiplierField.id(), Default::default());

        this.mModeSettingsMenu[12].add_non_control(this.miKingOfTheHillModeLeftHeaderBar.id());
        this.mModeSettingsMenu[12].add_non_control(this.miKingOfTheHillModeRightHeaderBar.id());
        this.mModeSettingsMenu[12].add_non_control(this.miKingOfTheHillModeHeaderText.id());

        this.mModeSettingsMenu[12].set_initial_focus(g, this.miKingOfTheHillModeSizeField.id());
        this.mModeSettingsMenu[12].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Race Mode Settings
        //***********************

        this.miRaceModeQuantityField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 180, "Quantity", 400, 180), g);
        g.ctl(this.miRaceModeQuantityField).add_random("2", 2, false);
        g.ctl(this.miRaceModeQuantityField).add("3", 3);
        g.ctl(this.miRaceModeQuantityField).add("4", 4);
        g.ctl(this.miRaceModeQuantityField).add("5", 5);
        g.ctl(this.miRaceModeQuantityField).add("6", 6);
        g.ctl(this.miRaceModeQuantityField).add("7", 7);
        g.ctl(this.miRaceModeQuantityField).add("8", MAXRACEGOALS as i16);
        g.ctl(this.miRaceModeQuantityField).set_output_ptr(&mut g.game_values.gamemodemenusettings.race.quantity as *mut _);
        g.ctl(this.miRaceModeQuantityField).set_current_value(g.game_values.gamemodemenusettings.race.quantity);

        this.miRaceModeSpeedField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 220, "Speed", 400, 180), g);
        g.ctl(this.miRaceModeSpeedField).add("Stationary", 0);
        g.ctl(this.miRaceModeSpeedField).add("Very Slow", 2);
        g.ctl(this.miRaceModeSpeedField).add("Slow", 3);
        g.ctl(this.miRaceModeSpeedField).add("Moderate", 4);
        g.ctl(this.miRaceModeSpeedField).add("Fast", 6);
        g.ctl(this.miRaceModeSpeedField).add_random("Very Fast", 8, false);
        g.ctl(this.miRaceModeSpeedField).add_random("Extremely Fast", 15, false);
        g.ctl(this.miRaceModeSpeedField).add_random("Insanely Fast", 30, false);
        g.ctl(this.miRaceModeSpeedField).set_output_ptr(&mut g.game_values.gamemodemenusettings.race.speed as *mut _);
        g.ctl(this.miRaceModeSpeedField).set_current_value(g.game_values.gamemodemenusettings.race.speed);

        this.miRaceModePenaltyField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 260, "Penalty", 400, 180), g);
        g.ctl(this.miRaceModePenaltyField).add("None", 0);
        g.ctl(this.miRaceModePenaltyField).add("One Goal", 1);
        g.ctl(this.miRaceModePenaltyField).add("All Goals", 2);
        g.ctl(this.miRaceModePenaltyField).set_output_ptr(&mut g.game_values.gamemodemenusettings.race.penalty as *mut _);
        g.ctl(this.miRaceModePenaltyField).set_current_value(g.game_values.gamemodemenusettings.race.penalty);

        this.miRaceModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miRaceModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miRaceModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miRaceModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miRaceModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Race Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[13].add_control(g, this.miRaceModeQuantityField.id(), this.miRaceModeBackButton.id(), this.miRaceModeSpeedField.id(), Default::default(), this.miRaceModeBackButton.id());
        this.mModeSettingsMenu[13].add_control(g, this.miRaceModeSpeedField.id(), this.miRaceModeQuantityField.id(), this.miRaceModePenaltyField.id(), Default::default(), this.miRaceModeBackButton.id());
        this.mModeSettingsMenu[13].add_control(g, this.miRaceModePenaltyField.id(), this.miRaceModeSpeedField.id(), this.miRaceModeBackButton.id(), Default::default(), this.miRaceModeBackButton.id());

        this.mModeSettingsMenu[13].add_control(g, this.miRaceModeBackButton.id(), this.miRaceModePenaltyField.id(), this.miRaceModeQuantityField.id(), this.miRaceModePenaltyField.id(), Default::default());

        this.mModeSettingsMenu[13].add_non_control(this.miRaceModeLeftHeaderBar.id());
        this.mModeSettingsMenu[13].add_non_control(this.miRaceModeRightHeaderBar.id());
        this.mModeSettingsMenu[13].add_non_control(this.miRaceModeHeaderText.id());

        this.mModeSettingsMenu[13].set_initial_focus(g, this.miRaceModeQuantityField.id());
        this.mModeSettingsMenu[13].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Frenzy Mode Settings
        //***********************

        this.miFrenzyModeOptions = Ctl::new(MI_FrenzyModeOptions::new(g, 50, 44, 640, 7), g);
        g.ctl(this.miFrenzyModeOptions).set_auto_modify(true);

        this.miFrenzyModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miFrenzyModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miFrenzyModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Frenzy Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[15].add_control(g, this.miFrenzyModeOptions.id(), ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);

        this.mModeSettingsMenu[15].add_non_control(this.miFrenzyModeLeftHeaderBar.id());
        this.mModeSettingsMenu[15].add_non_control(this.miFrenzyModeRightHeaderBar.id());
        this.mModeSettingsMenu[15].add_non_control(this.miFrenzyModeHeaderText.id());

        this.mModeSettingsMenu[15].set_initial_focus(g, this.miFrenzyModeOptions.id());
        this.mModeSettingsMenu[15].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Survival Mode Settings
        //***********************

        for iEnemy in 0..NUMSURVIVALENEMIES as i16 {
            this.miSurvivalModeEnemySlider[iEnemy as usize] = Ctl::new(MI_PowerupSlider::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, g.rm.menu_survival, 120, 120 + 40 * iEnemy, 400, iEnemy), g);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 0);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 1);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 2);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 3);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 4);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 5);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 6);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 7);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 8);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 9);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).add("", 10);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).allow_wrap(false);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).set_output_ptr(&mut g.game_values.gamemodemenusettings.survival.enemyweight[iEnemy as usize] as *mut _);
            g.ctl(this.miSurvivalModeEnemySlider[iEnemy as usize]).set_current_value(g.game_values.gamemodemenusettings.survival.enemyweight[iEnemy as usize]);
        }

        this.miSurvivalModeDensityField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 240, "Density", 400, 180), g);
        g.ctl(this.miSurvivalModeDensityField).add_random("Very Low", 40, false);
        g.ctl(this.miSurvivalModeDensityField).add("Low", 30);
        g.ctl(this.miSurvivalModeDensityField).add("Medium", 20);
        g.ctl(this.miSurvivalModeDensityField).add("High", 15);
        g.ctl(this.miSurvivalModeDensityField).add_random("Very High", 10, false);
        g.ctl(this.miSurvivalModeDensityField).add("Extremely High", 6);
        g.ctl(this.miSurvivalModeDensityField).add_random("Insanely High", 2, false);
        g.ctl(this.miSurvivalModeDensityField).set_output_ptr(&mut g.game_values.gamemodemenusettings.survival.density as *mut _);
        g.ctl(this.miSurvivalModeDensityField).set_current_value(g.game_values.gamemodemenusettings.survival.density);

        this.miSurvivalModeSpeedField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 280, "Speed", 400, 180), g);
        g.ctl(this.miSurvivalModeSpeedField).add_random("Very Slow", 2, false);
        g.ctl(this.miSurvivalModeSpeedField).add("Slow", 3);
        g.ctl(this.miSurvivalModeSpeedField).add("Moderate", 4);
        g.ctl(this.miSurvivalModeSpeedField).add("Fast", 6);
        g.ctl(this.miSurvivalModeSpeedField).add_random("Very Fast", 8, false);
        g.ctl(this.miSurvivalModeSpeedField).add_random("Extremely Fast", 15, false);
        g.ctl(this.miSurvivalModeSpeedField).add_random("Insanely Fast", 30, false);
        g.ctl(this.miSurvivalModeSpeedField).set_output_ptr(&mut g.game_values.gamemodemenusettings.survival.speed as *mut _);
        g.ctl(this.miSurvivalModeSpeedField).set_current_value(g.game_values.gamemodemenusettings.survival.speed);

        this.miSurvivalModeShieldField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 320, "Shield", 400, 180), g);
        g.ctl(this.miSurvivalModeShieldField).add("Off", false);
        g.ctl(this.miSurvivalModeShieldField).add("On", true);
        g.ctl(this.miSurvivalModeShieldField).set_output_ptr(&mut g.game_values.gamemodemenusettings.survival.shield as *mut _);
        g.ctl(this.miSurvivalModeShieldField).set_current_value(g.game_values.gamemodemenusettings.survival.shield);
        g.ctl(this.miSurvivalModeShieldField).set_auto_advance(true);

        this.miSurvivalModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miSurvivalModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miSurvivalModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miSurvivalModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miSurvivalModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Survival Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeEnemySlider[0].id(), this.miSurvivalModeBackButton.id(), this.miSurvivalModeEnemySlider[1].id(), Default::default(), this.miSurvivalModeBackButton.id());
        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeEnemySlider[1].id(), this.miSurvivalModeEnemySlider[0].id(), this.miSurvivalModeEnemySlider[2].id(), Default::default(), this.miSurvivalModeBackButton.id());
        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeEnemySlider[2].id(), this.miSurvivalModeEnemySlider[1].id(), this.miSurvivalModeDensityField.id(), Default::default(), this.miSurvivalModeBackButton.id());
        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeDensityField.id(), this.miSurvivalModeEnemySlider[2].id(), this.miSurvivalModeSpeedField.id(), Default::default(), this.miSurvivalModeBackButton.id());
        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeSpeedField.id(), this.miSurvivalModeDensityField.id(), this.miSurvivalModeShieldField.id(), Default::default(), this.miSurvivalModeBackButton.id());
        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeShieldField.id(), this.miSurvivalModeSpeedField.id(), this.miSurvivalModeBackButton.id(), Default::default(), this.miSurvivalModeBackButton.id());
        this.mModeSettingsMenu[16].add_control(g, this.miSurvivalModeBackButton.id(), this.miSurvivalModeShieldField.id(), this.miSurvivalModeEnemySlider[0].id(), this.miSurvivalModeShieldField.id(), Default::default());

        this.mModeSettingsMenu[16].add_non_control(this.miSurvivalModeLeftHeaderBar.id());
        this.mModeSettingsMenu[16].add_non_control(this.miSurvivalModeRightHeaderBar.id());
        this.mModeSettingsMenu[16].add_non_control(this.miSurvivalModeHeaderText.id());

        this.mModeSettingsMenu[16].set_initial_focus(g, this.miSurvivalModeEnemySlider[0].id());
        this.mModeSettingsMenu[16].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Greed Mode Settings
        //***********************
        this.miGreedModeCoinLife = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 160, "Coin Life", 400, 150), g);
        g.ctl(this.miGreedModeCoinLife).add_random("1 Second", 60, false);
        g.ctl(this.miGreedModeCoinLife).add_random("2 Seconds", 120, false);
        g.ctl(this.miGreedModeCoinLife).add("3 Seconds", 180);
        g.ctl(this.miGreedModeCoinLife).add("4 Seconds", 240);
        g.ctl(this.miGreedModeCoinLife).add("5 Seconds", 300);
        g.ctl(this.miGreedModeCoinLife).add("6 Seconds", 360);
        g.ctl(this.miGreedModeCoinLife).add("7 Seconds", 420);
        g.ctl(this.miGreedModeCoinLife).add("8 Seconds", 480);
        g.ctl(this.miGreedModeCoinLife).add("9 Seconds", 540);
        g.ctl(this.miGreedModeCoinLife).add("10 Seconds", 600);
        g.ctl(this.miGreedModeCoinLife).add("12 Seconds", 720);
        g.ctl(this.miGreedModeCoinLife).add("15 Seconds", 900);
        g.ctl(this.miGreedModeCoinLife).add("18 Seconds", 1080);
        g.ctl(this.miGreedModeCoinLife).add("20 Seconds", 1200);
        g.ctl(this.miGreedModeCoinLife).add("25 Seconds", 1500);
        g.ctl(this.miGreedModeCoinLife).add("30 Seconds", 1800);
        g.ctl(this.miGreedModeCoinLife).set_output_ptr(&mut g.game_values.gamemodemenusettings.greed.coinlife as *mut _);
        g.ctl(this.miGreedModeCoinLife).set_current_value(g.game_values.gamemodemenusettings.greed.coinlife);

        this.miGreedModeOwnCoins = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 200, "Own Coins", 400, 150), g);
        g.ctl(this.miGreedModeOwnCoins).add("Yes", true);
        g.ctl(this.miGreedModeOwnCoins).add("No", false);
        g.ctl(this.miGreedModeOwnCoins).set_output_ptr(&mut g.game_values.gamemodemenusettings.greed.owncoins as *mut _);
        g.ctl(this.miGreedModeOwnCoins).set_current_value(g.game_values.gamemodemenusettings.greed.owncoins);
        g.ctl(this.miGreedModeOwnCoins).set_auto_advance(true);

        this.miGreedModeMultiplier = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 240, "Multipler", 400, 150), g);
        g.ctl(this.miGreedModeMultiplier).add_random("0.5", 1, false);
        g.ctl(this.miGreedModeMultiplier).add("1", 2);
        g.ctl(this.miGreedModeMultiplier).add("1.5", 3);
        g.ctl(this.miGreedModeMultiplier).add("2", 4);
        g.ctl(this.miGreedModeMultiplier).add_random("2.5", 5, false);
        g.ctl(this.miGreedModeMultiplier).add_random("3", 6, false);
        g.ctl(this.miGreedModeMultiplier).set_output_ptr(&mut g.game_values.gamemodemenusettings.greed.multiplier as *mut _);
        g.ctl(this.miGreedModeMultiplier).set_current_value(g.game_values.gamemodemenusettings.greed.multiplier);

        this.miGreedModePercentExtraCoin = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 280, "Extra Coins", 400, 150, 384), g);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("0", 0, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("5", 5, false);
        g.ctl(this.miGreedModePercentExtraCoin).add("10", 10);
        g.ctl(this.miGreedModePercentExtraCoin).add("15", 15);
        g.ctl(this.miGreedModePercentExtraCoin).add("20", 20);
        g.ctl(this.miGreedModePercentExtraCoin).add("25", 25);
        g.ctl(this.miGreedModePercentExtraCoin).add("30", 30);
        g.ctl(this.miGreedModePercentExtraCoin).add("35", 35);
        g.ctl(this.miGreedModePercentExtraCoin).add("40", 40);
        g.ctl(this.miGreedModePercentExtraCoin).add("45", 45);
        g.ctl(this.miGreedModePercentExtraCoin).add("50", 50);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("55", 55, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("60", 60, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("65", 65, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("70", 70, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("75", 75, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("80", 80, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("85", 85, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("90", 90, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("95", 95, false);
        g.ctl(this.miGreedModePercentExtraCoin).add_random("100", 100, false);
        g.ctl(this.miGreedModePercentExtraCoin).set_output_ptr(&mut g.game_values.gamemodemenusettings.greed.percentextracoin as *mut _);
        g.ctl(this.miGreedModePercentExtraCoin).set_current_value(g.game_values.gamemodemenusettings.greed.percentextracoin);
        g.ctl(this.miGreedModePercentExtraCoin).allow_wrap(false);

        this.miGreedModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miGreedModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miGreedModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miGreedModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miGreedModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Greed Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[17].add_control(g, this.miGreedModeCoinLife.id(), this.miGreedModeBackButton.id(), this.miGreedModeOwnCoins.id(), Default::default(), this.miGreedModeBackButton.id());
        this.mModeSettingsMenu[17].add_control(g, this.miGreedModeOwnCoins.id(), this.miGreedModeCoinLife.id(), this.miGreedModeMultiplier.id(), Default::default(), this.miGreedModeBackButton.id());
        this.mModeSettingsMenu[17].add_control(g, this.miGreedModeMultiplier.id(), this.miGreedModeOwnCoins.id(), this.miGreedModePercentExtraCoin.id(), Default::default(), this.miGreedModeBackButton.id());
        this.mModeSettingsMenu[17].add_control(g, this.miGreedModePercentExtraCoin.id(), this.miGreedModeMultiplier.id(), this.miGreedModeBackButton.id(), Default::default(), this.miGreedModeBackButton.id());

        this.mModeSettingsMenu[17].add_control(g, this.miGreedModeBackButton.id(), this.miGreedModePercentExtraCoin.id(), this.miGreedModeCoinLife.id(), this.miGreedModePercentExtraCoin.id(), Default::default());

        this.mModeSettingsMenu[17].add_non_control(this.miGreedModeLeftHeaderBar.id());
        this.mModeSettingsMenu[17].add_non_control(this.miGreedModeRightHeaderBar.id());
        this.mModeSettingsMenu[17].add_non_control(this.miGreedModeHeaderText.id());

        this.mModeSettingsMenu[17].set_initial_focus(g, this.miGreedModeCoinLife.id());
        this.mModeSettingsMenu[17].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Health Mode Settings
        //***********************
        this.miHealthModeStartLife = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 180, "Start Life", 400, 150), g);
        g.ctl(this.miHealthModeStartLife).add_random("2", 2, false);
        g.ctl(this.miHealthModeStartLife).add_random("3", 3, false);
        g.ctl(this.miHealthModeStartLife).add("4", 4);
        g.ctl(this.miHealthModeStartLife).add("5", 5);
        g.ctl(this.miHealthModeStartLife).add("6", 6);
        g.ctl(this.miHealthModeStartLife).add_random("7", 7, false);
        g.ctl(this.miHealthModeStartLife).add_random("8", 8, false);
        g.ctl(this.miHealthModeStartLife).add_random("9", 9, false);
        g.ctl(this.miHealthModeStartLife).add_random("10", 10, false);
        g.ctl(this.miHealthModeStartLife).set_output_ptr(&mut g.game_values.gamemodemenusettings.health.startlife as *mut _);
        g.ctl(this.miHealthModeStartLife).set_current_value(g.game_values.gamemodemenusettings.health.startlife);
        g.ctl(this.miHealthModeStartLife).allow_wrap(false);
        g.ctl(this.miHealthModeStartLife).set_item_changed_code(MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED);

        this.miHealthModeMaxLife = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 220, "Max Life", 400, 150), g);
        g.ctl(this.miHealthModeMaxLife).add_random("2", 2, false);
        g.ctl(this.miHealthModeMaxLife).add_random("3", 3, false);
        g.ctl(this.miHealthModeMaxLife).add_random("4", 4, false);
        g.ctl(this.miHealthModeMaxLife).add_random("5", 5, false);
        g.ctl(this.miHealthModeMaxLife).add("6", 6);
        g.ctl(this.miHealthModeMaxLife).add("7", 7);
        g.ctl(this.miHealthModeMaxLife).add("8", 8);
        g.ctl(this.miHealthModeMaxLife).add("9", 9);
        g.ctl(this.miHealthModeMaxLife).add("10", 10);
        g.ctl(this.miHealthModeMaxLife).set_output_ptr(&mut g.game_values.gamemodemenusettings.health.maxlife as *mut _);
        g.ctl(this.miHealthModeMaxLife).set_current_value(g.game_values.gamemodemenusettings.health.maxlife);
        g.ctl(this.miHealthModeMaxLife).allow_wrap(false);
        g.ctl(this.miHealthModeMaxLife).set_item_changed_code(MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED);

        this.miHealthModePercentExtraLife = Ctl::new(MI_SliderField::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, 120, 260, "Extra Life", 400, 150, 384), g);
        g.ctl(this.miHealthModePercentExtraLife).add_random("0", 0, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("5", 5, false);
        g.ctl(this.miHealthModePercentExtraLife).add("10", 10);
        g.ctl(this.miHealthModePercentExtraLife).add("15", 15);
        g.ctl(this.miHealthModePercentExtraLife).add("20", 20);
        g.ctl(this.miHealthModePercentExtraLife).add("25", 25);
        g.ctl(this.miHealthModePercentExtraLife).add("30", 30);
        g.ctl(this.miHealthModePercentExtraLife).add("35", 35);
        g.ctl(this.miHealthModePercentExtraLife).add("40", 40);
        g.ctl(this.miHealthModePercentExtraLife).add("45", 45);
        g.ctl(this.miHealthModePercentExtraLife).add("50", 50);
        g.ctl(this.miHealthModePercentExtraLife).add_random("55", 55, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("60", 60, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("65", 65, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("70", 70, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("75", 75, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("80", 80, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("85", 85, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("90", 90, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("95", 95, false);
        g.ctl(this.miHealthModePercentExtraLife).add_random("100", 100, false);
        g.ctl(this.miHealthModePercentExtraLife).set_output_ptr(&mut g.game_values.gamemodemenusettings.health.percentextralife as *mut _);
        g.ctl(this.miHealthModePercentExtraLife).set_current_value(g.game_values.gamemodemenusettings.health.percentextralife);
        g.ctl(this.miHealthModePercentExtraLife).allow_wrap(false);

        this.miHealthModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miHealthModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miHealthModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miHealthModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miHealthModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Health Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[18].add_control(g, this.miHealthModeStartLife.id(), this.miHealthModeBackButton.id(), this.miHealthModeMaxLife.id(), Default::default(), this.miHealthModeBackButton.id());
        this.mModeSettingsMenu[18].add_control(g, this.miHealthModeMaxLife.id(), this.miHealthModeStartLife.id(), this.miHealthModePercentExtraLife.id(), Default::default(), this.miHealthModeBackButton.id());
        this.mModeSettingsMenu[18].add_control(g, this.miHealthModePercentExtraLife.id(), this.miHealthModeMaxLife.id(), this.miHealthModeBackButton.id(), Default::default(), this.miHealthModeBackButton.id());
        this.mModeSettingsMenu[18].add_control(g, this.miHealthModeBackButton.id(), this.miHealthModePercentExtraLife.id(), this.miHealthModeStartLife.id(), this.miHealthModePercentExtraLife.id(), Default::default());

        this.mModeSettingsMenu[18].add_non_control(this.miHealthModeLeftHeaderBar.id());
        this.mModeSettingsMenu[18].add_non_control(this.miHealthModeRightHeaderBar.id());
        this.mModeSettingsMenu[18].add_non_control(this.miHealthModeHeaderText.id());

        this.mModeSettingsMenu[18].set_initial_focus(g, this.miHealthModeStartLife.id());
        this.mModeSettingsMenu[18].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Card Collection Mode Settings
        //***********************

        this.miCollectionModeQuantityField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 160, "Limit", 400, 180), g);
        g.ctl(this.miCollectionModeQuantityField).add("1 Card", 1);
        g.ctl(this.miCollectionModeQuantityField).add("2 Cards", 2);
        g.ctl(this.miCollectionModeQuantityField).add("3 Cards", 3);
        g.ctl(this.miCollectionModeQuantityField).add_random("4 Cards", 4, false);
        g.ctl(this.miCollectionModeQuantityField).add_random("5 Cards", 5, false);
        g.ctl(this.miCollectionModeQuantityField).add("# Players - 1", 6);
        g.ctl(this.miCollectionModeQuantityField).add("# Players", 7);
        g.ctl(this.miCollectionModeQuantityField).add_random("# Players + 1", 8, false);
        g.ctl(this.miCollectionModeQuantityField).add_random("# Players + 2", 9, false);
        g.ctl(this.miCollectionModeQuantityField).add_random("# Players + 3", 10, false);
        g.ctl(this.miCollectionModeQuantityField).set_output_ptr(&mut g.game_values.gamemodemenusettings.collection.quantity as *mut _);
        g.ctl(this.miCollectionModeQuantityField).set_current_value(g.game_values.gamemodemenusettings.collection.quantity);

        this.miCollectionModeRateField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 200, "Rate", 400, 180), g);
        g.ctl(this.miCollectionModeRateField).add("Instant", 0);
        g.ctl(this.miCollectionModeRateField).add("1 Second", 60);
        g.ctl(this.miCollectionModeRateField).add("2 Seconds", 120);
        g.ctl(this.miCollectionModeRateField).add("3 Seconds", 180);
        g.ctl(this.miCollectionModeRateField).add("5 Seconds", 300);
        g.ctl(this.miCollectionModeRateField).add_random("10 Seconds", 600, false);
        g.ctl(this.miCollectionModeRateField).add_random("15 Seconds", 900, false);
        g.ctl(this.miCollectionModeRateField).add_random("20 Seconds", 1200, false);
        g.ctl(this.miCollectionModeRateField).add_random("25 Seconds", 1500, false);
        g.ctl(this.miCollectionModeRateField).add_random("30 Seconds", 1800, false);
        g.ctl(this.miCollectionModeRateField).set_output_ptr(&mut g.game_values.gamemodemenusettings.collection.rate as *mut _);
        g.ctl(this.miCollectionModeRateField).set_current_value(g.game_values.gamemodemenusettings.collection.rate);

        this.miCollectionModeBankTimeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 240, "Bank Time", 400, 180), g);
        g.ctl(this.miCollectionModeBankTimeField).add_random("Instant", 0, false);
        g.ctl(this.miCollectionModeBankTimeField).add_random("1 Second", 60, false);
        g.ctl(this.miCollectionModeBankTimeField).add("2 Seconds", 120);
        g.ctl(this.miCollectionModeBankTimeField).add("3 Seconds", 180);
        g.ctl(this.miCollectionModeBankTimeField).add("4 Seconds", 240);
        g.ctl(this.miCollectionModeBankTimeField).add("5 Seconds", 300);
        g.ctl(this.miCollectionModeBankTimeField).add_random("6 Seconds", 360, false);
        g.ctl(this.miCollectionModeBankTimeField).add_random("7 Seconds", 420, false);
        g.ctl(this.miCollectionModeBankTimeField).add_random("8 Seconds", 480, false);
        g.ctl(this.miCollectionModeBankTimeField).add_random("9 Seconds", 540, false);
        g.ctl(this.miCollectionModeBankTimeField).add_random("10 Seconds", 600, false);
        g.ctl(this.miCollectionModeBankTimeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.collection.banktime as *mut _);
        g.ctl(this.miCollectionModeBankTimeField).set_current_value(g.game_values.gamemodemenusettings.collection.banktime);

        this.miCollectionModeCardLifeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 280, "Card Life", 400, 180), g);
        g.ctl(this.miCollectionModeCardLifeField).add_random("1 Second", 60, false);
        g.ctl(this.miCollectionModeCardLifeField).add_random("2 Seconds", 120, false);
        g.ctl(this.miCollectionModeCardLifeField).add("3 Seconds", 180);
        g.ctl(this.miCollectionModeCardLifeField).add("4 Seconds", 240);
        g.ctl(this.miCollectionModeCardLifeField).add("5 Seconds", 300);
        g.ctl(this.miCollectionModeCardLifeField).add("6 Seconds", 360);
        g.ctl(this.miCollectionModeCardLifeField).add("7 Seconds", 420);
        g.ctl(this.miCollectionModeCardLifeField).add("8 Seconds", 480);
        g.ctl(this.miCollectionModeCardLifeField).add("9 Seconds", 540);
        g.ctl(this.miCollectionModeCardLifeField).add("10 Seconds", 600);
        g.ctl(this.miCollectionModeCardLifeField).add("12 Seconds", 720);
        g.ctl(this.miCollectionModeCardLifeField).add("15 Seconds", 900);
        g.ctl(this.miCollectionModeCardLifeField).add("18 Seconds", 1080);
        g.ctl(this.miCollectionModeCardLifeField).add("20 Seconds", 1200);
        g.ctl(this.miCollectionModeCardLifeField).add("25 Seconds", 1500);
        g.ctl(this.miCollectionModeCardLifeField).add("30 Seconds", 1800);
        g.ctl(this.miCollectionModeCardLifeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.collection.cardlife as *mut _);
        g.ctl(this.miCollectionModeCardLifeField).set_current_value(g.game_values.gamemodemenusettings.collection.cardlife);

        this.miCollectionModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miCollectionModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miCollectionModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miCollectionModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miCollectionModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Card Collection Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[19].add_control(g, this.miCollectionModeQuantityField.id(), this.miCollectionModeBackButton.id(), this.miCollectionModeRateField.id(), Default::default(), this.miCollectionModeBackButton.id());
        this.mModeSettingsMenu[19].add_control(g, this.miCollectionModeRateField.id(), this.miCollectionModeQuantityField.id(), this.miCollectionModeBankTimeField.id(), Default::default(), this.miCollectionModeBackButton.id());
        this.mModeSettingsMenu[19].add_control(g, this.miCollectionModeBankTimeField.id(), this.miCollectionModeRateField.id(), this.miCollectionModeCardLifeField.id(), Default::default(), this.miCollectionModeBackButton.id());
        this.mModeSettingsMenu[19].add_control(g, this.miCollectionModeCardLifeField.id(), this.miCollectionModeBankTimeField.id(), this.miCollectionModeBackButton.id(), Default::default(), this.miCollectionModeBackButton.id());
        this.mModeSettingsMenu[19].add_control(g, this.miCollectionModeBackButton.id(), this.miCollectionModeCardLifeField.id(), this.miCollectionModeQuantityField.id(), this.miCollectionModeCardLifeField.id(), Default::default());

        this.mModeSettingsMenu[19].add_non_control(this.miCollectionModeLeftHeaderBar.id());
        this.mModeSettingsMenu[19].add_non_control(this.miCollectionModeRightHeaderBar.id());
        this.mModeSettingsMenu[19].add_non_control(this.miCollectionModeHeaderText.id());

        this.mModeSettingsMenu[19].set_initial_focus(g, this.miCollectionModeQuantityField.id());
        this.mModeSettingsMenu[19].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        //***********************
        // Chase Mode Settings
        //***********************

        this.miChaseModeSpeedField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 160, "Speed", 400, 180), g);
        g.ctl(this.miChaseModeSpeedField).add_random("Very Slow", 3, false);
        g.ctl(this.miChaseModeSpeedField).add("Slow", 4);
        g.ctl(this.miChaseModeSpeedField).add("Moderate", 5);
        g.ctl(this.miChaseModeSpeedField).add("Fast", 6);
        g.ctl(this.miChaseModeSpeedField).add("Very Fast", 7);
        g.ctl(this.miChaseModeSpeedField).add_random("Extremely Fast", 8, false);
        g.ctl(this.miChaseModeSpeedField).add_random("Insanely Fast", 10, false);
        g.ctl(this.miChaseModeSpeedField).set_output_ptr(&mut g.game_values.gamemodemenusettings.chase.phantospeed as *mut _);
        g.ctl(this.miChaseModeSpeedField).set_current_value(g.game_values.gamemodemenusettings.chase.phantospeed);

        for iPhanto in 0..3 {
            this.miChaseModeQuantitySlider[iPhanto as usize] = Ctl::new(MI_PowerupSlider::new(g.rm.spr_selectfield, g.rm.menu_slider_bar, g.rm.spr_phanto, 120, 200 + 40 * iPhanto, 400, iPhanto), g);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).add_random("", 0, !(iPhanto == 0));
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).add("", 1);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).add("", 2);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).add_random("", 3, false);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).add_random("", 4, false);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).add_random("", 5, false);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).allow_wrap(false);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).set_output_ptr(&mut g.game_values.gamemodemenusettings.chase.phantoquantity[iPhanto as usize] as *mut _);
            g.ctl(this.miChaseModeQuantitySlider[iPhanto as usize]).set_current_value(g.game_values.gamemodemenusettings.chase.phantoquantity[iPhanto as usize]);
        }

        this.miChaseModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miChaseModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miChaseModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miChaseModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miChaseModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Phanto Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[20].add_control(g, this.miChaseModeSpeedField.id(), this.miChaseModeBackButton.id(), this.miChaseModeQuantitySlider[0].id(), Default::default(), this.miChaseModeBackButton.id());

        this.mModeSettingsMenu[20].add_control(g, this.miChaseModeQuantitySlider[0].id(), this.miChaseModeSpeedField.id(), this.miChaseModeQuantitySlider[1].id(), Default::default(), this.miChaseModeBackButton.id());
        this.mModeSettingsMenu[20].add_control(g, this.miChaseModeQuantitySlider[1].id(), this.miChaseModeQuantitySlider[0].id(), this.miChaseModeQuantitySlider[2].id(), Default::default(), this.miChaseModeBackButton.id());
        this.mModeSettingsMenu[20].add_control(g, this.miChaseModeQuantitySlider[2].id(), this.miChaseModeQuantitySlider[1].id(), this.miChaseModeBackButton.id(), Default::default(), this.miChaseModeBackButton.id());

        this.mModeSettingsMenu[20].add_control(g, this.miChaseModeBackButton.id(), this.miChaseModeQuantitySlider[2].id(), this.miChaseModeSpeedField.id(), this.miChaseModeQuantitySlider[2].id(), Default::default());

        this.mModeSettingsMenu[20].add_non_control(this.miChaseModeLeftHeaderBar.id());
        this.mModeSettingsMenu[20].add_non_control(this.miChaseModeRightHeaderBar.id());
        this.mModeSettingsMenu[20].add_non_control(this.miChaseModeHeaderText.id());

        this.mModeSettingsMenu[20].set_initial_focus(g, this.miChaseModeSpeedField.id());
        this.mModeSettingsMenu[20].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Shyguy Tag Mode Settings
        //***********************

        this.miShyGuyTagModeTagOnSuicideField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 120, 180, "Suicide Tag", 400, 180), g);
        g.ctl(this.miShyGuyTagModeTagOnSuicideField).add("Off", false);
        g.ctl(this.miShyGuyTagModeTagOnSuicideField).add("On", true);
        g.ctl(this.miShyGuyTagModeTagOnSuicideField).set_output_ptr(&mut g.game_values.gamemodemenusettings.shyguytag.tagonsuicide as *mut _);
        g.ctl(this.miShyGuyTagModeTagOnSuicideField).set_current_value(g.game_values.gamemodemenusettings.shyguytag.tagonsuicide);
        g.ctl(this.miShyGuyTagModeTagOnSuicideField).set_auto_advance(true);

        this.miShyGuyTagModeTagOnStompField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 220, "Tag Transfer", 400, 180), g);
        g.ctl(this.miShyGuyTagModeTagOnStompField).add("Touch Only", 0);
        g.ctl(this.miShyGuyTagModeTagOnStompField).add("Kills Only", 1);
        g.ctl(this.miShyGuyTagModeTagOnStompField).add("Touch and Kills", 2);
        g.ctl(this.miShyGuyTagModeTagOnStompField).set_output_ptr(&mut g.game_values.gamemodemenusettings.shyguytag.tagtransfer as *mut _);
        g.ctl(this.miShyGuyTagModeTagOnStompField).set_current_value(g.game_values.gamemodemenusettings.shyguytag.tagtransfer);

        this.miShyGuyTagModeFreeTimeField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 260, "Free Time", 400, 180), g);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("Instant", 0);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("1 Second", 1);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("2 Seconds", 2);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("3 Seconds", 3);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("4 Seconds", 4);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("5 Seconds", 5);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("6 Seconds", 6);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("7 Seconds", 7);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("8 Seconds", 8);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("9 Seconds", 9);
        g.ctl(this.miShyGuyTagModeFreeTimeField).add("10 Seconds", 10);
        g.ctl(this.miShyGuyTagModeFreeTimeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.shyguytag.freetime as *mut _);
        g.ctl(this.miShyGuyTagModeFreeTimeField).set_current_value(g.game_values.gamemodemenusettings.shyguytag.freetime);

        this.miShyGuyTagModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miShyGuyTagModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miShyGuyTagModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miShyGuyTagModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miShyGuyTagModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Shyguy Tag Mode Menu", 320, 5), g);

        this.mModeSettingsMenu[21].add_control(g, this.miShyGuyTagModeTagOnSuicideField.id(), this.miShyGuyTagModeBackButton.id(), this.miShyGuyTagModeTagOnStompField.id(), Default::default(), this.miShyGuyTagModeBackButton.id());
        this.mModeSettingsMenu[21].add_control(g, this.miShyGuyTagModeTagOnStompField.id(), this.miShyGuyTagModeTagOnSuicideField.id(), this.miShyGuyTagModeFreeTimeField.id(), Default::default(), this.miShyGuyTagModeBackButton.id());
        this.mModeSettingsMenu[21].add_control(g, this.miShyGuyTagModeFreeTimeField.id(), this.miShyGuyTagModeTagOnStompField.id(), this.miShyGuyTagModeBackButton.id(), Default::default(), this.miShyGuyTagModeBackButton.id());
        this.mModeSettingsMenu[21].add_control(g, this.miShyGuyTagModeBackButton.id(), this.miShyGuyTagModeFreeTimeField.id(), this.miShyGuyTagModeTagOnSuicideField.id(), this.miShyGuyTagModeFreeTimeField.id(), Default::default());

        this.mModeSettingsMenu[21].add_non_control(this.miShyGuyTagModeLeftHeaderBar.id());
        this.mModeSettingsMenu[21].add_non_control(this.miShyGuyTagModeRightHeaderBar.id());
        this.mModeSettingsMenu[21].add_non_control(this.miShyGuyTagModeHeaderText.id());

        this.mModeSettingsMenu[21].set_initial_focus(g, this.miShyGuyTagModeTagOnSuicideField.id());
        this.mModeSettingsMenu[21].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


        //***********************
        // Boss Mode Settings
        //***********************

        this.miBossModeTypeField = Ctl::new(MI_SelectField::<Boss>::new(g.rm.spr_selectfield, 120, 180, "Type", 400, 180), g);
        g.ctl(this.miBossModeTypeField).add("Hammer", Boss::Hammer);
        g.ctl(this.miBossModeTypeField).add("Bomb", Boss::Bomb);
        g.ctl(this.miBossModeTypeField).add("Fire", Boss::Fire);
        g.ctl(this.miBossModeTypeField).set_output_ptr(&mut g.game_values.gamemodemenusettings.boss.bosstype as *mut _);
        g.ctl(this.miBossModeTypeField).set_current_value(g.game_values.gamemodemenusettings.boss.bosstype);

        this.miBossModeDifficultyField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 220, "Difficulty", 400, 180), g);
        g.ctl(this.miBossModeDifficultyField).add_random("Very Easy", 0, false);
        g.ctl(this.miBossModeDifficultyField).add("Easy", 1);
        g.ctl(this.miBossModeDifficultyField).add("Moderate", 2);
        g.ctl(this.miBossModeDifficultyField).add("Hard", 3);
        g.ctl(this.miBossModeDifficultyField).add_random("Very Hard", 4, false);
        g.ctl(this.miBossModeDifficultyField).set_output_ptr(&mut g.game_values.gamemodemenusettings.boss.difficulty as *mut _);
        g.ctl(this.miBossModeDifficultyField).set_current_value(g.game_values.gamemodemenusettings.boss.difficulty);

        this.miBossModeHitPointsField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 120, 260, "Health", 400, 180), g);
        g.ctl(this.miBossModeHitPointsField).add_random("1", 1, false);
        g.ctl(this.miBossModeHitPointsField).add_random("2", 2, false);
        g.ctl(this.miBossModeHitPointsField).add("3", 3);
        g.ctl(this.miBossModeHitPointsField).add("4", 4);
        g.ctl(this.miBossModeHitPointsField).add("5", 5);
        g.ctl(this.miBossModeHitPointsField).add("6", 6);
        g.ctl(this.miBossModeHitPointsField).add("7", 7);
        g.ctl(this.miBossModeHitPointsField).add("8", 8);
        g.ctl(this.miBossModeHitPointsField).add_random("9", 9, false);
        g.ctl(this.miBossModeHitPointsField).add_random("10", 10, false);
        g.ctl(this.miBossModeHitPointsField).add_random("11", 11, false);
        g.ctl(this.miBossModeHitPointsField).add_random("12", 12, false);
        g.ctl(this.miBossModeHitPointsField).add_random("13", 13, false);
        g.ctl(this.miBossModeHitPointsField).add_random("14", 14, false);
        g.ctl(this.miBossModeHitPointsField).add_random("15", 15, false);
        g.ctl(this.miBossModeHitPointsField).add_random("16", 16, false);
        g.ctl(this.miBossModeHitPointsField).add_random("17", 17, false);
        g.ctl(this.miBossModeHitPointsField).add_random("18", 18, false);
        g.ctl(this.miBossModeHitPointsField).add_random("19", 19, false);
        g.ctl(this.miBossModeHitPointsField).add_random("20", 20, false);
        g.ctl(this.miBossModeHitPointsField).set_output_ptr(&mut g.game_values.gamemodemenusettings.boss.hitpoints as *mut _);
        g.ctl(this.miBossModeHitPointsField).set_current_value(g.game_values.gamemodemenusettings.boss.hitpoints);

        this.miBossModeBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miBossModeBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miBossModeLeftHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miBossModeRightHeaderBar = Ctl::new(MI_Image::new(g.rm.menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miBossModeHeaderText = Ctl::new(MI_HeaderText::new(g, "Boss Minigame Menu", 320, 5), g);

        this.mBossSettingsMenu.add_control(g, this.miBossModeTypeField.id(), this.miBossModeBackButton.id(), this.miBossModeDifficultyField.id(), Default::default(), this.miBossModeBackButton.id());
        this.mBossSettingsMenu.add_control(g, this.miBossModeDifficultyField.id(), this.miBossModeTypeField.id(), this.miBossModeHitPointsField.id(), Default::default(), this.miBossModeBackButton.id());
        this.mBossSettingsMenu.add_control(g, this.miBossModeHitPointsField.id(), this.miBossModeDifficultyField.id(), this.miBossModeBackButton.id(), Default::default(), this.miBossModeBackButton.id());
        this.mBossSettingsMenu.add_control(g, this.miBossModeBackButton.id(), this.miBossModeHitPointsField.id(), this.miBossModeTypeField.id(), this.miBossModeHitPointsField.id(), Default::default());

        this.mBossSettingsMenu.add_non_control(this.miBossModeLeftHeaderBar.id());
        this.mBossSettingsMenu.add_non_control(this.miBossModeRightHeaderBar.id());
        this.mBossSettingsMenu.add_non_control(this.miBossModeHeaderText.id());

        this.mBossSettingsMenu.set_initial_focus(g, this.miBossModeTypeField.id());
        this.mBossSettingsMenu.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);
        this
    }

    /// Sets every mode menu's controlling team (-1 lets anyone control it).
    pub fn set_controlling_team(&mut self, iControlTeam: i16) {
        for iMode in 0..GAMEMODE_LAST as i16 {
            self.mModeSettingsMenu[iMode as usize].set_controlling_team(iControlTeam);
        }
    }

    /// A net match's main mode options as the menu labels them, for the preview before it starts.
    pub fn describe_game_mode_settings(&self, g: &mut Game, iMode: i16) -> Vec<String> {
        let gm = &g.game_values.gamemodesettings;
        let lines = match iMode as GameModeType {
            game_mode_classic => vec![g.ctl(self.miClassicModeStyleField).describe(gm.classic.style), g.ctl(self.miClassicModeScoringField).describe(gm.classic.scoring)],
            game_mode_frag => vec![g.ctl(self.miFragModeStyleField).describe(gm.frag.style), g.ctl(self.miFragModeScoringField).describe(gm.frag.scoring)],
            game_mode_timelimit => vec![g.ctl(self.miTimeLimitModeStyleField).describe(gm.time.style), g.ctl(self.miTimeLimitModeScoringField).describe(gm.time.scoring)],
            game_mode_jail => vec![g.ctl(self.miJailModeStyleField).describe(gm.jail.style), g.ctl(self.miJailModeTimeFreeField).describe(gm.jail.timetofree), g.ctl(self.miJailModeTagFreeField).describe(gm.jail.tagfree)],
            game_mode_coins => vec![g.ctl(self.miCoinModePenaltyField).describe(gm.coins.penalty), g.ctl(self.miCoinModeQuantityField).describe(gm.coins.quantity)],
            game_mode_stomp => vec![g.ctl(self.miStompModeRateField).describe(gm.stomp.rate)],
            game_mode_domination => vec![g.ctl(self.miDominationModeQuantityField).describe(gm.domination.quantity)],
            game_mode_race => vec![g.ctl(self.miRaceModeQuantityField).describe(gm.race.quantity), g.ctl(self.miRaceModePenaltyField).describe(gm.race.penalty)],
            game_mode_collection => vec![g.ctl(self.miCollectionModeQuantityField).describe(gm.collection.quantity)],
            _ => vec![],
        };
        lines.into_iter().flatten().collect()
    }

    /// Rolls random values for mode `iMode` into `gamemodesettings`; sliders copy their current value instead.
    pub fn set_random_game_mode_settings(&mut self, g: &mut Game, iMode: i16) {
        let iMode = iMode as GameModeType;
        if iMode == game_mode_classic {
            // classic
            g.game_values.gamemodesettings.classic.style = g.ctl(self.miClassicModeStyleField).random_value(g);
            g.game_values.gamemodesettings.classic.scoring = g.ctl(self.miClassicModeScoringField).random_value(g);
        } else if iMode == game_mode_frag {
            // frag
            g.game_values.gamemodesettings.frag.style = g.ctl(self.miFragModeStyleField).random_value(g);
            g.game_values.gamemodesettings.frag.scoring = g.ctl(self.miFragModeScoringField).random_value(g);
        } else if iMode == game_mode_timelimit {
            // time
            g.game_values.gamemodesettings.time.style = g.ctl(self.miTimeLimitModeStyleField).random_value(g);
            g.game_values.gamemodesettings.time.scoring = g.ctl(self.miTimeLimitModeScoringField).random_value(g);
            g.game_values.gamemodesettings.time.percentextratime = g.ctl(self.miTimeLimitModePercentExtraTime).current_value();
        } else if iMode == game_mode_jail {
            // jail
            g.game_values.gamemodesettings.jail.style = g.ctl(self.miJailModeStyleField).random_value(g);
            g.game_values.gamemodesettings.jail.timetofree = g.ctl(self.miJailModeTimeFreeField).random_value(g);
            g.game_values.gamemodesettings.jail.tagfree = g.ctl(self.miJailModeTagFreeField).random_value(g);
            g.game_values.gamemodesettings.jail.percentkey = g.ctl(self.miJailModeJailKeyField).current_value();
        } else if iMode == game_mode_coins {
            // coins
            g.game_values.gamemodesettings.coins.penalty = g.ctl(self.miCoinModePenaltyField).random_value(g);
            g.game_values.gamemodesettings.coins.quantity = g.ctl(self.miCoinModeQuantityField).random_value(g);
            g.game_values.gamemodesettings.coins.percentextracoin = g.ctl(self.miCoinModePercentExtraCoin).current_value();
        } else if iMode == game_mode_stomp {
            // stomp
            g.game_values.gamemodesettings.stomp.rate = g.ctl(self.miStompModeRateField).random_value(g);

            for iEnemy in 0..NUMSTOMPENEMIES as usize {
                g.game_values.gamemodesettings.stomp.enemyweight[iEnemy] = g.ctl(self.miStompModeEnemySlider[iEnemy]).current_value();
            }
        } else if iMode == game_mode_eggs {
            // egg
            for iEgg in 0..4 {
                g.game_values.gamemodesettings.egg.eggs[iEgg] = g.ctl(self.miEggModeEggQuantityField[iEgg]).current_value();
            }

            for iYoshi in 0..4 {
                g.game_values.gamemodesettings.egg.yoshis[iYoshi] = g.ctl(self.miEggModeYoshiQuantityField[iYoshi]).current_value();
            }

            g.game_values.gamemodesettings.egg.explode = g.ctl(self.miEggModeExplosionTimeField).random_value(g);
        } else if iMode == game_mode_ctf {
            // capture the flag
            g.game_values.gamemodesettings.flag.speed = g.ctl(self.miFlagModeSpeedField).current_value();
            g.game_values.gamemodesettings.flag.touchreturn = g.ctl(self.miFlagModeTouchReturnField).random_value(g);
            g.game_values.gamemodesettings.flag.pointmove = g.ctl(self.miFlagModePointMoveField).random_value(g);
            g.game_values.gamemodesettings.flag.autoreturn = g.ctl(self.miFlagModeAutoReturnField).random_value(g);
            g.game_values.gamemodesettings.flag.homescore = g.ctl(self.miFlagModeHomeScoreField).random_value(g);
            g.game_values.gamemodesettings.flag.centerflag = g.ctl(self.miFlagModeCenterFlagField).random_value(g);
        } else if iMode == game_mode_chicken {
            // chicken
            g.game_values.gamemodesettings.chicken.usetarget = g.ctl(self.miChickenModeShowTargetField).random_value(g);
            g.game_values.gamemodesettings.chicken.glide = g.ctl(self.miChickenModeGlideField).random_value(g);
        } else if iMode == game_mode_tag {
            // tag
            g.game_values.gamemodesettings.tag.tagontouch = g.ctl(self.miTagModeTagOnTouchField).random_value(g);
        } else if iMode == game_mode_star {
            // star
            g.game_values.gamemodesettings.star.time = g.ctl(self.miStarModeTimeField).random_value(g);
            g.game_values.gamemodesettings.star.shine = g.ctl(self.miStarModeShineField).random_value(g);
            g.game_values.gamemodesettings.star.percentextratime = g.ctl(self.miStarModePercentExtraTime).current_value();
        } else if iMode == game_mode_domination {
            // domination
            g.game_values.gamemodesettings.domination.quantity = g.ctl(self.miDominationModeQuantityField).random_value(g);
            g.game_values.gamemodesettings.domination.loseondeath = g.ctl(self.miDominationModeLoseOnDeathField).random_value(g);
            g.game_values.gamemodesettings.domination.relocateondeath = g.ctl(self.miDominationModeRelocateOnDeathField).random_value(g);
            g.game_values.gamemodesettings.domination.stealondeath = g.ctl(self.miDominationModeStealOnDeathField).random_value(g);
            g.game_values.gamemodesettings.domination.relocationfrequency = g.ctl(self.miDominationModeRelocateFrequencyField).random_value(g);
        } else if iMode == game_mode_koth {
            // king of the hill
            g.game_values.gamemodesettings.kingofthehill.areasize = g.ctl(self.miKingOfTheHillModeSizeField).random_value(g);
            g.game_values.gamemodesettings.kingofthehill.relocationfrequency = g.ctl(self.miKingOfTheHillModeRelocateFrequencyField).random_value(g);
            g.game_values.gamemodesettings.kingofthehill.maxmultiplier = g.ctl(self.miKingOfTheHillModeMultiplierField).random_value(g);
        } else if iMode == game_mode_race {
            // race
            g.game_values.gamemodesettings.race.quantity = g.ctl(self.miRaceModeQuantityField).random_value(g);
            g.game_values.gamemodesettings.race.speed = g.ctl(self.miRaceModeSpeedField).random_value(g);
            g.game_values.gamemodesettings.race.penalty = g.ctl(self.miRaceModePenaltyField).random_value(g);
        } else if iMode == game_mode_frenzy {
            // frenzy
            g.ctl(self.miFrenzyModeOptions).set_random_game_mode_settings(g);
        } else if iMode == game_mode_survival {
            // survival
            g.game_values.gamemodesettings.survival.density = g.ctl(self.miSurvivalModeDensityField).random_value(g);
            g.game_values.gamemodesettings.survival.speed = g.ctl(self.miSurvivalModeSpeedField).random_value(g);
            g.game_values.gamemodesettings.survival.shield = g.ctl(self.miSurvivalModeShieldField).random_value(g);

            for iEnemy in 0..NUMSURVIVALENEMIES as usize {
                g.game_values.gamemodesettings.survival.enemyweight[iEnemy] = g.ctl(self.miSurvivalModeEnemySlider[iEnemy]).current_value();
            }
        } else if iMode == game_mode_greed {
            // greed
            g.game_values.gamemodesettings.greed.coinlife = g.ctl(self.miGreedModeCoinLife).random_value(g);
            g.game_values.gamemodesettings.greed.owncoins = g.ctl(self.miGreedModeOwnCoins).random_value(g);
            g.game_values.gamemodesettings.greed.multiplier = g.ctl(self.miGreedModeMultiplier).random_value(g);
            g.game_values.gamemodesettings.greed.percentextracoin = g.ctl(self.miGreedModePercentExtraCoin).current_value();
        } else if iMode == game_mode_health {
            // health
            g.game_values.gamemodesettings.health.startlife = g.ctl(self.miHealthModeStartLife).random_value(g);
            g.game_values.gamemodesettings.health.maxlife = g.ctl(self.miHealthModeMaxLife).random_value(g);
            g.game_values.gamemodesettings.health.percentextralife = g.ctl(self.miHealthModePercentExtraLife).current_value();
        } else if iMode == game_mode_collection {
            // card collection
            g.game_values.gamemodesettings.collection.quantity = g.ctl(self.miCollectionModeQuantityField).random_value(g);
            g.game_values.gamemodesettings.collection.rate = g.ctl(self.miCollectionModeRateField).random_value(g);
            g.game_values.gamemodesettings.collection.banktime = g.ctl(self.miCollectionModeBankTimeField).random_value(g);
            g.game_values.gamemodesettings.collection.cardlife = g.ctl(self.miCollectionModeCardLifeField).random_value(g);
        } else if iMode == game_mode_chase {
            // chase (phanto)
            g.game_values.gamemodesettings.chase.phantospeed = g.ctl(self.miChaseModeSpeedField).random_value(g);

            for iPhanto in 0..3 {
                g.game_values.gamemodesettings.chase.phantoquantity[iPhanto] = g.ctl(self.miChaseModeQuantitySlider[iPhanto]).current_value();
            }
        } else if iMode == game_mode_shyguytag {
            // shyguy tag
            g.game_values.gamemodesettings.shyguytag.tagonsuicide = g.ctl(self.miShyGuyTagModeTagOnSuicideField).random_value(g);
            g.game_values.gamemodesettings.shyguytag.tagtransfer = g.ctl(self.miShyGuyTagModeTagOnStompField).random_value(g);
            // freetime reads the tag transfer field, not miShyGuyTagModeFreeTimeField.
            g.game_values.gamemodesettings.shyguytag.freetime = g.ctl(self.miShyGuyTagModeTagOnStompField).random_value(g);
        } else if iMode == game_mode_boss_minigame {
            // boss
            g.game_values.gamemodesettings.boss.bosstype = g.ctl(self.miBossModeTypeField).random_value(g);
            g.game_values.gamemodesettings.boss.difficulty = g.ctl(self.miBossModeDifficultyField).random_value(g);
            g.game_values.gamemodesettings.boss.hitpoints = g.ctl(self.miBossModeHitPointsField).random_value(g);
        }
    }

    /// The settings menu of mode `iMode`.
    pub fn get_options_menu(&mut self, iMode: i16) -> Ptr<UI_Menu> {
        Ptr::from_mut(&mut self.mModeSettingsMenu[iMode as usize])
    }
    pub fn get_boss_options_menu(&mut self) -> Ptr<UI_Menu> {
        Ptr::from_mut(&mut self.mBossSettingsMenu)
    }

    /// Keeps start life at or below max life.
    pub fn health_mode_start_life_changed(&mut self, g: &mut Game) {
        let iMaxLife: i16 = g.ctl(self.miHealthModeMaxLife).current_value();
        if g.ctl(self.miHealthModeStartLife).current_value() > iMaxLife {
            g.ctl(self.miHealthModeStartLife).set_current_value(iMaxLife);
        }
    }

    /// Keeps max life at or above start life.
    pub fn health_mode_max_life_changed(&mut self, g: &mut Game) {
        let iStartLife: i16 = g.ctl(self.miHealthModeStartLife).current_value();
        if g.ctl(self.miHealthModeMaxLife).current_value() < iStartLife {
            g.ctl(self.miHealthModeMaxLife).set_current_value(iStartLife);
        }
    }

    /// Refreshes every mode menu's fields from the values they write.
    pub fn refresh(&mut self, g: &mut Game) {
        for iMode in 0..GAMEMODE_LAST as i16 {
            self.mModeSettingsMenu[iMode as usize].refresh(g);
        }

        self.mBossSettingsMenu.refresh(g);
    }
}
