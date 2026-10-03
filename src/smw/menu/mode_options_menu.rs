//! Port of src/smw/menu/ModeOptionsMenu.cpp

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
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_frenzy_mode_options::MI_FrenzyModeOptions;
use crate::smw::ui::mi_powerup_slider::MI_PowerupSlider;

#[derive(Default)]
pub struct UI_ModeOptionsMenu {
    pub ui_menu: UI_Menu,

    // Game Mode Settings Menu
    pub mModeSettingsMenu: [UI_Menu; 22],
    pub mBossSettingsMenu: UI_Menu,
    // Classic
    pub miClassicModeStyleField: Ptr<MI_SelectField<DeathStyle>>,
    pub miClassicModeScoringField: Ptr<MI_SelectField<ScoringStyle>>,
    pub miClassicModeBackButton: Ptr<MI_Button>,
    pub miClassicModeLeftHeaderBar: Ptr<MI_Image>,
    pub miClassicModeRightHeaderBar: Ptr<MI_Image>,
    pub miClassicModeHeaderText: Ptr<MI_Text>,
    // Frag
    pub miFragModeStyleField: Ptr<MI_SelectField<DeathStyle>>,
    pub miFragModeScoringField: Ptr<MI_SelectField<ScoringStyle>>,
    pub miFragModeBackButton: Ptr<MI_Button>,
    pub miFragModeLeftHeaderBar: Ptr<MI_Image>,
    pub miFragModeRightHeaderBar: Ptr<MI_Image>,
    pub miFragModeHeaderText: Ptr<MI_Text>,
    // Time Limit
    pub miTimeLimitModeStyleField: Ptr<MI_SelectField<DeathStyle>>,
    pub miTimeLimitModeScoringField: Ptr<MI_SelectField<ScoringStyle>>,
    pub miTimeLimitModePercentExtraTime: Ptr<MI_SliderField>,
    pub miTimeLimitModeBackButton: Ptr<MI_Button>,
    pub miTimeLimitModeLeftHeaderBar: Ptr<MI_Image>,
    pub miTimeLimitModeRightHeaderBar: Ptr<MI_Image>,
    pub miTimeLimitModeHeaderText: Ptr<MI_Text>,
    // Jail
    pub miJailModeStyleField: Ptr<MI_SelectField<JailStyle>>,
    pub miJailModeTagFreeField: Ptr<MI_SelectField<bool>>,
    pub miJailModeTimeFreeField: Ptr<MI_SelectField<i16>>,
    pub miJailModeJailKeyField: Ptr<MI_SliderField>,
    pub miJailModeBackButton: Ptr<MI_Button>,
    pub miJailModeLeftHeaderBar: Ptr<MI_Image>,
    pub miJailModeRightHeaderBar: Ptr<MI_Image>,
    pub miJailModeHeaderText: Ptr<MI_Text>,
    // Coins
    pub miCoinModePenaltyField: Ptr<MI_SelectField<bool>>,
    pub miCoinModeQuantityField: Ptr<MI_SelectField<i16>>,
    pub miCoinModePercentExtraCoin: Ptr<MI_SliderField>,
    pub miCoinModeBackButton: Ptr<MI_Button>,
    pub miCoinModeLeftHeaderBar: Ptr<MI_Image>,
    pub miCoinModeRightHeaderBar: Ptr<MI_Image>,
    pub miCoinModeHeaderText: Ptr<MI_Text>,
    // Yoshi's Eggs
    pub miEggModeEggQuantityField: [Ptr<MI_PowerupSlider>; 4],
    pub miEggModeYoshiQuantityField: [Ptr<MI_PowerupSlider>; 4],
    pub miEggModeExplosionTimeField: Ptr<MI_SelectField<i16>>,
    pub miEggModeBackButton: Ptr<MI_Button>,
    pub miEggModeLeftHeaderBar: Ptr<MI_Image>,
    pub miEggModeRightHeaderBar: Ptr<MI_Image>,
    pub miEggModeHeaderText: Ptr<MI_Text>,
    // Capture The Flag
    pub miFlagModeSpeedField: Ptr<MI_SliderField>,
    pub miFlagModeTouchReturnField: Ptr<MI_SelectField<bool>>,
    pub miFlagModePointMoveField: Ptr<MI_SelectField<bool>>,
    pub miFlagModeAutoReturnField: Ptr<MI_SelectField<i16>>,
    pub miFlagModeHomeScoreField: Ptr<MI_SelectField<bool>>,
    pub miFlagModeCenterFlagField: Ptr<MI_SelectField<bool>>,
    pub miFlagModeBackButton: Ptr<MI_Button>,
    pub miFlagModeLeftHeaderBar: Ptr<MI_Image>,
    pub miFlagModeRightHeaderBar: Ptr<MI_Image>,
    pub miFlagModeHeaderText: Ptr<MI_Text>,
    // Chicken
    pub miChickenModeShowTargetField: Ptr<MI_SelectField<bool>>,
    pub miChickenModeGlideField: Ptr<MI_SelectField<bool>>,
    pub miChickenModeBackButton: Ptr<MI_Button>,
    pub miChickenModeLeftHeaderBar: Ptr<MI_Image>,
    pub miChickenModeRightHeaderBar: Ptr<MI_Image>,
    pub miChickenModeHeaderText: Ptr<MI_Text>,
    // Tag
    pub miTagModeTagOnTouchField: Ptr<MI_SelectField<bool>>,
    pub miTagModeBackButton: Ptr<MI_Button>,
    pub miTagModeLeftHeaderBar: Ptr<MI_Image>,
    pub miTagModeRightHeaderBar: Ptr<MI_Image>,
    pub miTagModeHeaderText: Ptr<MI_Text>,
    // Star
    pub miStarModeTimeField: Ptr<MI_SelectField<i16>>,
    pub miStarModeShineField: Ptr<MI_SelectField<StarStyle>>,
    pub miStarModePercentExtraTime: Ptr<MI_SliderField>,
    pub miStarModeBackButton: Ptr<MI_Button>,
    pub miStarModeLeftHeaderBar: Ptr<MI_Image>,
    pub miStarModeRightHeaderBar: Ptr<MI_Image>,
    pub miStarModeHeaderText: Ptr<MI_Text>,
    // Domination
    pub miDominationModeQuantityField: Ptr<MI_SelectField<i16>>,
    pub miDominationModeLoseOnDeathField: Ptr<MI_SelectField<bool>>,
    pub miDominationModeRelocateOnDeathField: Ptr<MI_SelectField<bool>>,
    pub miDominationModeStealOnDeathField: Ptr<MI_SelectField<bool>>,
    pub miDominationModeRelocateFrequencyField: Ptr<MI_SelectField<i16>>,
    pub miDominationModeDeathText: Ptr<MI_Text>,
    pub miDominationModeBackButton: Ptr<MI_Button>,
    pub miDominationModeLeftHeaderBar: Ptr<MI_Image>,
    pub miDominationModeRightHeaderBar: Ptr<MI_Image>,
    pub miDominationModeHeaderText: Ptr<MI_Text>,
    // King of the Hill
    pub miKingOfTheHillModeSizeField: Ptr<MI_SelectField<i16>>,
    pub miKingOfTheHillModeRelocateFrequencyField: Ptr<MI_SelectField<i16>>,
    pub miKingOfTheHillModeMultiplierField: Ptr<MI_SelectField<i16>>,
    pub miKingOfTheHillModeBackButton: Ptr<MI_Button>,
    pub miKingOfTheHillModeLeftHeaderBar: Ptr<MI_Image>,
    pub miKingOfTheHillModeRightHeaderBar: Ptr<MI_Image>,
    pub miKingOfTheHillModeHeaderText: Ptr<MI_Text>,
    // Race
    pub miRaceModeQuantityField: Ptr<MI_SelectField<i16>>,
    pub miRaceModeSpeedField: Ptr<MI_SelectField<i16>>,
    pub miRaceModePenaltyField: Ptr<MI_SelectField<i16>>,
    pub miRaceModeBackButton: Ptr<MI_Button>,
    pub miRaceModeLeftHeaderBar: Ptr<MI_Image>,
    pub miRaceModeRightHeaderBar: Ptr<MI_Image>,
    pub miRaceModeHeaderText: Ptr<MI_Text>,
    // Stomp
    pub miStompModeRateField: Ptr<MI_SelectField<i16>>,
    pub miStompModeEnemySlider: [Ptr<MI_PowerupSlider>; NUMSTOMPENEMIES as usize],
    pub miStompModeBackButton: Ptr<MI_Button>,
    pub miStompModeLeftHeaderBar: Ptr<MI_Image>,
    pub miStompModeRightHeaderBar: Ptr<MI_Image>,
    pub miStompModeHeaderText: Ptr<MI_Text>,
    // Frenzy
    pub miFrenzyModeOptions: Ptr<MI_FrenzyModeOptions>,
    pub miFrenzyModeLeftHeaderBar: Ptr<MI_Image>,
    pub miFrenzyModeRightHeaderBar: Ptr<MI_Image>,
    pub miFrenzyModeHeaderText: Ptr<MI_Text>,
    // Survival
    pub miSurvivalModeEnemySlider: [Ptr<MI_PowerupSlider>; NUMSURVIVALENEMIES as usize],
    pub miSurvivalModeDensityField: Ptr<MI_SelectField<i16>>,
    pub miSurvivalModeSpeedField: Ptr<MI_SelectField<i16>>,
    pub miSurvivalModeShieldField: Ptr<MI_SelectField<bool>>,
    pub miSurvivalModeBackButton: Ptr<MI_Button>,
    pub miSurvivalModeLeftHeaderBar: Ptr<MI_Image>,
    pub miSurvivalModeRightHeaderBar: Ptr<MI_Image>,
    pub miSurvivalModeHeaderText: Ptr<MI_Text>,
    // Greed
    pub miGreedModeCoinLife: Ptr<MI_SelectField<i16>>,
    pub miGreedModeOwnCoins: Ptr<MI_SelectField<bool>>,
    pub miGreedModeMultiplier: Ptr<MI_SelectField<i16>>,
    pub miGreedModePercentExtraCoin: Ptr<MI_SliderField>,
    pub miGreedModeBackButton: Ptr<MI_Button>,
    pub miGreedModeLeftHeaderBar: Ptr<MI_Image>,
    pub miGreedModeRightHeaderBar: Ptr<MI_Image>,
    pub miGreedModeHeaderText: Ptr<MI_Text>,
    // Health
    pub miHealthModeStartLife: Ptr<MI_SelectField<i16>>,
    pub miHealthModeMaxLife: Ptr<MI_SelectField<i16>>,
    pub miHealthModePercentExtraLife: Ptr<MI_SliderField>,
    pub miHealthModeBackButton: Ptr<MI_Button>,
    pub miHealthModeLeftHeaderBar: Ptr<MI_Image>,
    pub miHealthModeRightHeaderBar: Ptr<MI_Image>,
    pub miHealthModeHeaderText: Ptr<MI_Text>,
    // Card Collection
    pub miCollectionModeQuantityField: Ptr<MI_SelectField<i16>>,
    pub miCollectionModeRateField: Ptr<MI_SelectField<i16>>,
    pub miCollectionModeBankTimeField: Ptr<MI_SelectField<i16>>,
    pub miCollectionModeCardLifeField: Ptr<MI_SelectField<i16>>,
    pub miCollectionModeBackButton: Ptr<MI_Button>,
    pub miCollectionModeLeftHeaderBar: Ptr<MI_Image>,
    pub miCollectionModeRightHeaderBar: Ptr<MI_Image>,
    pub miCollectionModeHeaderText: Ptr<MI_Text>,
    // Phanto Chase
    pub miChaseModeSpeedField: Ptr<MI_SelectField<i16>>,
    pub miChaseModeQuantitySlider: [Ptr<MI_PowerupSlider>; 3],
    pub miChaseModeBackButton: Ptr<MI_Button>,
    pub miChaseModeLeftHeaderBar: Ptr<MI_Image>,
    pub miChaseModeRightHeaderBar: Ptr<MI_Image>,
    pub miChaseModeHeaderText: Ptr<MI_Text>,
    // Shyguy Tag
    pub miShyGuyTagModeTagOnSuicideField: Ptr<MI_SelectField<bool>>,
    pub miShyGuyTagModeTagOnStompField: Ptr<MI_SelectField<i16>>,
    pub miShyGuyTagModeFreeTimeField: Ptr<MI_SelectField<i16>>,
    pub miShyGuyTagModeBackButton: Ptr<MI_Button>,
    pub miShyGuyTagModeLeftHeaderBar: Ptr<MI_Image>,
    pub miShyGuyTagModeRightHeaderBar: Ptr<MI_Image>,
    pub miShyGuyTagModeHeaderText: Ptr<MI_Text>,
    // Boss
    pub miBossModeTypeField: Ptr<MI_SelectField<Boss>>,
    pub miBossModeDifficultyField: Ptr<MI_SelectField<i16>>,
    pub miBossModeHitPointsField: Ptr<MI_SelectField<i16>>,
    pub miBossModeBackButton: Ptr<MI_Button>,
    pub miBossModeLeftHeaderBar: Ptr<MI_Image>,
    pub miBossModeRightHeaderBar: Ptr<MI_Image>,
    pub miBossModeHeaderText: Ptr<MI_Text>,
}

crate::impl_base!(UI_ModeOptionsMenu => ui_menu: UI_Menu);

impl UI_ModeOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            //***********************
            // Classic Mode Settings
            //***********************

            this.miClassicModeStyleField = Ptr::new_box(MI_SelectField::<DeathStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "On Kill", 400, 180));
            this.miClassicModeStyleField.add("Respawn", DeathStyle::Respawn);
            this.miClassicModeStyleField.add("Shield", DeathStyle::Shield);
            this.miClassicModeStyleField.set_output_ptr(&mut game_values.gamemodemenusettings.classic.style as *mut _);
            this.miClassicModeStyleField.set_current_value(game_values.gamemodemenusettings.classic.style);

            this.miClassicModeScoringField = Ptr::new_box(MI_SelectField::<ScoringStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Scoring", 400, 180));
            this.miClassicModeScoringField.add("All Kills", ScoringStyle::AllKills);
            this.miClassicModeScoringField.add_random("Push Kills Only", ScoringStyle::PushOnly, false);
            this.miClassicModeScoringField.set_output_ptr(&mut game_values.gamemodemenusettings.classic.scoring as *mut _);
            this.miClassicModeScoringField.set_current_value(game_values.gamemodemenusettings.classic.scoring);

            this.miClassicModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miClassicModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miClassicModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miClassicModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miClassicModeHeaderText = Ptr::new_box(MI_HeaderText::new("Classic Mode Menu", 320, 5));

            this.mModeSettingsMenu[0].add_control(ctl_ptr(this.miClassicModeStyleField), ctl_ptr(this.miClassicModeBackButton), ctl_ptr(this.miClassicModeScoringField), Ptr::null(), ctl_ptr(this.miClassicModeBackButton));
            this.mModeSettingsMenu[0].add_control(ctl_ptr(this.miClassicModeScoringField), ctl_ptr(this.miClassicModeStyleField), ctl_ptr(this.miClassicModeBackButton), Ptr::null(), ctl_ptr(this.miClassicModeBackButton));
            this.mModeSettingsMenu[0].add_control(ctl_ptr(this.miClassicModeBackButton), ctl_ptr(this.miClassicModeScoringField), ctl_ptr(this.miClassicModeStyleField), ctl_ptr(this.miClassicModeScoringField), Ptr::null());

            this.mModeSettingsMenu[0].add_non_control(ctl_ptr(this.miClassicModeLeftHeaderBar));
            this.mModeSettingsMenu[0].add_non_control(ctl_ptr(this.miClassicModeRightHeaderBar));
            this.mModeSettingsMenu[0].add_non_control(ctl_ptr(this.miClassicModeHeaderText));

            this.mModeSettingsMenu[0].set_initial_focus(ctl_ptr(this.miClassicModeStyleField));
            this.mModeSettingsMenu[0].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Frag Mode Settings
            //***********************

            this.miFragModeStyleField = Ptr::new_box(MI_SelectField::<DeathStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "On Kill", 400, 180));
            this.miFragModeStyleField.add("Respawn", DeathStyle::Respawn);
            this.miFragModeStyleField.add("Shield", DeathStyle::Shield);
            this.miFragModeStyleField.set_output_ptr(&mut game_values.gamemodemenusettings.frag.style as *mut _);
            this.miFragModeStyleField.set_current_value(game_values.gamemodemenusettings.frag.style);

            this.miFragModeScoringField = Ptr::new_box(MI_SelectField::<ScoringStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Scoring", 400, 180));
            this.miFragModeScoringField.add("All Kills", ScoringStyle::AllKills);
            this.miFragModeScoringField.add_random("Push Kills Only", ScoringStyle::PushOnly, false);
            this.miFragModeScoringField.set_output_ptr(&mut game_values.gamemodemenusettings.frag.scoring as *mut _);
            this.miFragModeScoringField.set_current_value(game_values.gamemodemenusettings.frag.scoring);

            this.miFragModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miFragModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miFragModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miFragModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miFragModeHeaderText = Ptr::new_box(MI_HeaderText::new("Frag Mode Menu", 320, 5));

            this.mModeSettingsMenu[1].add_control(ctl_ptr(this.miFragModeStyleField), ctl_ptr(this.miFragModeBackButton), ctl_ptr(this.miFragModeScoringField), Ptr::null(), ctl_ptr(this.miFragModeBackButton));
            this.mModeSettingsMenu[1].add_control(ctl_ptr(this.miFragModeScoringField), ctl_ptr(this.miFragModeStyleField), ctl_ptr(this.miFragModeBackButton), Ptr::null(), ctl_ptr(this.miFragModeBackButton));
            this.mModeSettingsMenu[1].add_control(ctl_ptr(this.miFragModeBackButton), ctl_ptr(this.miFragModeScoringField), ctl_ptr(this.miFragModeStyleField), ctl_ptr(this.miFragModeScoringField), Ptr::null());

            this.mModeSettingsMenu[1].add_non_control(ctl_ptr(this.miFragModeLeftHeaderBar));
            this.mModeSettingsMenu[1].add_non_control(ctl_ptr(this.miFragModeRightHeaderBar));
            this.mModeSettingsMenu[1].add_non_control(ctl_ptr(this.miFragModeHeaderText));

            this.mModeSettingsMenu[1].set_initial_focus(ctl_ptr(this.miFragModeStyleField));
            this.mModeSettingsMenu[1].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Time Limit Mode Settings
            //***********************

            this.miTimeLimitModeStyleField = Ptr::new_box(MI_SelectField::<DeathStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "On Kill", 400, 150));
            this.miTimeLimitModeStyleField.add("Respawn", DeathStyle::Respawn);
            this.miTimeLimitModeStyleField.add("Shield", DeathStyle::Shield);
            this.miTimeLimitModeStyleField.set_output_ptr(&mut game_values.gamemodemenusettings.time.style as *mut _);
            this.miTimeLimitModeStyleField.set_current_value(game_values.gamemodemenusettings.time.style);

            this.miTimeLimitModeScoringField = Ptr::new_box(MI_SelectField::<ScoringStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Scoring", 400, 150));
            this.miTimeLimitModeScoringField.add("All Kills", ScoringStyle::AllKills);
            this.miTimeLimitModeScoringField.add_random("Push Kills Only", ScoringStyle::PushOnly, false);
            this.miTimeLimitModeScoringField.set_output_ptr(&mut game_values.gamemodemenusettings.time.scoring as *mut _);
            this.miTimeLimitModeScoringField.set_current_value(game_values.gamemodemenusettings.time.scoring);

            this.miTimeLimitModePercentExtraTime = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 260, "Extra Time", 400, 150, 384));
            this.miTimeLimitModePercentExtraTime.add_random("0", 0, false);
            this.miTimeLimitModePercentExtraTime.add_random("5", 5, false);
            this.miTimeLimitModePercentExtraTime.add("10", 10);
            this.miTimeLimitModePercentExtraTime.add("15", 15);
            this.miTimeLimitModePercentExtraTime.add("20", 20);
            this.miTimeLimitModePercentExtraTime.add("25", 25);
            this.miTimeLimitModePercentExtraTime.add("30", 30);
            this.miTimeLimitModePercentExtraTime.add("35", 35);
            this.miTimeLimitModePercentExtraTime.add("40", 40);
            this.miTimeLimitModePercentExtraTime.add("45", 45);
            this.miTimeLimitModePercentExtraTime.add("50", 50);
            this.miTimeLimitModePercentExtraTime.add_random("55", 55, false);
            this.miTimeLimitModePercentExtraTime.add_random("60", 60, false);
            this.miTimeLimitModePercentExtraTime.add_random("65", 65, false);
            this.miTimeLimitModePercentExtraTime.add_random("70", 70, false);
            this.miTimeLimitModePercentExtraTime.add_random("75", 75, false);
            this.miTimeLimitModePercentExtraTime.add_random("80", 80, false);
            this.miTimeLimitModePercentExtraTime.add_random("85", 85, false);
            this.miTimeLimitModePercentExtraTime.add_random("90", 90, false);
            this.miTimeLimitModePercentExtraTime.add_random("95", 95, false);
            this.miTimeLimitModePercentExtraTime.add_random("100", 100, false);
            this.miTimeLimitModePercentExtraTime.set_output_ptr(&mut game_values.gamemodemenusettings.time.percentextratime as *mut _);
            this.miTimeLimitModePercentExtraTime.set_current_value(game_values.gamemodemenusettings.time.percentextratime);
            this.miTimeLimitModePercentExtraTime.allow_wrap(false);

            this.miTimeLimitModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miTimeLimitModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miTimeLimitModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miTimeLimitModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miTimeLimitModeHeaderText = Ptr::new_box(MI_HeaderText::new("Time Mode Menu", 320, 5));

            this.mModeSettingsMenu[2].add_control(ctl_ptr(this.miTimeLimitModeStyleField), ctl_ptr(this.miTimeLimitModeBackButton), ctl_ptr(this.miTimeLimitModeScoringField), Ptr::null(), ctl_ptr(this.miTimeLimitModeBackButton));
            this.mModeSettingsMenu[2].add_control(ctl_ptr(this.miTimeLimitModeScoringField), ctl_ptr(this.miTimeLimitModeStyleField), ctl_ptr(this.miTimeLimitModePercentExtraTime), Ptr::null(), ctl_ptr(this.miTimeLimitModeBackButton));
            this.mModeSettingsMenu[2].add_control(ctl_ptr(this.miTimeLimitModePercentExtraTime), ctl_ptr(this.miTimeLimitModeScoringField), ctl_ptr(this.miTimeLimitModeBackButton), Ptr::null(), ctl_ptr(this.miTimeLimitModeBackButton));
            this.mModeSettingsMenu[2].add_control(ctl_ptr(this.miTimeLimitModeBackButton), ctl_ptr(this.miTimeLimitModePercentExtraTime), ctl_ptr(this.miTimeLimitModeStyleField), ctl_ptr(this.miTimeLimitModePercentExtraTime), Ptr::null());

            this.mModeSettingsMenu[2].add_non_control(ctl_ptr(this.miTimeLimitModeLeftHeaderBar));
            this.mModeSettingsMenu[2].add_non_control(ctl_ptr(this.miTimeLimitModeRightHeaderBar));
            this.mModeSettingsMenu[2].add_non_control(ctl_ptr(this.miTimeLimitModeHeaderText));

            this.mModeSettingsMenu[2].set_initial_focus(ctl_ptr(this.miTimeLimitModeStyleField));
            this.mModeSettingsMenu[2].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Jail Mode Settings
            //***********************

            this.miJailModeStyleField = Ptr::new_box(MI_SelectField::<JailStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 160, "Style", 400, 150));
            this.miJailModeStyleField.add("Classic", JailStyle::Classic);
            this.miJailModeStyleField.add("Owned", JailStyle::Owned);
            this.miJailModeStyleField.add("Free For All", JailStyle::FreeForAll);
            this.miJailModeStyleField.set_output_ptr(&mut game_values.gamemodemenusettings.jail.style as *mut _);
            this.miJailModeStyleField.set_current_value(game_values.gamemodemenusettings.jail.style);

            this.miJailModeTimeFreeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "Free Timer", 400, 150));
            this.miJailModeTimeFreeField.add("None", 1);
            this.miJailModeTimeFreeField.add_random("5 Seconds", 310, false);
            this.miJailModeTimeFreeField.add("10 Seconds", 620);
            this.miJailModeTimeFreeField.add("15 Seconds", 930);
            this.miJailModeTimeFreeField.add("20 Seconds", 1240);
            this.miJailModeTimeFreeField.add("25 Seconds", 1550);
            this.miJailModeTimeFreeField.add("30 Seconds", 1860);
            this.miJailModeTimeFreeField.add("35 Seconds", 2170);
            this.miJailModeTimeFreeField.add("40 Seconds", 2480);
            this.miJailModeTimeFreeField.add_random("45 Seconds", 2790, false);
            this.miJailModeTimeFreeField.add_random("50 Seconds", 3100, false);
            this.miJailModeTimeFreeField.add_random("55 Seconds", 3410, false);
            this.miJailModeTimeFreeField.add_random("60 Seconds", 3720, false);
            this.miJailModeTimeFreeField.set_output_ptr(&mut game_values.gamemodemenusettings.jail.timetofree as *mut _);
            this.miJailModeTimeFreeField.set_current_value(game_values.gamemodemenusettings.jail.timetofree);

            this.miJailModeTagFreeField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Tag Free", 400, 150));
            this.miJailModeTagFreeField.add("Off", false);
            this.miJailModeTagFreeField.add("On", true);
            this.miJailModeTagFreeField.set_output_ptr(&mut game_values.gamemodemenusettings.jail.tagfree as *mut _);
            this.miJailModeTagFreeField.set_current_value(game_values.gamemodemenusettings.jail.tagfree);
            this.miJailModeTagFreeField.set_auto_advance(true);

            this.miJailModeJailKeyField = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 280, "Jail Key", 400, 150, 384));
            this.miJailModeJailKeyField.add_random("0", 0, false);
            this.miJailModeJailKeyField.add_random("5", 5, false);
            this.miJailModeJailKeyField.add("10", 10);
            this.miJailModeJailKeyField.add("15", 15);
            this.miJailModeJailKeyField.add("20", 20);
            this.miJailModeJailKeyField.add("25", 25);
            this.miJailModeJailKeyField.add("30", 30);
            this.miJailModeJailKeyField.add("35", 35);
            this.miJailModeJailKeyField.add("40", 40);
            this.miJailModeJailKeyField.add("45", 45);
            this.miJailModeJailKeyField.add("50", 50);
            this.miJailModeJailKeyField.add_random("55", 55, false);
            this.miJailModeJailKeyField.add_random("60", 60, false);
            this.miJailModeJailKeyField.add_random("65", 65, false);
            this.miJailModeJailKeyField.add_random("70", 70, false);
            this.miJailModeJailKeyField.add_random("75", 75, false);
            this.miJailModeJailKeyField.add_random("80", 80, false);
            this.miJailModeJailKeyField.add_random("85", 85, false);
            this.miJailModeJailKeyField.add_random("90", 90, false);
            this.miJailModeJailKeyField.add_random("95", 95, false);
            this.miJailModeJailKeyField.add_random("100", 100, false);
            this.miJailModeJailKeyField.set_output_ptr(&mut game_values.gamemodemenusettings.jail.percentkey as *mut _);
            this.miJailModeJailKeyField.set_current_value(game_values.gamemodemenusettings.jail.percentkey);
            this.miJailModeJailKeyField.allow_wrap(false);

            this.miJailModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miJailModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miJailModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miJailModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miJailModeHeaderText = Ptr::new_box(MI_HeaderText::new("Jail Mode Menu", 320, 5));

            this.mModeSettingsMenu[3].add_control(ctl_ptr(this.miJailModeStyleField), ctl_ptr(this.miJailModeBackButton), ctl_ptr(this.miJailModeTimeFreeField), Ptr::null(), ctl_ptr(this.miJailModeBackButton));
            this.mModeSettingsMenu[3].add_control(ctl_ptr(this.miJailModeTimeFreeField), ctl_ptr(this.miJailModeStyleField), ctl_ptr(this.miJailModeTagFreeField), Ptr::null(), ctl_ptr(this.miJailModeBackButton));
            this.mModeSettingsMenu[3].add_control(ctl_ptr(this.miJailModeTagFreeField), ctl_ptr(this.miJailModeTimeFreeField), ctl_ptr(this.miJailModeJailKeyField), Ptr::null(), ctl_ptr(this.miJailModeBackButton));
            this.mModeSettingsMenu[3].add_control(ctl_ptr(this.miJailModeJailKeyField), ctl_ptr(this.miJailModeTagFreeField), ctl_ptr(this.miJailModeBackButton), Ptr::null(), ctl_ptr(this.miJailModeBackButton));
            this.mModeSettingsMenu[3].add_control(ctl_ptr(this.miJailModeBackButton), ctl_ptr(this.miJailModeJailKeyField), ctl_ptr(this.miJailModeStyleField), ctl_ptr(this.miJailModeJailKeyField), Ptr::null());

            this.mModeSettingsMenu[3].add_non_control(ctl_ptr(this.miJailModeLeftHeaderBar));
            this.mModeSettingsMenu[3].add_non_control(ctl_ptr(this.miJailModeRightHeaderBar));
            this.mModeSettingsMenu[3].add_non_control(ctl_ptr(this.miJailModeHeaderText));

            this.mModeSettingsMenu[3].set_initial_focus(ctl_ptr(this.miJailModeStyleField));
            this.mModeSettingsMenu[3].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Coins Mode Settings
            //***********************

            this.miCoinModePenaltyField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Penalty", 400, 150));
            this.miCoinModePenaltyField.add("Off", false);
            this.miCoinModePenaltyField.add("On", true);
            this.miCoinModePenaltyField.set_output_ptr(&mut game_values.gamemodemenusettings.coins.penalty as *mut _);
            this.miCoinModePenaltyField.set_current_value(game_values.gamemodemenusettings.coins.penalty);
            this.miCoinModePenaltyField.set_auto_advance(true);

            this.miCoinModeQuantityField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Quantity", 400, 150));
            this.miCoinModeQuantityField.add("1", 1);
            this.miCoinModeQuantityField.add("2", 2);
            this.miCoinModeQuantityField.add("3", 3);
            this.miCoinModeQuantityField.add("4", 4);
            this.miCoinModeQuantityField.add("5", 5);
            this.miCoinModeQuantityField.add("6", 6);
            this.miCoinModeQuantityField.add("7", 7);
            this.miCoinModeQuantityField.add("8", 8);
            this.miCoinModeQuantityField.add("9", 9);
            this.miCoinModeQuantityField.add("10", 10);
            this.miCoinModeQuantityField.set_output_ptr(&mut game_values.gamemodemenusettings.coins.quantity as *mut _);
            this.miCoinModeQuantityField.set_current_value(game_values.gamemodemenusettings.coins.quantity);

            this.miCoinModePercentExtraCoin = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 260, "Extra Coins", 400, 150, 384));
            this.miCoinModePercentExtraCoin.add_random("0", 0, false);
            this.miCoinModePercentExtraCoin.add_random("5", 5, false);
            this.miCoinModePercentExtraCoin.add("10", 10);
            this.miCoinModePercentExtraCoin.add("15", 15);
            this.miCoinModePercentExtraCoin.add("20", 20);
            this.miCoinModePercentExtraCoin.add("25", 25);
            this.miCoinModePercentExtraCoin.add("30", 30);
            this.miCoinModePercentExtraCoin.add("35", 35);
            this.miCoinModePercentExtraCoin.add("40", 40);
            this.miCoinModePercentExtraCoin.add("45", 45);
            this.miCoinModePercentExtraCoin.add("50", 50);
            this.miCoinModePercentExtraCoin.add_random("55", 55, false);
            this.miCoinModePercentExtraCoin.add_random("60", 60, false);
            this.miCoinModePercentExtraCoin.add_random("65", 65, false);
            this.miCoinModePercentExtraCoin.add_random("70", 70, false);
            this.miCoinModePercentExtraCoin.add_random("75", 75, false);
            this.miCoinModePercentExtraCoin.add_random("80", 80, false);
            this.miCoinModePercentExtraCoin.add_random("85", 85, false);
            this.miCoinModePercentExtraCoin.add_random("90", 90, false);
            this.miCoinModePercentExtraCoin.add_random("95", 95, false);
            this.miCoinModePercentExtraCoin.add_random("100", 100, false);
            this.miCoinModePercentExtraCoin.set_output_ptr(&mut game_values.gamemodemenusettings.coins.percentextracoin as *mut _);
            this.miCoinModePercentExtraCoin.set_current_value(game_values.gamemodemenusettings.coins.percentextracoin);
            this.miCoinModePercentExtraCoin.allow_wrap(false);

            this.miCoinModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miCoinModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miCoinModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miCoinModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miCoinModeHeaderText = Ptr::new_box(MI_HeaderText::new("Coin Collection Mode Menu", 320, 5));

            this.mModeSettingsMenu[4].add_control(ctl_ptr(this.miCoinModePenaltyField), ctl_ptr(this.miCoinModeBackButton), ctl_ptr(this.miCoinModeQuantityField), Ptr::null(), ctl_ptr(this.miCoinModeBackButton));
            this.mModeSettingsMenu[4].add_control(ctl_ptr(this.miCoinModeQuantityField), ctl_ptr(this.miCoinModePenaltyField), ctl_ptr(this.miCoinModePercentExtraCoin), Ptr::null(), ctl_ptr(this.miCoinModeBackButton));
            this.mModeSettingsMenu[4].add_control(ctl_ptr(this.miCoinModePercentExtraCoin), ctl_ptr(this.miCoinModeQuantityField), ctl_ptr(this.miCoinModeBackButton), Ptr::null(), ctl_ptr(this.miCoinModeBackButton));
            this.mModeSettingsMenu[4].add_control(ctl_ptr(this.miCoinModeBackButton), ctl_ptr(this.miCoinModePercentExtraCoin), ctl_ptr(this.miCoinModePenaltyField), ctl_ptr(this.miCoinModePercentExtraCoin), Ptr::null());

            this.mModeSettingsMenu[4].add_non_control(ctl_ptr(this.miCoinModeLeftHeaderBar));
            this.mModeSettingsMenu[4].add_non_control(ctl_ptr(this.miCoinModeRightHeaderBar));
            this.mModeSettingsMenu[4].add_non_control(ctl_ptr(this.miCoinModeHeaderText));

            this.mModeSettingsMenu[4].set_initial_focus(ctl_ptr(this.miCoinModePenaltyField));
            this.mModeSettingsMenu[4].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Stomp Mode Settings
            //***********************

            this.miStompModeRateField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 40, "Rate", 400, 180));
            this.miStompModeRateField.add_random("Very Slow", 150, false);
            this.miStompModeRateField.add("Slow", 120);
            this.miStompModeRateField.add("Moderate", 90);
            this.miStompModeRateField.add("Fast", 60);
            this.miStompModeRateField.add_random("Very Fast", 30, false);
            this.miStompModeRateField.add_random("Extremely Fast", 15, false);
            this.miStompModeRateField.add_random("Insanely Fast", 5, false);
            this.miStompModeRateField.set_output_ptr(&mut game_values.gamemodemenusettings.stomp.rate as *mut _);
            this.miStompModeRateField.set_current_value(game_values.gamemodemenusettings.stomp.rate);

            for iEnemy in 0..NUMSTOMPENEMIES as i16 {
                this.miStompModeEnemySlider[iEnemy as usize] = Ptr::new_box(MI_PowerupSlider::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), Ptr::from_mut(&mut rm.menu_stomp), 120, 80 + 40 * iEnemy, 400, iEnemy));
                this.miStompModeEnemySlider[iEnemy as usize].add("", 0);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 1);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 2);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 3);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 4);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 5);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 6);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 7);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 8);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 9);
                this.miStompModeEnemySlider[iEnemy as usize].add("", 10);
                this.miStompModeEnemySlider[iEnemy as usize].allow_wrap(false);
                this.miStompModeEnemySlider[iEnemy as usize].set_output_ptr(&mut game_values.gamemodemenusettings.stomp.enemyweight[iEnemy as usize] as *mut _);
                this.miStompModeEnemySlider[iEnemy as usize].set_current_value(game_values.gamemodemenusettings.stomp.enemyweight[iEnemy as usize]);
            }

            this.miStompModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miStompModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miStompModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miStompModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miStompModeHeaderText = Ptr::new_box(MI_HeaderText::new("Stomp Mode Menu", 320, 5));


            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeRateField), ctl_ptr(this.miStompModeBackButton), ctl_ptr(this.miStompModeEnemySlider[0]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));

            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[0]), ctl_ptr(this.miStompModeRateField), ctl_ptr(this.miStompModeEnemySlider[1]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[1]), ctl_ptr(this.miStompModeEnemySlider[0]), ctl_ptr(this.miStompModeEnemySlider[2]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[2]), ctl_ptr(this.miStompModeEnemySlider[1]), ctl_ptr(this.miStompModeEnemySlider[3]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[3]), ctl_ptr(this.miStompModeEnemySlider[2]), ctl_ptr(this.miStompModeEnemySlider[4]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[4]), ctl_ptr(this.miStompModeEnemySlider[3]), ctl_ptr(this.miStompModeEnemySlider[5]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[5]), ctl_ptr(this.miStompModeEnemySlider[4]), ctl_ptr(this.miStompModeEnemySlider[6]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[6]), ctl_ptr(this.miStompModeEnemySlider[5]), ctl_ptr(this.miStompModeEnemySlider[7]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[7]), ctl_ptr(this.miStompModeEnemySlider[6]), ctl_ptr(this.miStompModeEnemySlider[8]), Ptr::null(), ctl_ptr(this.miStompModeBackButton));
            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeEnemySlider[8]), ctl_ptr(this.miStompModeEnemySlider[7]), ctl_ptr(this.miStompModeBackButton), Ptr::null(), ctl_ptr(this.miStompModeBackButton));

            this.mModeSettingsMenu[5].add_control(ctl_ptr(this.miStompModeBackButton), ctl_ptr(this.miStompModeEnemySlider[8]), ctl_ptr(this.miStompModeRateField), ctl_ptr(this.miStompModeEnemySlider[8]), Ptr::null());

            this.mModeSettingsMenu[5].add_non_control(ctl_ptr(this.miStompModeLeftHeaderBar));
            this.mModeSettingsMenu[5].add_non_control(ctl_ptr(this.miStompModeRightHeaderBar));
            this.mModeSettingsMenu[5].add_non_control(ctl_ptr(this.miStompModeHeaderText));

            this.mModeSettingsMenu[5].set_initial_focus(ctl_ptr(this.miStompModeRateField));
            this.mModeSettingsMenu[5].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Yoshi's Eggs Mode Settings
            //***********************

            for iEggField in 0..4 {
                this.miEggModeEggQuantityField[iEggField as usize] = Ptr::new_box(MI_PowerupSlider::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), Ptr::from_mut(&mut rm.menu_egg), 170, 60 + 40 * iEggField, 300, iEggField));
                this.miEggModeEggQuantityField[iEggField as usize].add_random("0", 0, !(iEggField == 0));
                this.miEggModeEggQuantityField[iEggField as usize].add_random("1", 1, !(iEggField >= 2));
                this.miEggModeEggQuantityField[iEggField as usize].add_random("2", 2, !(iEggField >= 2));
                this.miEggModeEggQuantityField[iEggField as usize].add_random("3", 3, !(iEggField >= 1));
                this.miEggModeEggQuantityField[iEggField as usize].add("4", 4);
                this.miEggModeEggQuantityField[iEggField as usize].set_output_ptr(&mut game_values.gamemodemenusettings.egg.eggs[iEggField as usize] as *mut _);
                this.miEggModeEggQuantityField[iEggField as usize].set_current_value(game_values.gamemodemenusettings.egg.eggs[iEggField as usize]);
                this.miEggModeEggQuantityField[iEggField as usize].allow_wrap(false);
            }

            for iYoshiField in 0..4 {
                this.miEggModeYoshiQuantityField[iYoshiField as usize] = Ptr::new_box(MI_PowerupSlider::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), Ptr::from_mut(&mut rm.menu_egg), 170, 220 + 40 * iYoshiField, 300, iYoshiField + 4));
                this.miEggModeYoshiQuantityField[iYoshiField as usize].add_random("0", 0, !(iYoshiField == 0));
                this.miEggModeYoshiQuantityField[iYoshiField as usize].add_random("1", 1, !(iYoshiField >= 2));
                this.miEggModeYoshiQuantityField[iYoshiField as usize].add_random("2", 2, !(iYoshiField >= 1));
                this.miEggModeYoshiQuantityField[iYoshiField as usize].add("3", 3);
                this.miEggModeYoshiQuantityField[iYoshiField as usize].add("4", 4);
                this.miEggModeYoshiQuantityField[iYoshiField as usize].set_output_ptr(&mut game_values.gamemodemenusettings.egg.yoshis[iYoshiField as usize] as *mut _);
                this.miEggModeYoshiQuantityField[iYoshiField as usize].set_current_value(game_values.gamemodemenusettings.egg.yoshis[iYoshiField as usize]);
                this.miEggModeYoshiQuantityField[iYoshiField as usize].allow_wrap(false);
            }

            this.miEggModeExplosionTimeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 380, "Explosion Timer", 400, 220));
            this.miEggModeExplosionTimeField.add("Off", 0);
            this.miEggModeExplosionTimeField.add_random("3 Seconds", 3, false);
            this.miEggModeExplosionTimeField.add("5 Seconds", 5);
            this.miEggModeExplosionTimeField.add("8 Seconds", 8);
            this.miEggModeExplosionTimeField.add("10 Seconds", 10);
            this.miEggModeExplosionTimeField.add("15 Seconds", 15);
            this.miEggModeExplosionTimeField.add("20 Seconds", 20);
            this.miEggModeExplosionTimeField.set_output_ptr(&mut game_values.gamemodemenusettings.egg.explode as *mut _);
            this.miEggModeExplosionTimeField.set_current_value(game_values.gamemodemenusettings.egg.explode);

            this.miEggModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miEggModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miEggModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miEggModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miEggModeHeaderText = Ptr::new_box(MI_HeaderText::new("Yoshi's Eggs Mode Menu", 320, 5));

            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeEggQuantityField[0]), ctl_ptr(this.miEggModeBackButton), ctl_ptr(this.miEggModeEggQuantityField[1]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));
            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeEggQuantityField[1]), ctl_ptr(this.miEggModeEggQuantityField[0]), ctl_ptr(this.miEggModeEggQuantityField[2]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));
            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeEggQuantityField[2]), ctl_ptr(this.miEggModeEggQuantityField[1]), ctl_ptr(this.miEggModeEggQuantityField[3]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));
            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeEggQuantityField[3]), ctl_ptr(this.miEggModeEggQuantityField[2]), ctl_ptr(this.miEggModeYoshiQuantityField[0]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));

            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeYoshiQuantityField[0]), ctl_ptr(this.miEggModeEggQuantityField[3]), ctl_ptr(this.miEggModeYoshiQuantityField[1]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));
            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeYoshiQuantityField[1]), ctl_ptr(this.miEggModeYoshiQuantityField[0]), ctl_ptr(this.miEggModeYoshiQuantityField[2]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));
            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeYoshiQuantityField[2]), ctl_ptr(this.miEggModeYoshiQuantityField[1]), ctl_ptr(this.miEggModeYoshiQuantityField[3]), Ptr::null(), ctl_ptr(this.miEggModeBackButton));
            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeYoshiQuantityField[3]), ctl_ptr(this.miEggModeYoshiQuantityField[2]), ctl_ptr(this.miEggModeExplosionTimeField), Ptr::null(), ctl_ptr(this.miEggModeBackButton));

            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeExplosionTimeField), ctl_ptr(this.miEggModeYoshiQuantityField[3]), ctl_ptr(this.miEggModeBackButton), Ptr::null(), ctl_ptr(this.miEggModeBackButton));

            this.mModeSettingsMenu[6].add_control(ctl_ptr(this.miEggModeBackButton), ctl_ptr(this.miEggModeExplosionTimeField), ctl_ptr(this.miEggModeEggQuantityField[0]), ctl_ptr(this.miEggModeExplosionTimeField), Ptr::null());

            this.mModeSettingsMenu[6].add_non_control(ctl_ptr(this.miEggModeLeftHeaderBar));
            this.mModeSettingsMenu[6].add_non_control(ctl_ptr(this.miEggModeRightHeaderBar));
            this.mModeSettingsMenu[6].add_non_control(ctl_ptr(this.miEggModeHeaderText));

            this.mModeSettingsMenu[6].set_initial_focus(ctl_ptr(this.miEggModeEggQuantityField[0]));
            this.mModeSettingsMenu[6].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            //***********************
            // Flag Mode Settings
            //***********************

            this.miFlagModeSpeedField = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 120, "Speed", 400, 180, 380));
            this.miFlagModeSpeedField.add("-", -1);
            this.miFlagModeSpeedField.add("0", 0);
            this.miFlagModeSpeedField.add("1", 1);
            this.miFlagModeSpeedField.add("2", 2);
            this.miFlagModeSpeedField.add("3", 3);
            this.miFlagModeSpeedField.add("4", 4);
            this.miFlagModeSpeedField.add("5", 5);
            this.miFlagModeSpeedField.add_random("6", 6, false);
            this.miFlagModeSpeedField.add_random("7", 7, false);
            this.miFlagModeSpeedField.add_random("8", 8, false);
            this.miFlagModeSpeedField.set_output_ptr(&mut game_values.gamemodemenusettings.flag.speed as *mut _);
            this.miFlagModeSpeedField.set_current_value(game_values.gamemodemenusettings.flag.speed);
            this.miFlagModeSpeedField.allow_wrap(false);

            this.miFlagModeTouchReturnField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 160, "Touch Return", 400, 180));
            this.miFlagModeTouchReturnField.add("Off", false);
            this.miFlagModeTouchReturnField.add("On", true);
            this.miFlagModeTouchReturnField.set_output_ptr(&mut game_values.gamemodemenusettings.flag.touchreturn as *mut _);
            this.miFlagModeTouchReturnField.set_current_value(game_values.gamemodemenusettings.flag.touchreturn);
            this.miFlagModeTouchReturnField.set_auto_advance(true);

            this.miFlagModePointMoveField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "Point Move", 400, 180));
            this.miFlagModePointMoveField.add("Off", false);
            this.miFlagModePointMoveField.add("On", true);
            this.miFlagModePointMoveField.set_output_ptr(&mut game_values.gamemodemenusettings.flag.pointmove as *mut _);
            this.miFlagModePointMoveField.set_current_value(game_values.gamemodemenusettings.flag.pointmove);
            this.miFlagModePointMoveField.set_auto_advance(true);

            this.miFlagModeAutoReturnField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Auto Return", 400, 180));
            this.miFlagModeAutoReturnField.add("None", 0);
            this.miFlagModeAutoReturnField.add_random("5 Seconds", 310, false);
            this.miFlagModeAutoReturnField.add("10 Seconds", 620);
            this.miFlagModeAutoReturnField.add_random("15 Seconds", 930, false);
            this.miFlagModeAutoReturnField.add("20 Seconds", 1240);
            this.miFlagModeAutoReturnField.add_random("25 Seconds", 1550, false);
            this.miFlagModeAutoReturnField.add_random("30 Seconds", 1860, false);
            this.miFlagModeAutoReturnField.add_random("35 Seconds", 2170, false);
            this.miFlagModeAutoReturnField.add_random("40 Seconds", 2480, false);
            this.miFlagModeAutoReturnField.add_random("45 Seconds", 2790, false);
            this.miFlagModeAutoReturnField.add_random("50 Seconds", 3100, false);
            this.miFlagModeAutoReturnField.add_random("55 Seconds", 3410, false);
            this.miFlagModeAutoReturnField.add_random("60 Seconds", 3720, false);
            this.miFlagModeAutoReturnField.set_output_ptr(&mut game_values.gamemodemenusettings.flag.autoreturn as *mut _);
            this.miFlagModeAutoReturnField.set_current_value(game_values.gamemodemenusettings.flag.autoreturn);

            this.miFlagModeHomeScoreField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 280, "Need Home", 400, 180));
            this.miFlagModeHomeScoreField.add("Off", false);
            this.miFlagModeHomeScoreField.add("On", true);
            this.miFlagModeHomeScoreField.set_output_ptr(&mut game_values.gamemodemenusettings.flag.homescore as *mut _);
            this.miFlagModeHomeScoreField.set_current_value(game_values.gamemodemenusettings.flag.homescore);
            this.miFlagModeHomeScoreField.set_auto_advance(true);

            this.miFlagModeCenterFlagField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 320, "Center Flag", 400, 180));
            this.miFlagModeCenterFlagField.add("Off", false);
            this.miFlagModeCenterFlagField.add("On", true);
            this.miFlagModeCenterFlagField.set_output_ptr(&mut game_values.gamemodemenusettings.flag.centerflag as *mut _);
            this.miFlagModeCenterFlagField.set_current_value(game_values.gamemodemenusettings.flag.centerflag);
            this.miFlagModeCenterFlagField.set_auto_advance(true);

            this.miFlagModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miFlagModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miFlagModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miFlagModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miFlagModeHeaderText = Ptr::new_box(MI_HeaderText::new("Capture The Flag Mode Menu", 320, 5));

            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModeSpeedField), ctl_ptr(this.miFlagModeBackButton), ctl_ptr(this.miFlagModeTouchReturnField), Ptr::null(), ctl_ptr(this.miFlagModeBackButton));
            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModeTouchReturnField), ctl_ptr(this.miFlagModeSpeedField), ctl_ptr(this.miFlagModePointMoveField), Ptr::null(), ctl_ptr(this.miFlagModeBackButton));
            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModePointMoveField), ctl_ptr(this.miFlagModeTouchReturnField), ctl_ptr(this.miFlagModeAutoReturnField), Ptr::null(), ctl_ptr(this.miFlagModeBackButton));
            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModeAutoReturnField), ctl_ptr(this.miFlagModePointMoveField), ctl_ptr(this.miFlagModeHomeScoreField), Ptr::null(), ctl_ptr(this.miFlagModeBackButton));
            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModeHomeScoreField), ctl_ptr(this.miFlagModeAutoReturnField), ctl_ptr(this.miFlagModeCenterFlagField), Ptr::null(), ctl_ptr(this.miFlagModeBackButton));
            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModeCenterFlagField), ctl_ptr(this.miFlagModeHomeScoreField), ctl_ptr(this.miFlagModeBackButton), Ptr::null(), ctl_ptr(this.miFlagModeBackButton));

            this.mModeSettingsMenu[7].add_control(ctl_ptr(this.miFlagModeBackButton), ctl_ptr(this.miFlagModeCenterFlagField), ctl_ptr(this.miFlagModeSpeedField), ctl_ptr(this.miFlagModeCenterFlagField), Ptr::null());

            this.mModeSettingsMenu[7].add_non_control(ctl_ptr(this.miFlagModeLeftHeaderBar));
            this.mModeSettingsMenu[7].add_non_control(ctl_ptr(this.miFlagModeRightHeaderBar));
            this.mModeSettingsMenu[7].add_non_control(ctl_ptr(this.miFlagModeHeaderText));

            this.mModeSettingsMenu[7].set_initial_focus(ctl_ptr(this.miFlagModeSpeedField));
            this.mModeSettingsMenu[7].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Chicken Mode Settings
            //***********************

            this.miChickenModeShowTargetField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "Show Target", 400, 180));
            this.miChickenModeShowTargetField.add("Off", false);
            this.miChickenModeShowTargetField.add("On", true);
            this.miChickenModeShowTargetField.set_output_ptr(&mut game_values.gamemodemenusettings.chicken.usetarget as *mut _);
            this.miChickenModeShowTargetField.set_current_value(game_values.gamemodemenusettings.chicken.usetarget);
            this.miChickenModeShowTargetField.set_auto_advance(true);

            this.miChickenModeGlideField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Chicken Glide", 400, 180));
            this.miChickenModeGlideField.add("Off", false);
            this.miChickenModeGlideField.add("On", true);
            this.miChickenModeGlideField.set_output_ptr(&mut game_values.gamemodemenusettings.chicken.glide as *mut _);
            this.miChickenModeGlideField.set_current_value(game_values.gamemodemenusettings.chicken.glide);
            this.miChickenModeGlideField.set_auto_advance(true);

            this.miChickenModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miChickenModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miChickenModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miChickenModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miChickenModeHeaderText = Ptr::new_box(MI_HeaderText::new("Chicken Mode Menu", 320, 5));


            this.mModeSettingsMenu[8].add_control(ctl_ptr(this.miChickenModeShowTargetField), ctl_ptr(this.miChickenModeBackButton), ctl_ptr(this.miChickenModeGlideField), Ptr::null(), ctl_ptr(this.miChickenModeBackButton));
            this.mModeSettingsMenu[8].add_control(ctl_ptr(this.miChickenModeGlideField), ctl_ptr(this.miChickenModeShowTargetField), ctl_ptr(this.miChickenModeBackButton), Ptr::null(), ctl_ptr(this.miChickenModeBackButton));
            this.mModeSettingsMenu[8].add_control(ctl_ptr(this.miChickenModeBackButton), ctl_ptr(this.miChickenModeGlideField), ctl_ptr(this.miChickenModeShowTargetField), ctl_ptr(this.miChickenModeGlideField), Ptr::null());

            this.mModeSettingsMenu[8].add_non_control(ctl_ptr(this.miChickenModeLeftHeaderBar));
            this.mModeSettingsMenu[8].add_non_control(ctl_ptr(this.miChickenModeRightHeaderBar));
            this.mModeSettingsMenu[8].add_non_control(ctl_ptr(this.miChickenModeHeaderText));

            this.mModeSettingsMenu[8].set_initial_focus(ctl_ptr(this.miChickenModeShowTargetField));
            this.mModeSettingsMenu[8].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Tag Mode Settings
            //***********************

            this.miTagModeTagOnTouchField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Touch Tag", 400, 180));
            this.miTagModeTagOnTouchField.add("Off", false);
            this.miTagModeTagOnTouchField.add("On", true);
            this.miTagModeTagOnTouchField.set_output_ptr(&mut game_values.gamemodemenusettings.tag.tagontouch as *mut _);
            this.miTagModeTagOnTouchField.set_current_value(game_values.gamemodemenusettings.tag.tagontouch);
            this.miTagModeTagOnTouchField.set_auto_advance(true);

            this.miTagModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miTagModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miTagModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miTagModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miTagModeHeaderText = Ptr::new_box(MI_HeaderText::new("Tag Mode Menu", 320, 5));


            this.mModeSettingsMenu[9].add_control(ctl_ptr(this.miTagModeTagOnTouchField), ctl_ptr(this.miTagModeBackButton), ctl_ptr(this.miTagModeBackButton), Ptr::null(), ctl_ptr(this.miTagModeBackButton));
            this.mModeSettingsMenu[9].add_control(ctl_ptr(this.miTagModeBackButton), ctl_ptr(this.miTagModeTagOnTouchField), ctl_ptr(this.miTagModeTagOnTouchField), ctl_ptr(this.miTagModeTagOnTouchField), Ptr::null());

            this.mModeSettingsMenu[9].add_non_control(ctl_ptr(this.miTagModeLeftHeaderBar));
            this.mModeSettingsMenu[9].add_non_control(ctl_ptr(this.miTagModeRightHeaderBar));
            this.mModeSettingsMenu[9].add_non_control(ctl_ptr(this.miTagModeHeaderText));

            this.mModeSettingsMenu[9].set_initial_focus(ctl_ptr(this.miTagModeTagOnTouchField));
            this.mModeSettingsMenu[9].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Star Mode Settings
            //***********************

            this.miStarModeTimeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Time", 400, 150));
            this.miStarModeTimeField.add_random("5 Seconds", 5, false);
            this.miStarModeTimeField.add_random("10 Seconds", 10, false);
            this.miStarModeTimeField.add("15 Seconds", 15);
            this.miStarModeTimeField.add("20 Seconds", 20);
            this.miStarModeTimeField.add("25 Seconds", 25);
            this.miStarModeTimeField.add("30 Seconds", 30);
            this.miStarModeTimeField.add("35 Seconds", 35);
            this.miStarModeTimeField.add("40 Seconds", 40);
            this.miStarModeTimeField.add_random("45 Seconds", 45, false);
            this.miStarModeTimeField.add_random("50 Seconds", 50, false);
            this.miStarModeTimeField.add_random("55 Seconds", 55, false);
            this.miStarModeTimeField.add_random("60 Seconds", 60, false);
            this.miStarModeTimeField.set_output_ptr(&mut game_values.gamemodemenusettings.star.time as *mut _);
            this.miStarModeTimeField.set_current_value(game_values.gamemodemenusettings.star.time);

            this.miStarModeShineField = Ptr::new_box(MI_SelectField::<StarStyle>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Star Type", 400, 150));
            this.miStarModeShineField.add("Ztar", StarStyle::Ztar);
            this.miStarModeShineField.add("Shine", StarStyle::Shine);
            this.miStarModeShineField.add("Multi Star", StarStyle::Multi);
            this.miStarModeShineField.add("Random", StarStyle::Random);
            this.miStarModeShineField.set_output_ptr(&mut game_values.gamemodemenusettings.star.shine as *mut _);
            this.miStarModeShineField.set_current_value(game_values.gamemodemenusettings.star.shine);

            this.miStarModePercentExtraTime = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 260, "Extra Time", 400, 150, 384));
            this.miStarModePercentExtraTime.add_random("0", 0, false);
            this.miStarModePercentExtraTime.add_random("5", 5, false);
            this.miStarModePercentExtraTime.add("10", 10);
            this.miStarModePercentExtraTime.add("15", 15);
            this.miStarModePercentExtraTime.add("20", 20);
            this.miStarModePercentExtraTime.add("25", 25);
            this.miStarModePercentExtraTime.add("30", 30);
            this.miStarModePercentExtraTime.add("35", 35);
            this.miStarModePercentExtraTime.add("40", 40);
            this.miStarModePercentExtraTime.add("45", 45);
            this.miStarModePercentExtraTime.add("50", 50);
            this.miStarModePercentExtraTime.add_random("55", 55, false);
            this.miStarModePercentExtraTime.add_random("60", 60, false);
            this.miStarModePercentExtraTime.add_random("65", 65, false);
            this.miStarModePercentExtraTime.add_random("70", 70, false);
            this.miStarModePercentExtraTime.add_random("75", 75, false);
            this.miStarModePercentExtraTime.add_random("80", 80, false);
            this.miStarModePercentExtraTime.add_random("85", 85, false);
            this.miStarModePercentExtraTime.add_random("90", 90, false);
            this.miStarModePercentExtraTime.add_random("95", 95, false);
            this.miStarModePercentExtraTime.add_random("100", 100, false);
            this.miStarModePercentExtraTime.set_output_ptr(&mut game_values.gamemodemenusettings.star.percentextratime as *mut _);
            this.miStarModePercentExtraTime.set_current_value(game_values.gamemodemenusettings.star.percentextratime);
            this.miStarModePercentExtraTime.allow_wrap(false);

            this.miStarModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miStarModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miStarModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miStarModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miStarModeHeaderText = Ptr::new_box(MI_HeaderText::new("Star Mode Menu", 320, 5));

            this.mModeSettingsMenu[10].add_control(ctl_ptr(this.miStarModeTimeField), ctl_ptr(this.miStarModeBackButton), ctl_ptr(this.miStarModeShineField), Ptr::null(), ctl_ptr(this.miStarModeBackButton));
            this.mModeSettingsMenu[10].add_control(ctl_ptr(this.miStarModeShineField), ctl_ptr(this.miStarModeTimeField), ctl_ptr(this.miStarModePercentExtraTime), Ptr::null(), ctl_ptr(this.miStarModeBackButton));
            this.mModeSettingsMenu[10].add_control(ctl_ptr(this.miStarModePercentExtraTime), ctl_ptr(this.miStarModeShineField), ctl_ptr(this.miStarModeBackButton), Ptr::null(), ctl_ptr(this.miStarModeBackButton));
            this.mModeSettingsMenu[10].add_control(ctl_ptr(this.miStarModeBackButton), ctl_ptr(this.miStarModePercentExtraTime), ctl_ptr(this.miStarModeTimeField), ctl_ptr(this.miStarModePercentExtraTime), Ptr::null());

            this.mModeSettingsMenu[10].add_non_control(ctl_ptr(this.miStarModeLeftHeaderBar));
            this.mModeSettingsMenu[10].add_non_control(ctl_ptr(this.miStarModeRightHeaderBar));
            this.mModeSettingsMenu[10].add_non_control(ctl_ptr(this.miStarModeHeaderText));

            this.mModeSettingsMenu[10].set_initial_focus(ctl_ptr(this.miStarModeTimeField));
            this.mModeSettingsMenu[10].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Domination Mode Settings
            //***********************

            this.miDominationModeQuantityField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 120, "Quantity", 400, 180));
            this.miDominationModeQuantityField.add_random("1 Base", 1, false);
            this.miDominationModeQuantityField.add_random("2 Bases", 2, false);
            this.miDominationModeQuantityField.add("3 Bases", 3);
            this.miDominationModeQuantityField.add("4 Bases", 4);
            this.miDominationModeQuantityField.add("5 Bases", 5);
            this.miDominationModeQuantityField.add_random("6 Bases", 6, false);
            this.miDominationModeQuantityField.add_random("7 Bases", 7, false);
            this.miDominationModeQuantityField.add_random("8 Bases", 8, false);
            this.miDominationModeQuantityField.add_random("9 Bases", 9, false);
            this.miDominationModeQuantityField.add_random("10 Bases", 10, false);
            this.miDominationModeQuantityField.add_random("# Players - 1", 11, false);
            this.miDominationModeQuantityField.add_random("# Players", 12, false);
            this.miDominationModeQuantityField.add("# Players + 1", 13);
            this.miDominationModeQuantityField.add("# Players + 2", 14);
            this.miDominationModeQuantityField.add("# Players + 3", 15);
            this.miDominationModeQuantityField.add("# Players + 4", 16);
            this.miDominationModeQuantityField.add("# Players + 5", 17);
            this.miDominationModeQuantityField.add_random("# Players + 6", 18, false);
            this.miDominationModeQuantityField.add_random("2x Players - 3", 19, false);
            this.miDominationModeQuantityField.add_random("2x Players - 2", 20, false);
            this.miDominationModeQuantityField.add("2x Players - 1", 21);
            this.miDominationModeQuantityField.add("2x Players", 22);
            this.miDominationModeQuantityField.add_random("2x Players + 1", 23, false);
            this.miDominationModeQuantityField.add_random("2x Players + 2", 24, false);
            this.miDominationModeQuantityField.set_output_ptr(&mut game_values.gamemodemenusettings.domination.quantity as *mut _);
            this.miDominationModeQuantityField.set_current_value(game_values.gamemodemenusettings.domination.quantity);

            this.miDominationModeRelocateFrequencyField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 160, "Relocate", 400, 180));
            this.miDominationModeRelocateFrequencyField.add("Never", 0);
            this.miDominationModeRelocateFrequencyField.add_random("5 Seconds", 310, false);
            this.miDominationModeRelocateFrequencyField.add("10 Seconds", 620);
            this.miDominationModeRelocateFrequencyField.add("15 Seconds", 930);
            this.miDominationModeRelocateFrequencyField.add("20 Seconds", 1240);
            this.miDominationModeRelocateFrequencyField.add("30 Seconds", 1860);
            this.miDominationModeRelocateFrequencyField.add("45 Seconds", 2790);
            this.miDominationModeRelocateFrequencyField.add("1 Minute", 3720);
            this.miDominationModeRelocateFrequencyField.add("1.5 Minutes", 5580);
            this.miDominationModeRelocateFrequencyField.add("2 Minutes", 7440);
            this.miDominationModeRelocateFrequencyField.add("2.5 Minutes", 9300);
            this.miDominationModeRelocateFrequencyField.add("3 Minutes", 11160);
            this.miDominationModeRelocateFrequencyField.set_output_ptr(&mut game_values.gamemodemenusettings.domination.relocationfrequency as *mut _);
            this.miDominationModeRelocateFrequencyField.set_current_value(game_values.gamemodemenusettings.domination.relocationfrequency);

            this.miDominationModeDeathText = Ptr::new_box(MI_Text::new("On Death", 120, 210, 0, true, TextAlign::LEFT));

            this.miDominationModeLoseOnDeathField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Lose Bases", 400, 180));
            this.miDominationModeLoseOnDeathField.add("Off", false);
            this.miDominationModeLoseOnDeathField.add("On", true);
            this.miDominationModeLoseOnDeathField.set_output_ptr(&mut game_values.gamemodemenusettings.domination.loseondeath as *mut _);
            this.miDominationModeLoseOnDeathField.set_current_value(game_values.gamemodemenusettings.domination.loseondeath);
            this.miDominationModeLoseOnDeathField.set_auto_advance(true);

            this.miDominationModeRelocateOnDeathField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 280, "Move Bases", 400, 180));
            this.miDominationModeRelocateOnDeathField.add("Off", false);
            this.miDominationModeRelocateOnDeathField.add("On", true);
            this.miDominationModeRelocateOnDeathField.set_output_ptr(&mut game_values.gamemodemenusettings.domination.relocateondeath as *mut _);
            this.miDominationModeRelocateOnDeathField.set_current_value(game_values.gamemodemenusettings.domination.relocateondeath);
            this.miDominationModeRelocateOnDeathField.set_auto_advance(true);

            this.miDominationModeStealOnDeathField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 320, "Steal Bases", 400, 180));
            this.miDominationModeStealOnDeathField.add("Off", false);
            this.miDominationModeStealOnDeathField.add("On", true);
            this.miDominationModeStealOnDeathField.set_output_ptr(&mut game_values.gamemodemenusettings.domination.stealondeath as *mut _);
            this.miDominationModeStealOnDeathField.set_current_value(game_values.gamemodemenusettings.domination.stealondeath);
            this.miDominationModeStealOnDeathField.set_auto_advance(true);


            this.miDominationModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miDominationModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miDominationModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miDominationModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miDominationModeHeaderText = Ptr::new_box(MI_HeaderText::new("Domination Mode Menu", 320, 5));

            this.mModeSettingsMenu[11].add_control(ctl_ptr(this.miDominationModeQuantityField), ctl_ptr(this.miDominationModeBackButton), ctl_ptr(this.miDominationModeRelocateFrequencyField), Ptr::null(), ctl_ptr(this.miDominationModeBackButton));
            this.mModeSettingsMenu[11].add_control(ctl_ptr(this.miDominationModeRelocateFrequencyField), ctl_ptr(this.miDominationModeQuantityField), ctl_ptr(this.miDominationModeLoseOnDeathField), Ptr::null(), ctl_ptr(this.miDominationModeBackButton));
            this.mModeSettingsMenu[11].add_control(ctl_ptr(this.miDominationModeLoseOnDeathField), ctl_ptr(this.miDominationModeRelocateFrequencyField), ctl_ptr(this.miDominationModeRelocateOnDeathField), Ptr::null(), ctl_ptr(this.miDominationModeBackButton));
            this.mModeSettingsMenu[11].add_control(ctl_ptr(this.miDominationModeRelocateOnDeathField), ctl_ptr(this.miDominationModeLoseOnDeathField), ctl_ptr(this.miDominationModeStealOnDeathField), Ptr::null(), ctl_ptr(this.miDominationModeBackButton));
            this.mModeSettingsMenu[11].add_control(ctl_ptr(this.miDominationModeStealOnDeathField), ctl_ptr(this.miDominationModeRelocateOnDeathField), ctl_ptr(this.miDominationModeBackButton), Ptr::null(), ctl_ptr(this.miDominationModeBackButton));

            this.mModeSettingsMenu[11].add_control(ctl_ptr(this.miDominationModeBackButton), ctl_ptr(this.miDominationModeStealOnDeathField), ctl_ptr(this.miDominationModeQuantityField), ctl_ptr(this.miDominationModeStealOnDeathField), Ptr::null());

            this.mModeSettingsMenu[11].add_non_control(ctl_ptr(this.miDominationModeLeftHeaderBar));
            this.mModeSettingsMenu[11].add_non_control(ctl_ptr(this.miDominationModeRightHeaderBar));
            this.mModeSettingsMenu[11].add_non_control(ctl_ptr(this.miDominationModeHeaderText));
            this.mModeSettingsMenu[11].add_non_control(ctl_ptr(this.miDominationModeDeathText));

            this.mModeSettingsMenu[11].set_initial_focus(ctl_ptr(this.miDominationModeQuantityField));
            this.mModeSettingsMenu[11].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // King of the Hill Mode Settings
            //***********************

            this.miKingOfTheHillModeSizeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Size", 400, 180));
            this.miKingOfTheHillModeSizeField.add("2 x 2", 2);
            this.miKingOfTheHillModeSizeField.add("3 x 3", 3);
            this.miKingOfTheHillModeSizeField.add("4 x 4", 4);
            this.miKingOfTheHillModeSizeField.add("5 x 5", 5);
            this.miKingOfTheHillModeSizeField.add("6 x 6", 6);
            this.miKingOfTheHillModeSizeField.add("7 x 7", 7);
            this.miKingOfTheHillModeSizeField.set_output_ptr(&mut game_values.gamemodemenusettings.kingofthehill.areasize as *mut _);
            this.miKingOfTheHillModeSizeField.set_current_value(game_values.gamemodemenusettings.kingofthehill.areasize);

            this.miKingOfTheHillModeRelocateFrequencyField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Relocate", 400, 180));
            this.miKingOfTheHillModeRelocateFrequencyField.add("Never", 0);
            this.miKingOfTheHillModeRelocateFrequencyField.add_random("5 Seconds", 310, false);
            this.miKingOfTheHillModeRelocateFrequencyField.add("10 Seconds", 620);
            this.miKingOfTheHillModeRelocateFrequencyField.add("15 Seconds", 930);
            this.miKingOfTheHillModeRelocateFrequencyField.add("20 Seconds", 1240);
            this.miKingOfTheHillModeRelocateFrequencyField.add("30 Seconds", 1860);
            this.miKingOfTheHillModeRelocateFrequencyField.add("45 Seconds", 2790);
            this.miKingOfTheHillModeRelocateFrequencyField.add("1 Minute", 3720);
            this.miKingOfTheHillModeRelocateFrequencyField.add("1.5 Minutes", 5580);
            this.miKingOfTheHillModeRelocateFrequencyField.add("2 Minutes", 7440);
            this.miKingOfTheHillModeRelocateFrequencyField.add("2.5 Minutes", 9300);
            this.miKingOfTheHillModeRelocateFrequencyField.add("3 Minutes", 11160);
            this.miKingOfTheHillModeRelocateFrequencyField.set_output_ptr(&mut game_values.gamemodemenusettings.kingofthehill.relocationfrequency as *mut _);
            this.miKingOfTheHillModeRelocateFrequencyField.set_current_value(game_values.gamemodemenusettings.kingofthehill.relocationfrequency);

            this.miKingOfTheHillModeMultiplierField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 260, "Max Multiplier", 400, 180));
            this.miKingOfTheHillModeMultiplierField.add("None", 1);
            this.miKingOfTheHillModeMultiplierField.add("2", 2);
            this.miKingOfTheHillModeMultiplierField.add("3", 3);
            this.miKingOfTheHillModeMultiplierField.add("4", 4);
            this.miKingOfTheHillModeMultiplierField.add("5", 5);
            this.miKingOfTheHillModeMultiplierField.set_output_ptr(&mut game_values.gamemodemenusettings.kingofthehill.maxmultiplier as *mut _);
            this.miKingOfTheHillModeMultiplierField.set_current_value(game_values.gamemodemenusettings.kingofthehill.maxmultiplier);

            this.miKingOfTheHillModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miKingOfTheHillModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miKingOfTheHillModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miKingOfTheHillModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miKingOfTheHillModeHeaderText = Ptr::new_box(MI_HeaderText::new("King of the Hill Mode Menu", 320, 5));

            this.mModeSettingsMenu[12].add_control(ctl_ptr(this.miKingOfTheHillModeSizeField), ctl_ptr(this.miKingOfTheHillModeBackButton), ctl_ptr(this.miKingOfTheHillModeRelocateFrequencyField), Ptr::null(), ctl_ptr(this.miKingOfTheHillModeBackButton));
            this.mModeSettingsMenu[12].add_control(ctl_ptr(this.miKingOfTheHillModeRelocateFrequencyField), ctl_ptr(this.miKingOfTheHillModeSizeField), ctl_ptr(this.miKingOfTheHillModeMultiplierField), Ptr::null(), ctl_ptr(this.miKingOfTheHillModeBackButton));
            this.mModeSettingsMenu[12].add_control(ctl_ptr(this.miKingOfTheHillModeMultiplierField), ctl_ptr(this.miKingOfTheHillModeRelocateFrequencyField), ctl_ptr(this.miKingOfTheHillModeBackButton), Ptr::null(), ctl_ptr(this.miKingOfTheHillModeBackButton));

            this.mModeSettingsMenu[12].add_control(ctl_ptr(this.miKingOfTheHillModeBackButton), ctl_ptr(this.miKingOfTheHillModeMultiplierField), ctl_ptr(this.miKingOfTheHillModeSizeField), ctl_ptr(this.miKingOfTheHillModeMultiplierField), Ptr::null());

            this.mModeSettingsMenu[12].add_non_control(ctl_ptr(this.miKingOfTheHillModeLeftHeaderBar));
            this.mModeSettingsMenu[12].add_non_control(ctl_ptr(this.miKingOfTheHillModeRightHeaderBar));
            this.mModeSettingsMenu[12].add_non_control(ctl_ptr(this.miKingOfTheHillModeHeaderText));

            this.mModeSettingsMenu[12].set_initial_focus(ctl_ptr(this.miKingOfTheHillModeSizeField));
            this.mModeSettingsMenu[12].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Race Mode Settings
            //***********************

            this.miRaceModeQuantityField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Quantity", 400, 180));
            this.miRaceModeQuantityField.add_random("2", 2, false);
            this.miRaceModeQuantityField.add("3", 3);
            this.miRaceModeQuantityField.add("4", 4);
            this.miRaceModeQuantityField.add("5", 5);
            this.miRaceModeQuantityField.add("6", 6);
            this.miRaceModeQuantityField.add("7", 7);
            this.miRaceModeQuantityField.add("8", MAXRACEGOALS as i16);
            this.miRaceModeQuantityField.set_output_ptr(&mut game_values.gamemodemenusettings.race.quantity as *mut _);
            this.miRaceModeQuantityField.set_current_value(game_values.gamemodemenusettings.race.quantity);

            this.miRaceModeSpeedField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Speed", 400, 180));
            this.miRaceModeSpeedField.add("Stationary", 0);
            this.miRaceModeSpeedField.add("Very Slow", 2);
            this.miRaceModeSpeedField.add("Slow", 3);
            this.miRaceModeSpeedField.add("Moderate", 4);
            this.miRaceModeSpeedField.add("Fast", 6);
            this.miRaceModeSpeedField.add_random("Very Fast", 8, false);
            this.miRaceModeSpeedField.add_random("Extremely Fast", 15, false);
            this.miRaceModeSpeedField.add_random("Insanely Fast", 30, false);
            this.miRaceModeSpeedField.set_output_ptr(&mut game_values.gamemodemenusettings.race.speed as *mut _);
            this.miRaceModeSpeedField.set_current_value(game_values.gamemodemenusettings.race.speed);

            this.miRaceModePenaltyField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 260, "Penalty", 400, 180));
            this.miRaceModePenaltyField.add("None", 0);
            this.miRaceModePenaltyField.add("One Goal", 1);
            this.miRaceModePenaltyField.add("All Goals", 2);
            this.miRaceModePenaltyField.set_output_ptr(&mut game_values.gamemodemenusettings.race.penalty as *mut _);
            this.miRaceModePenaltyField.set_current_value(game_values.gamemodemenusettings.race.penalty);

            this.miRaceModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miRaceModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miRaceModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miRaceModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miRaceModeHeaderText = Ptr::new_box(MI_HeaderText::new("Race Mode Menu", 320, 5));

            this.mModeSettingsMenu[13].add_control(ctl_ptr(this.miRaceModeQuantityField), ctl_ptr(this.miRaceModeBackButton), ctl_ptr(this.miRaceModeSpeedField), Ptr::null(), ctl_ptr(this.miRaceModeBackButton));
            this.mModeSettingsMenu[13].add_control(ctl_ptr(this.miRaceModeSpeedField), ctl_ptr(this.miRaceModeQuantityField), ctl_ptr(this.miRaceModePenaltyField), Ptr::null(), ctl_ptr(this.miRaceModeBackButton));
            this.mModeSettingsMenu[13].add_control(ctl_ptr(this.miRaceModePenaltyField), ctl_ptr(this.miRaceModeSpeedField), ctl_ptr(this.miRaceModeBackButton), Ptr::null(), ctl_ptr(this.miRaceModeBackButton));

            this.mModeSettingsMenu[13].add_control(ctl_ptr(this.miRaceModeBackButton), ctl_ptr(this.miRaceModePenaltyField), ctl_ptr(this.miRaceModeQuantityField), ctl_ptr(this.miRaceModePenaltyField), Ptr::null());

            this.mModeSettingsMenu[13].add_non_control(ctl_ptr(this.miRaceModeLeftHeaderBar));
            this.mModeSettingsMenu[13].add_non_control(ctl_ptr(this.miRaceModeRightHeaderBar));
            this.mModeSettingsMenu[13].add_non_control(ctl_ptr(this.miRaceModeHeaderText));

            this.mModeSettingsMenu[13].set_initial_focus(ctl_ptr(this.miRaceModeQuantityField));
            this.mModeSettingsMenu[13].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Frenzy Mode Settings
            //***********************

            this.miFrenzyModeOptions = Ptr::new_box(MI_FrenzyModeOptions::new(50, 44, 640, 7));
            this.miFrenzyModeOptions.set_auto_modify(true);

            this.miFrenzyModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miFrenzyModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miFrenzyModeHeaderText = Ptr::new_box(MI_HeaderText::new("Frenzy Mode Menu", 320, 5));

            this.mModeSettingsMenu[15].add_control(ctl_ptr(this.miFrenzyModeOptions), Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());

            this.mModeSettingsMenu[15].add_non_control(ctl_ptr(this.miFrenzyModeLeftHeaderBar));
            this.mModeSettingsMenu[15].add_non_control(ctl_ptr(this.miFrenzyModeRightHeaderBar));
            this.mModeSettingsMenu[15].add_non_control(ctl_ptr(this.miFrenzyModeHeaderText));

            this.mModeSettingsMenu[15].set_initial_focus(ctl_ptr(this.miFrenzyModeOptions));
            this.mModeSettingsMenu[15].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Survival Mode Settings
            //***********************

            for iEnemy in 0..NUMSURVIVALENEMIES as i16 {
                this.miSurvivalModeEnemySlider[iEnemy as usize] = Ptr::new_box(MI_PowerupSlider::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), Ptr::from_mut(&mut rm.menu_survival), 120, 120 + 40 * iEnemy, 400, iEnemy));
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 0);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 1);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 2);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 3);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 4);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 5);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 6);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 7);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 8);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 9);
                this.miSurvivalModeEnemySlider[iEnemy as usize].add("", 10);
                this.miSurvivalModeEnemySlider[iEnemy as usize].allow_wrap(false);
                this.miSurvivalModeEnemySlider[iEnemy as usize].set_output_ptr(&mut game_values.gamemodemenusettings.survival.enemyweight[iEnemy as usize] as *mut _);
                this.miSurvivalModeEnemySlider[iEnemy as usize].set_current_value(game_values.gamemodemenusettings.survival.enemyweight[iEnemy as usize]);
            }

            this.miSurvivalModeDensityField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Density", 400, 180));
            this.miSurvivalModeDensityField.add_random("Very Low", 40, false);
            this.miSurvivalModeDensityField.add("Low", 30);
            this.miSurvivalModeDensityField.add("Medium", 20);
            this.miSurvivalModeDensityField.add("High", 15);
            this.miSurvivalModeDensityField.add_random("Very High", 10, false);
            this.miSurvivalModeDensityField.add("Extremely High", 6);
            this.miSurvivalModeDensityField.add_random("Insanely High", 2, false);
            this.miSurvivalModeDensityField.set_output_ptr(&mut game_values.gamemodemenusettings.survival.density as *mut _);
            this.miSurvivalModeDensityField.set_current_value(game_values.gamemodemenusettings.survival.density);

            this.miSurvivalModeSpeedField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 280, "Speed", 400, 180));
            this.miSurvivalModeSpeedField.add_random("Very Slow", 2, false);
            this.miSurvivalModeSpeedField.add("Slow", 3);
            this.miSurvivalModeSpeedField.add("Moderate", 4);
            this.miSurvivalModeSpeedField.add("Fast", 6);
            this.miSurvivalModeSpeedField.add_random("Very Fast", 8, false);
            this.miSurvivalModeSpeedField.add_random("Extremely Fast", 15, false);
            this.miSurvivalModeSpeedField.add_random("Insanely Fast", 30, false);
            this.miSurvivalModeSpeedField.set_output_ptr(&mut game_values.gamemodemenusettings.survival.speed as *mut _);
            this.miSurvivalModeSpeedField.set_current_value(game_values.gamemodemenusettings.survival.speed);

            this.miSurvivalModeShieldField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 320, "Shield", 400, 180));
            this.miSurvivalModeShieldField.add("Off", false);
            this.miSurvivalModeShieldField.add("On", true);
            this.miSurvivalModeShieldField.set_output_ptr(&mut game_values.gamemodemenusettings.survival.shield as *mut _);
            this.miSurvivalModeShieldField.set_current_value(game_values.gamemodemenusettings.survival.shield);
            this.miSurvivalModeShieldField.set_auto_advance(true);

            this.miSurvivalModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miSurvivalModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miSurvivalModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miSurvivalModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miSurvivalModeHeaderText = Ptr::new_box(MI_HeaderText::new("Survival Mode Menu", 320, 5));

            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeEnemySlider[0]), ctl_ptr(this.miSurvivalModeBackButton), ctl_ptr(this.miSurvivalModeEnemySlider[1]), Ptr::null(), ctl_ptr(this.miSurvivalModeBackButton));
            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeEnemySlider[1]), ctl_ptr(this.miSurvivalModeEnemySlider[0]), ctl_ptr(this.miSurvivalModeEnemySlider[2]), Ptr::null(), ctl_ptr(this.miSurvivalModeBackButton));
            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeEnemySlider[2]), ctl_ptr(this.miSurvivalModeEnemySlider[1]), ctl_ptr(this.miSurvivalModeDensityField), Ptr::null(), ctl_ptr(this.miSurvivalModeBackButton));
            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeDensityField), ctl_ptr(this.miSurvivalModeEnemySlider[2]), ctl_ptr(this.miSurvivalModeSpeedField), Ptr::null(), ctl_ptr(this.miSurvivalModeBackButton));
            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeSpeedField), ctl_ptr(this.miSurvivalModeDensityField), ctl_ptr(this.miSurvivalModeShieldField), Ptr::null(), ctl_ptr(this.miSurvivalModeBackButton));
            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeShieldField), ctl_ptr(this.miSurvivalModeSpeedField), ctl_ptr(this.miSurvivalModeBackButton), Ptr::null(), ctl_ptr(this.miSurvivalModeBackButton));
            this.mModeSettingsMenu[16].add_control(ctl_ptr(this.miSurvivalModeBackButton), ctl_ptr(this.miSurvivalModeShieldField), ctl_ptr(this.miSurvivalModeEnemySlider[0]), ctl_ptr(this.miSurvivalModeShieldField), Ptr::null());

            this.mModeSettingsMenu[16].add_non_control(ctl_ptr(this.miSurvivalModeLeftHeaderBar));
            this.mModeSettingsMenu[16].add_non_control(ctl_ptr(this.miSurvivalModeRightHeaderBar));
            this.mModeSettingsMenu[16].add_non_control(ctl_ptr(this.miSurvivalModeHeaderText));

            this.mModeSettingsMenu[16].set_initial_focus(ctl_ptr(this.miSurvivalModeEnemySlider[0]));
            this.mModeSettingsMenu[16].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Greed Mode Settings
            //***********************
            this.miGreedModeCoinLife = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 160, "Coin Life", 400, 150));
            this.miGreedModeCoinLife.add_random("1 Second", 62, false);
            this.miGreedModeCoinLife.add_random("2 Seconds", 124, false);
            this.miGreedModeCoinLife.add("3 Seconds", 186);
            this.miGreedModeCoinLife.add("4 Seconds", 248);
            this.miGreedModeCoinLife.add("5 Seconds", 310);
            this.miGreedModeCoinLife.add("6 Seconds", 372);
            this.miGreedModeCoinLife.add("7 Seconds", 434);
            this.miGreedModeCoinLife.add("8 Seconds", 496);
            this.miGreedModeCoinLife.add("9 Seconds", 558);
            this.miGreedModeCoinLife.add("10 Seconds", 620);
            this.miGreedModeCoinLife.add("12 Seconds", 744);
            this.miGreedModeCoinLife.add("15 Seconds", 930);
            this.miGreedModeCoinLife.add("18 Seconds", 1116);
            this.miGreedModeCoinLife.add("20 Seconds", 1240);
            this.miGreedModeCoinLife.add("25 Seconds", 1550);
            this.miGreedModeCoinLife.add("30 Seconds", 1860);
            this.miGreedModeCoinLife.set_output_ptr(&mut game_values.gamemodemenusettings.greed.coinlife as *mut _);
            this.miGreedModeCoinLife.set_current_value(game_values.gamemodemenusettings.greed.coinlife);

            this.miGreedModeOwnCoins = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "Own Coins", 400, 150));
            this.miGreedModeOwnCoins.add("Yes", true);
            this.miGreedModeOwnCoins.add("No", false);
            this.miGreedModeOwnCoins.set_output_ptr(&mut game_values.gamemodemenusettings.greed.owncoins as *mut _);
            this.miGreedModeOwnCoins.set_current_value(game_values.gamemodemenusettings.greed.owncoins);
            this.miGreedModeOwnCoins.set_auto_advance(true);

            this.miGreedModeMultiplier = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Multipler", 400, 150));
            this.miGreedModeMultiplier.add_random("0.5", 1, false);
            this.miGreedModeMultiplier.add("1", 2);
            this.miGreedModeMultiplier.add("1.5", 3);
            this.miGreedModeMultiplier.add("2", 4);
            this.miGreedModeMultiplier.add_random("2.5", 5, false);
            this.miGreedModeMultiplier.add_random("3", 6, false);
            this.miGreedModeMultiplier.set_output_ptr(&mut game_values.gamemodemenusettings.greed.multiplier as *mut _);
            this.miGreedModeMultiplier.set_current_value(game_values.gamemodemenusettings.greed.multiplier);

            this.miGreedModePercentExtraCoin = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 280, "Extra Coins", 400, 150, 384));
            this.miGreedModePercentExtraCoin.add_random("0", 0, false);
            this.miGreedModePercentExtraCoin.add_random("5", 5, false);
            this.miGreedModePercentExtraCoin.add("10", 10);
            this.miGreedModePercentExtraCoin.add("15", 15);
            this.miGreedModePercentExtraCoin.add("20", 20);
            this.miGreedModePercentExtraCoin.add("25", 25);
            this.miGreedModePercentExtraCoin.add("30", 30);
            this.miGreedModePercentExtraCoin.add("35", 35);
            this.miGreedModePercentExtraCoin.add("40", 40);
            this.miGreedModePercentExtraCoin.add("45", 45);
            this.miGreedModePercentExtraCoin.add("50", 50);
            this.miGreedModePercentExtraCoin.add_random("55", 55, false);
            this.miGreedModePercentExtraCoin.add_random("60", 60, false);
            this.miGreedModePercentExtraCoin.add_random("65", 65, false);
            this.miGreedModePercentExtraCoin.add_random("70", 70, false);
            this.miGreedModePercentExtraCoin.add_random("75", 75, false);
            this.miGreedModePercentExtraCoin.add_random("80", 80, false);
            this.miGreedModePercentExtraCoin.add_random("85", 85, false);
            this.miGreedModePercentExtraCoin.add_random("90", 90, false);
            this.miGreedModePercentExtraCoin.add_random("95", 95, false);
            this.miGreedModePercentExtraCoin.add_random("100", 100, false);
            this.miGreedModePercentExtraCoin.set_output_ptr(&mut game_values.gamemodemenusettings.greed.percentextracoin as *mut _);
            this.miGreedModePercentExtraCoin.set_current_value(game_values.gamemodemenusettings.greed.percentextracoin);
            this.miGreedModePercentExtraCoin.allow_wrap(false);

            this.miGreedModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miGreedModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miGreedModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miGreedModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miGreedModeHeaderText = Ptr::new_box(MI_HeaderText::new("Greed Mode Menu", 320, 5));

            this.mModeSettingsMenu[17].add_control(ctl_ptr(this.miGreedModeCoinLife), ctl_ptr(this.miGreedModeBackButton), ctl_ptr(this.miGreedModeOwnCoins), Ptr::null(), ctl_ptr(this.miGreedModeBackButton));
            this.mModeSettingsMenu[17].add_control(ctl_ptr(this.miGreedModeOwnCoins), ctl_ptr(this.miGreedModeCoinLife), ctl_ptr(this.miGreedModeMultiplier), Ptr::null(), ctl_ptr(this.miGreedModeBackButton));
            this.mModeSettingsMenu[17].add_control(ctl_ptr(this.miGreedModeMultiplier), ctl_ptr(this.miGreedModeOwnCoins), ctl_ptr(this.miGreedModePercentExtraCoin), Ptr::null(), ctl_ptr(this.miGreedModeBackButton));
            this.mModeSettingsMenu[17].add_control(ctl_ptr(this.miGreedModePercentExtraCoin), ctl_ptr(this.miGreedModeMultiplier), ctl_ptr(this.miGreedModeBackButton), Ptr::null(), ctl_ptr(this.miGreedModeBackButton));

            this.mModeSettingsMenu[17].add_control(ctl_ptr(this.miGreedModeBackButton), ctl_ptr(this.miGreedModePercentExtraCoin), ctl_ptr(this.miGreedModeCoinLife), ctl_ptr(this.miGreedModePercentExtraCoin), Ptr::null());

            this.mModeSettingsMenu[17].add_non_control(ctl_ptr(this.miGreedModeLeftHeaderBar));
            this.mModeSettingsMenu[17].add_non_control(ctl_ptr(this.miGreedModeRightHeaderBar));
            this.mModeSettingsMenu[17].add_non_control(ctl_ptr(this.miGreedModeHeaderText));

            this.mModeSettingsMenu[17].set_initial_focus(ctl_ptr(this.miGreedModeCoinLife));
            this.mModeSettingsMenu[17].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Health Mode Settings
            //***********************
            this.miHealthModeStartLife = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Start Life", 400, 150));
            this.miHealthModeStartLife.add_random("2", 2, false);
            this.miHealthModeStartLife.add_random("3", 3, false);
            this.miHealthModeStartLife.add("4", 4);
            this.miHealthModeStartLife.add("5", 5);
            this.miHealthModeStartLife.add("6", 6);
            this.miHealthModeStartLife.add_random("7", 7, false);
            this.miHealthModeStartLife.add_random("8", 8, false);
            this.miHealthModeStartLife.add_random("9", 9, false);
            this.miHealthModeStartLife.add_random("10", 10, false);
            this.miHealthModeStartLife.set_output_ptr(&mut game_values.gamemodemenusettings.health.startlife as *mut _);
            this.miHealthModeStartLife.set_current_value(game_values.gamemodemenusettings.health.startlife);
            this.miHealthModeStartLife.allow_wrap(false);
            this.miHealthModeStartLife.set_item_changed_code(MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED);

            this.miHealthModeMaxLife = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Max Life", 400, 150));
            this.miHealthModeMaxLife.add_random("2", 2, false);
            this.miHealthModeMaxLife.add_random("3", 3, false);
            this.miHealthModeMaxLife.add_random("4", 4, false);
            this.miHealthModeMaxLife.add_random("5", 5, false);
            this.miHealthModeMaxLife.add("6", 6);
            this.miHealthModeMaxLife.add("7", 7);
            this.miHealthModeMaxLife.add("8", 8);
            this.miHealthModeMaxLife.add("9", 9);
            this.miHealthModeMaxLife.add("10", 10);
            this.miHealthModeMaxLife.set_output_ptr(&mut game_values.gamemodemenusettings.health.maxlife as *mut _);
            this.miHealthModeMaxLife.set_current_value(game_values.gamemodemenusettings.health.maxlife);
            this.miHealthModeMaxLife.allow_wrap(false);
            this.miHealthModeMaxLife.set_item_changed_code(MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED);

            this.miHealthModePercentExtraLife = Ptr::new_box(MI_SliderField::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), 120, 260, "Extra Life", 400, 150, 384));
            this.miHealthModePercentExtraLife.add_random("0", 0, false);
            this.miHealthModePercentExtraLife.add_random("5", 5, false);
            this.miHealthModePercentExtraLife.add("10", 10);
            this.miHealthModePercentExtraLife.add("15", 15);
            this.miHealthModePercentExtraLife.add("20", 20);
            this.miHealthModePercentExtraLife.add("25", 25);
            this.miHealthModePercentExtraLife.add("30", 30);
            this.miHealthModePercentExtraLife.add("35", 35);
            this.miHealthModePercentExtraLife.add("40", 40);
            this.miHealthModePercentExtraLife.add("45", 45);
            this.miHealthModePercentExtraLife.add("50", 50);
            this.miHealthModePercentExtraLife.add_random("55", 55, false);
            this.miHealthModePercentExtraLife.add_random("60", 60, false);
            this.miHealthModePercentExtraLife.add_random("65", 65, false);
            this.miHealthModePercentExtraLife.add_random("70", 70, false);
            this.miHealthModePercentExtraLife.add_random("75", 75, false);
            this.miHealthModePercentExtraLife.add_random("80", 80, false);
            this.miHealthModePercentExtraLife.add_random("85", 85, false);
            this.miHealthModePercentExtraLife.add_random("90", 90, false);
            this.miHealthModePercentExtraLife.add_random("95", 95, false);
            this.miHealthModePercentExtraLife.add_random("100", 100, false);
            this.miHealthModePercentExtraLife.set_output_ptr(&mut game_values.gamemodemenusettings.health.percentextralife as *mut _);
            this.miHealthModePercentExtraLife.set_current_value(game_values.gamemodemenusettings.health.percentextralife);
            this.miHealthModePercentExtraLife.allow_wrap(false);

            this.miHealthModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miHealthModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miHealthModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miHealthModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miHealthModeHeaderText = Ptr::new_box(MI_HeaderText::new("Health Mode Menu", 320, 5));

            this.mModeSettingsMenu[18].add_control(ctl_ptr(this.miHealthModeStartLife), ctl_ptr(this.miHealthModeBackButton), ctl_ptr(this.miHealthModeMaxLife), Ptr::null(), ctl_ptr(this.miHealthModeBackButton));
            this.mModeSettingsMenu[18].add_control(ctl_ptr(this.miHealthModeMaxLife), ctl_ptr(this.miHealthModeStartLife), ctl_ptr(this.miHealthModePercentExtraLife), Ptr::null(), ctl_ptr(this.miHealthModeBackButton));
            this.mModeSettingsMenu[18].add_control(ctl_ptr(this.miHealthModePercentExtraLife), ctl_ptr(this.miHealthModeMaxLife), ctl_ptr(this.miHealthModeBackButton), Ptr::null(), ctl_ptr(this.miHealthModeBackButton));
            this.mModeSettingsMenu[18].add_control(ctl_ptr(this.miHealthModeBackButton), ctl_ptr(this.miHealthModePercentExtraLife), ctl_ptr(this.miHealthModeStartLife), ctl_ptr(this.miHealthModePercentExtraLife), Ptr::null());

            this.mModeSettingsMenu[18].add_non_control(ctl_ptr(this.miHealthModeLeftHeaderBar));
            this.mModeSettingsMenu[18].add_non_control(ctl_ptr(this.miHealthModeRightHeaderBar));
            this.mModeSettingsMenu[18].add_non_control(ctl_ptr(this.miHealthModeHeaderText));

            this.mModeSettingsMenu[18].set_initial_focus(ctl_ptr(this.miHealthModeStartLife));
            this.mModeSettingsMenu[18].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Card Collection Mode Settings
            //***********************

            this.miCollectionModeQuantityField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 160, "Limit", 400, 180));
            this.miCollectionModeQuantityField.add("1 Card", 1);
            this.miCollectionModeQuantityField.add("2 Cards", 2);
            this.miCollectionModeQuantityField.add("3 Cards", 3);
            this.miCollectionModeQuantityField.add_random("4 Cards", 4, false);
            this.miCollectionModeQuantityField.add_random("5 Cards", 5, false);
            this.miCollectionModeQuantityField.add("# Players - 1", 6);
            this.miCollectionModeQuantityField.add("# Players", 7);
            this.miCollectionModeQuantityField.add_random("# Players + 1", 8, false);
            this.miCollectionModeQuantityField.add_random("# Players + 2", 9, false);
            this.miCollectionModeQuantityField.add_random("# Players + 3", 10, false);
            this.miCollectionModeQuantityField.set_output_ptr(&mut game_values.gamemodemenusettings.collection.quantity as *mut _);
            this.miCollectionModeQuantityField.set_current_value(game_values.gamemodemenusettings.collection.quantity);

            this.miCollectionModeRateField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 200, "Rate", 400, 180));
            this.miCollectionModeRateField.add("Instant", 0);
            this.miCollectionModeRateField.add("1 Second", 62);
            this.miCollectionModeRateField.add("2 Seconds", 124);
            this.miCollectionModeRateField.add("3 Seconds", 186);
            this.miCollectionModeRateField.add("5 Seconds", 310);
            this.miCollectionModeRateField.add_random("10 Seconds", 620, false);
            this.miCollectionModeRateField.add_random("15 Seconds", 930, false);
            this.miCollectionModeRateField.add_random("20 Seconds", 1240, false);
            this.miCollectionModeRateField.add_random("25 Seconds", 1550, false);
            this.miCollectionModeRateField.add_random("30 Seconds", 1860, false);
            this.miCollectionModeRateField.set_output_ptr(&mut game_values.gamemodemenusettings.collection.rate as *mut _);
            this.miCollectionModeRateField.set_current_value(game_values.gamemodemenusettings.collection.rate);

            this.miCollectionModeBankTimeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 240, "Bank Time", 400, 180));
            this.miCollectionModeBankTimeField.add_random("Instant", 0, false);
            this.miCollectionModeBankTimeField.add_random("1 Second", 62, false);
            this.miCollectionModeBankTimeField.add("2 Seconds", 124);
            this.miCollectionModeBankTimeField.add("3 Seconds", 186);
            this.miCollectionModeBankTimeField.add("4 Seconds", 248);
            this.miCollectionModeBankTimeField.add("5 Seconds", 310);
            this.miCollectionModeBankTimeField.add_random("6 Seconds", 372, false);
            this.miCollectionModeBankTimeField.add_random("7 Seconds", 434, false);
            this.miCollectionModeBankTimeField.add_random("8 Seconds", 496, false);
            this.miCollectionModeBankTimeField.add_random("9 Seconds", 558, false);
            this.miCollectionModeBankTimeField.add_random("10 Seconds", 620, false);
            this.miCollectionModeBankTimeField.set_output_ptr(&mut game_values.gamemodemenusettings.collection.banktime as *mut _);
            this.miCollectionModeBankTimeField.set_current_value(game_values.gamemodemenusettings.collection.banktime);

            this.miCollectionModeCardLifeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 280, "Card Life", 400, 180));
            this.miCollectionModeCardLifeField.add_random("1 Second", 62, false);
            this.miCollectionModeCardLifeField.add_random("2 Seconds", 124, false);
            this.miCollectionModeCardLifeField.add("3 Seconds", 186);
            this.miCollectionModeCardLifeField.add("4 Seconds", 248);
            this.miCollectionModeCardLifeField.add("5 Seconds", 310);
            this.miCollectionModeCardLifeField.add("6 Seconds", 372);
            this.miCollectionModeCardLifeField.add("7 Seconds", 434);
            this.miCollectionModeCardLifeField.add("8 Seconds", 496);
            this.miCollectionModeCardLifeField.add("9 Seconds", 558);
            this.miCollectionModeCardLifeField.add("10 Seconds", 620);
            this.miCollectionModeCardLifeField.add("12 Seconds", 744);
            this.miCollectionModeCardLifeField.add("15 Seconds", 930);
            this.miCollectionModeCardLifeField.add("18 Seconds", 1116);
            this.miCollectionModeCardLifeField.add("20 Seconds", 1240);
            this.miCollectionModeCardLifeField.add("25 Seconds", 1550);
            this.miCollectionModeCardLifeField.add("30 Seconds", 1860);
            this.miCollectionModeCardLifeField.set_output_ptr(&mut game_values.gamemodemenusettings.collection.cardlife as *mut _);
            this.miCollectionModeCardLifeField.set_current_value(game_values.gamemodemenusettings.collection.cardlife);

            this.miCollectionModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miCollectionModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miCollectionModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miCollectionModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miCollectionModeHeaderText = Ptr::new_box(MI_HeaderText::new("Card Collection Mode Menu", 320, 5));

            this.mModeSettingsMenu[19].add_control(ctl_ptr(this.miCollectionModeQuantityField), ctl_ptr(this.miCollectionModeBackButton), ctl_ptr(this.miCollectionModeRateField), Ptr::null(), ctl_ptr(this.miCollectionModeBackButton));
            this.mModeSettingsMenu[19].add_control(ctl_ptr(this.miCollectionModeRateField), ctl_ptr(this.miCollectionModeQuantityField), ctl_ptr(this.miCollectionModeBankTimeField), Ptr::null(), ctl_ptr(this.miCollectionModeBackButton));
            this.mModeSettingsMenu[19].add_control(ctl_ptr(this.miCollectionModeBankTimeField), ctl_ptr(this.miCollectionModeRateField), ctl_ptr(this.miCollectionModeCardLifeField), Ptr::null(), ctl_ptr(this.miCollectionModeBackButton));
            this.mModeSettingsMenu[19].add_control(ctl_ptr(this.miCollectionModeCardLifeField), ctl_ptr(this.miCollectionModeBankTimeField), ctl_ptr(this.miCollectionModeBackButton), Ptr::null(), ctl_ptr(this.miCollectionModeBackButton));
            this.mModeSettingsMenu[19].add_control(ctl_ptr(this.miCollectionModeBackButton), ctl_ptr(this.miCollectionModeCardLifeField), ctl_ptr(this.miCollectionModeQuantityField), ctl_ptr(this.miCollectionModeCardLifeField), Ptr::null());

            this.mModeSettingsMenu[19].add_non_control(ctl_ptr(this.miCollectionModeLeftHeaderBar));
            this.mModeSettingsMenu[19].add_non_control(ctl_ptr(this.miCollectionModeRightHeaderBar));
            this.mModeSettingsMenu[19].add_non_control(ctl_ptr(this.miCollectionModeHeaderText));

            this.mModeSettingsMenu[19].set_initial_focus(ctl_ptr(this.miCollectionModeQuantityField));
            this.mModeSettingsMenu[19].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            //***********************
            // Chase Mode Settings
            //***********************

            this.miChaseModeSpeedField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 160, "Speed", 400, 180));
            this.miChaseModeSpeedField.add_random("Very Slow", 3, false);
            this.miChaseModeSpeedField.add("Slow", 4);
            this.miChaseModeSpeedField.add("Moderate", 5);
            this.miChaseModeSpeedField.add("Fast", 6);
            this.miChaseModeSpeedField.add("Very Fast", 7);
            this.miChaseModeSpeedField.add_random("Extremely Fast", 8, false);
            this.miChaseModeSpeedField.add_random("Insanely Fast", 10, false);
            this.miChaseModeSpeedField.set_output_ptr(&mut game_values.gamemodemenusettings.chase.phantospeed as *mut _);
            this.miChaseModeSpeedField.set_current_value(game_values.gamemodemenusettings.chase.phantospeed);

            for iPhanto in 0..3 {
                this.miChaseModeQuantitySlider[iPhanto as usize] = Ptr::new_box(MI_PowerupSlider::new(Ptr::from_mut(&mut rm.spr_selectfield), Ptr::from_mut(&mut rm.menu_slider_bar), Ptr::from_mut(&mut rm.spr_phanto), 120, 200 + 40 * iPhanto, 400, iPhanto));
                this.miChaseModeQuantitySlider[iPhanto as usize].add_random("", 0, !(iPhanto == 0));
                this.miChaseModeQuantitySlider[iPhanto as usize].add("", 1);
                this.miChaseModeQuantitySlider[iPhanto as usize].add("", 2);
                this.miChaseModeQuantitySlider[iPhanto as usize].add_random("", 3, false);
                this.miChaseModeQuantitySlider[iPhanto as usize].add_random("", 4, false);
                this.miChaseModeQuantitySlider[iPhanto as usize].add_random("", 5, false);
                this.miChaseModeQuantitySlider[iPhanto as usize].allow_wrap(false);
                this.miChaseModeQuantitySlider[iPhanto as usize].set_output_ptr(&mut game_values.gamemodemenusettings.chase.phantoquantity[iPhanto as usize] as *mut _);
                this.miChaseModeQuantitySlider[iPhanto as usize].set_current_value(game_values.gamemodemenusettings.chase.phantoquantity[iPhanto as usize]);
            }

            this.miChaseModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miChaseModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miChaseModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miChaseModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miChaseModeHeaderText = Ptr::new_box(MI_HeaderText::new("Phanto Mode Menu", 320, 5));

            this.mModeSettingsMenu[20].add_control(ctl_ptr(this.miChaseModeSpeedField), ctl_ptr(this.miChaseModeBackButton), ctl_ptr(this.miChaseModeQuantitySlider[0]), Ptr::null(), ctl_ptr(this.miChaseModeBackButton));

            this.mModeSettingsMenu[20].add_control(ctl_ptr(this.miChaseModeQuantitySlider[0]), ctl_ptr(this.miChaseModeSpeedField), ctl_ptr(this.miChaseModeQuantitySlider[1]), Ptr::null(), ctl_ptr(this.miChaseModeBackButton));
            this.mModeSettingsMenu[20].add_control(ctl_ptr(this.miChaseModeQuantitySlider[1]), ctl_ptr(this.miChaseModeQuantitySlider[0]), ctl_ptr(this.miChaseModeQuantitySlider[2]), Ptr::null(), ctl_ptr(this.miChaseModeBackButton));
            this.mModeSettingsMenu[20].add_control(ctl_ptr(this.miChaseModeQuantitySlider[2]), ctl_ptr(this.miChaseModeQuantitySlider[1]), ctl_ptr(this.miChaseModeBackButton), Ptr::null(), ctl_ptr(this.miChaseModeBackButton));

            this.mModeSettingsMenu[20].add_control(ctl_ptr(this.miChaseModeBackButton), ctl_ptr(this.miChaseModeQuantitySlider[2]), ctl_ptr(this.miChaseModeSpeedField), ctl_ptr(this.miChaseModeQuantitySlider[2]), Ptr::null());

            this.mModeSettingsMenu[20].add_non_control(ctl_ptr(this.miChaseModeLeftHeaderBar));
            this.mModeSettingsMenu[20].add_non_control(ctl_ptr(this.miChaseModeRightHeaderBar));
            this.mModeSettingsMenu[20].add_non_control(ctl_ptr(this.miChaseModeHeaderText));

            this.mModeSettingsMenu[20].set_initial_focus(ctl_ptr(this.miChaseModeSpeedField));
            this.mModeSettingsMenu[20].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Shyguy Tag Mode Settings
            //***********************

            this.miShyGuyTagModeTagOnSuicideField = Ptr::new_box(MI_SelectField::<bool>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Suicide Tag", 400, 180));
            this.miShyGuyTagModeTagOnSuicideField.add("Off", false);
            this.miShyGuyTagModeTagOnSuicideField.add("On", true);
            this.miShyGuyTagModeTagOnSuicideField.set_output_ptr(&mut game_values.gamemodemenusettings.shyguytag.tagonsuicide as *mut _);
            this.miShyGuyTagModeTagOnSuicideField.set_current_value(game_values.gamemodemenusettings.shyguytag.tagonsuicide);
            this.miShyGuyTagModeTagOnSuicideField.set_auto_advance(true);

            this.miShyGuyTagModeTagOnStompField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Tag Transfer", 400, 180));
            this.miShyGuyTagModeTagOnStompField.add("Touch Only", 0);
            this.miShyGuyTagModeTagOnStompField.add("Kills Only", 1);
            this.miShyGuyTagModeTagOnStompField.add("Touch and Kills", 2);
            this.miShyGuyTagModeTagOnStompField.set_output_ptr(&mut game_values.gamemodemenusettings.shyguytag.tagtransfer as *mut _);
            this.miShyGuyTagModeTagOnStompField.set_current_value(game_values.gamemodemenusettings.shyguytag.tagtransfer);

            this.miShyGuyTagModeFreeTimeField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 260, "Free Time", 400, 180));
            this.miShyGuyTagModeFreeTimeField.add("Instant", 0);
            this.miShyGuyTagModeFreeTimeField.add("1 Second", 1);
            this.miShyGuyTagModeFreeTimeField.add("2 Seconds", 2);
            this.miShyGuyTagModeFreeTimeField.add("3 Seconds", 3);
            this.miShyGuyTagModeFreeTimeField.add("4 Seconds", 4);
            this.miShyGuyTagModeFreeTimeField.add("5 Seconds", 5);
            this.miShyGuyTagModeFreeTimeField.add("6 Seconds", 6);
            this.miShyGuyTagModeFreeTimeField.add("7 Seconds", 7);
            this.miShyGuyTagModeFreeTimeField.add("8 Seconds", 8);
            this.miShyGuyTagModeFreeTimeField.add("9 Seconds", 9);
            this.miShyGuyTagModeFreeTimeField.add("10 Seconds", 10);
            this.miShyGuyTagModeFreeTimeField.set_output_ptr(&mut game_values.gamemodemenusettings.shyguytag.freetime as *mut _);
            this.miShyGuyTagModeFreeTimeField.set_current_value(game_values.gamemodemenusettings.shyguytag.freetime);

            this.miShyGuyTagModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miShyGuyTagModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miShyGuyTagModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miShyGuyTagModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miShyGuyTagModeHeaderText = Ptr::new_box(MI_HeaderText::new("Shyguy Tag Mode Menu", 320, 5));

            this.mModeSettingsMenu[21].add_control(ctl_ptr(this.miShyGuyTagModeTagOnSuicideField), ctl_ptr(this.miShyGuyTagModeBackButton), ctl_ptr(this.miShyGuyTagModeTagOnStompField), Ptr::null(), ctl_ptr(this.miShyGuyTagModeBackButton));
            this.mModeSettingsMenu[21].add_control(ctl_ptr(this.miShyGuyTagModeTagOnStompField), ctl_ptr(this.miShyGuyTagModeTagOnSuicideField), ctl_ptr(this.miShyGuyTagModeFreeTimeField), Ptr::null(), ctl_ptr(this.miShyGuyTagModeBackButton));
            this.mModeSettingsMenu[21].add_control(ctl_ptr(this.miShyGuyTagModeFreeTimeField), ctl_ptr(this.miShyGuyTagModeTagOnStompField), ctl_ptr(this.miShyGuyTagModeBackButton), Ptr::null(), ctl_ptr(this.miShyGuyTagModeBackButton));
            this.mModeSettingsMenu[21].add_control(ctl_ptr(this.miShyGuyTagModeBackButton), ctl_ptr(this.miShyGuyTagModeFreeTimeField), ctl_ptr(this.miShyGuyTagModeTagOnSuicideField), ctl_ptr(this.miShyGuyTagModeFreeTimeField), Ptr::null());

            this.mModeSettingsMenu[21].add_non_control(ctl_ptr(this.miShyGuyTagModeLeftHeaderBar));
            this.mModeSettingsMenu[21].add_non_control(ctl_ptr(this.miShyGuyTagModeRightHeaderBar));
            this.mModeSettingsMenu[21].add_non_control(ctl_ptr(this.miShyGuyTagModeHeaderText));

            this.mModeSettingsMenu[21].set_initial_focus(ctl_ptr(this.miShyGuyTagModeTagOnSuicideField));
            this.mModeSettingsMenu[21].set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);


            //***********************
            // Boss Mode Settings
            //***********************

            this.miBossModeTypeField = Ptr::new_box(MI_SelectField::<Boss>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 180, "Type", 400, 180));
            this.miBossModeTypeField.add("Hammer", Boss::Hammer);
            this.miBossModeTypeField.add("Bomb", Boss::Bomb);
            this.miBossModeTypeField.add("Fire", Boss::Fire);
            this.miBossModeTypeField.set_output_ptr(&mut game_values.gamemodemenusettings.boss.bosstype as *mut _);
            this.miBossModeTypeField.set_current_value(game_values.gamemodemenusettings.boss.bosstype);

            this.miBossModeDifficultyField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 220, "Difficulty", 400, 180));
            this.miBossModeDifficultyField.add_random("Very Easy", 0, false);
            this.miBossModeDifficultyField.add("Easy", 1);
            this.miBossModeDifficultyField.add("Moderate", 2);
            this.miBossModeDifficultyField.add("Hard", 3);
            this.miBossModeDifficultyField.add_random("Very Hard", 4, false);
            this.miBossModeDifficultyField.set_output_ptr(&mut game_values.gamemodemenusettings.boss.difficulty as *mut _);
            this.miBossModeDifficultyField.set_current_value(game_values.gamemodemenusettings.boss.difficulty);

            this.miBossModeHitPointsField = Ptr::new_box(MI_SelectField::<i16>::new(Ptr::from_mut(&mut rm.spr_selectfield), 120, 260, "Health", 400, 180));
            this.miBossModeHitPointsField.add_random("1", 1, false);
            this.miBossModeHitPointsField.add_random("2", 2, false);
            this.miBossModeHitPointsField.add("3", 3);
            this.miBossModeHitPointsField.add("4", 4);
            this.miBossModeHitPointsField.add("5", 5);
            this.miBossModeHitPointsField.add("6", 6);
            this.miBossModeHitPointsField.add("7", 7);
            this.miBossModeHitPointsField.add("8", 8);
            this.miBossModeHitPointsField.add_random("9", 9, false);
            this.miBossModeHitPointsField.add_random("10", 10, false);
            this.miBossModeHitPointsField.add_random("11", 11, false);
            this.miBossModeHitPointsField.add_random("12", 12, false);
            this.miBossModeHitPointsField.add_random("13", 13, false);
            this.miBossModeHitPointsField.add_random("14", 14, false);
            this.miBossModeHitPointsField.add_random("15", 15, false);
            this.miBossModeHitPointsField.add_random("16", 16, false);
            this.miBossModeHitPointsField.add_random("17", 17, false);
            this.miBossModeHitPointsField.add_random("18", 18, false);
            this.miBossModeHitPointsField.add_random("19", 19, false);
            this.miBossModeHitPointsField.add_random("20", 20, false);
            this.miBossModeHitPointsField.set_output_ptr(&mut game_values.gamemodemenusettings.boss.hitpoints as *mut _);
            this.miBossModeHitPointsField.set_current_value(game_values.gamemodemenusettings.boss.hitpoints);

            this.miBossModeBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            this.miBossModeBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miBossModeLeftHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miBossModeRightHeaderBar = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miBossModeHeaderText = Ptr::new_box(MI_HeaderText::new("Boss Minigame Menu", 320, 5));

            this.mBossSettingsMenu.add_control(ctl_ptr(this.miBossModeTypeField), ctl_ptr(this.miBossModeBackButton), ctl_ptr(this.miBossModeDifficultyField), Ptr::null(), ctl_ptr(this.miBossModeBackButton));
            this.mBossSettingsMenu.add_control(ctl_ptr(this.miBossModeDifficultyField), ctl_ptr(this.miBossModeTypeField), ctl_ptr(this.miBossModeHitPointsField), Ptr::null(), ctl_ptr(this.miBossModeBackButton));
            this.mBossSettingsMenu.add_control(ctl_ptr(this.miBossModeHitPointsField), ctl_ptr(this.miBossModeDifficultyField), ctl_ptr(this.miBossModeBackButton), Ptr::null(), ctl_ptr(this.miBossModeBackButton));
            this.mBossSettingsMenu.add_control(ctl_ptr(this.miBossModeBackButton), ctl_ptr(this.miBossModeHitPointsField), ctl_ptr(this.miBossModeTypeField), ctl_ptr(this.miBossModeHitPointsField), Ptr::null());

            this.mBossSettingsMenu.add_non_control(ctl_ptr(this.miBossModeLeftHeaderBar));
            this.mBossSettingsMenu.add_non_control(ctl_ptr(this.miBossModeRightHeaderBar));
            this.mBossSettingsMenu.add_non_control(ctl_ptr(this.miBossModeHeaderText));

            this.mBossSettingsMenu.set_initial_focus(ctl_ptr(this.miBossModeTypeField));
            this.mBossSettingsMenu.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);
        }
        this
    }

    pub fn set_controlling_team(&mut self, iControlTeam: i16) {
        for iMode in 0..GAMEMODE_LAST as i16 {
            self.mModeSettingsMenu[iMode as usize].set_controlling_team(iControlTeam);
        }
    }

    pub fn set_random_game_mode_settings(&mut self, iMode: i16) {
        unsafe {
            let iMode = iMode as GameModeType;
            if iMode == game_mode_classic {
                // classic
                game_values.gamemodesettings.classic.style = self.miClassicModeStyleField.random_value();
                game_values.gamemodesettings.classic.scoring = self.miClassicModeScoringField.random_value();
            } else if iMode == game_mode_frag {
                // frag
                game_values.gamemodesettings.frag.style = self.miFragModeStyleField.random_value();
                game_values.gamemodesettings.frag.scoring = self.miFragModeScoringField.random_value();
            } else if iMode == game_mode_timelimit {
                // time
                game_values.gamemodesettings.time.style = self.miTimeLimitModeStyleField.random_value();
                game_values.gamemodesettings.time.scoring = self.miTimeLimitModeScoringField.random_value();
                game_values.gamemodesettings.time.percentextratime = self.miTimeLimitModePercentExtraTime.current_value();
            } else if iMode == game_mode_jail {
                // jail
                game_values.gamemodesettings.jail.style = self.miJailModeStyleField.random_value();
                game_values.gamemodesettings.jail.timetofree = self.miJailModeTimeFreeField.random_value();
                game_values.gamemodesettings.jail.tagfree = self.miJailModeTagFreeField.random_value();
                game_values.gamemodesettings.jail.percentkey = self.miJailModeJailKeyField.current_value();
            } else if iMode == game_mode_coins {
                // coins
                game_values.gamemodesettings.coins.penalty = self.miCoinModePenaltyField.random_value();
                game_values.gamemodesettings.coins.quantity = self.miCoinModeQuantityField.random_value();
                game_values.gamemodesettings.coins.percentextracoin = self.miCoinModePercentExtraCoin.current_value();
            } else if iMode == game_mode_stomp {
                // stomp
                game_values.gamemodesettings.stomp.rate = self.miStompModeRateField.random_value();

                for iEnemy in 0..NUMSTOMPENEMIES as usize {
                    game_values.gamemodesettings.stomp.enemyweight[iEnemy] = self.miStompModeEnemySlider[iEnemy].current_value();
                }
            } else if iMode == game_mode_eggs {
                // egg
                for iEgg in 0..4 {
                    game_values.gamemodesettings.egg.eggs[iEgg] = self.miEggModeEggQuantityField[iEgg].current_value();
                }

                for iYoshi in 0..4 {
                    game_values.gamemodesettings.egg.yoshis[iYoshi] = self.miEggModeYoshiQuantityField[iYoshi].current_value();
                }

                game_values.gamemodesettings.egg.explode = self.miEggModeExplosionTimeField.random_value();
            } else if iMode == game_mode_ctf {
                // capture the flag
                game_values.gamemodesettings.flag.speed = self.miFlagModeSpeedField.current_value();
                game_values.gamemodesettings.flag.touchreturn = self.miFlagModeTouchReturnField.random_value();
                game_values.gamemodesettings.flag.pointmove = self.miFlagModePointMoveField.random_value();
                game_values.gamemodesettings.flag.autoreturn = self.miFlagModeAutoReturnField.random_value();
                game_values.gamemodesettings.flag.homescore = self.miFlagModeHomeScoreField.random_value();
                game_values.gamemodesettings.flag.centerflag = self.miFlagModeCenterFlagField.random_value();
            } else if iMode == game_mode_chicken {
                // chicken
                game_values.gamemodesettings.chicken.usetarget = self.miChickenModeShowTargetField.random_value();
                game_values.gamemodesettings.chicken.glide = self.miChickenModeGlideField.random_value();
            } else if iMode == game_mode_tag {
                // tag
                game_values.gamemodesettings.tag.tagontouch = self.miTagModeTagOnTouchField.random_value();
            } else if iMode == game_mode_star {
                // star
                game_values.gamemodesettings.star.time = self.miStarModeTimeField.random_value();
                game_values.gamemodesettings.star.shine = self.miStarModeShineField.random_value();
                game_values.gamemodesettings.star.percentextratime = self.miStarModePercentExtraTime.current_value();
            } else if iMode == game_mode_domination {
                // domination
                game_values.gamemodesettings.domination.quantity = self.miDominationModeQuantityField.random_value();
                game_values.gamemodesettings.domination.loseondeath = self.miDominationModeLoseOnDeathField.random_value();
                game_values.gamemodesettings.domination.relocateondeath = self.miDominationModeRelocateOnDeathField.random_value();
                game_values.gamemodesettings.domination.stealondeath = self.miDominationModeStealOnDeathField.random_value();
                game_values.gamemodesettings.domination.relocationfrequency = self.miDominationModeRelocateFrequencyField.random_value();
            } else if iMode == game_mode_koth {
                // king of the hill
                game_values.gamemodesettings.kingofthehill.areasize = self.miKingOfTheHillModeSizeField.random_value();
                game_values.gamemodesettings.kingofthehill.relocationfrequency = self.miKingOfTheHillModeRelocateFrequencyField.random_value();
                game_values.gamemodesettings.kingofthehill.maxmultiplier = self.miKingOfTheHillModeMultiplierField.random_value();
            } else if iMode == game_mode_race {
                // race
                game_values.gamemodesettings.race.quantity = self.miRaceModeQuantityField.random_value();
                game_values.gamemodesettings.race.speed = self.miRaceModeSpeedField.random_value();
                game_values.gamemodesettings.race.penalty = self.miRaceModePenaltyField.random_value();
            } else if iMode == game_mode_frenzy {
                // frenzy
                self.miFrenzyModeOptions.set_random_game_mode_settings();
            } else if iMode == game_mode_survival {
                // survival
                game_values.gamemodesettings.survival.density = self.miSurvivalModeDensityField.random_value();
                game_values.gamemodesettings.survival.speed = self.miSurvivalModeSpeedField.random_value();
                game_values.gamemodesettings.survival.shield = self.miSurvivalModeShieldField.random_value();

                for iEnemy in 0..NUMSURVIVALENEMIES as usize {
                    game_values.gamemodesettings.survival.enemyweight[iEnemy] = self.miSurvivalModeEnemySlider[iEnemy].current_value();
                }
            } else if iMode == game_mode_greed {
                // greed
                game_values.gamemodesettings.greed.coinlife = self.miGreedModeCoinLife.random_value();
                game_values.gamemodesettings.greed.owncoins = self.miGreedModeOwnCoins.random_value();
                game_values.gamemodesettings.greed.multiplier = self.miGreedModeMultiplier.random_value();
                game_values.gamemodesettings.greed.percentextracoin = self.miGreedModePercentExtraCoin.current_value();
            } else if iMode == game_mode_health {
                // health
                game_values.gamemodesettings.health.startlife = self.miHealthModeStartLife.random_value();
                game_values.gamemodesettings.health.maxlife = self.miHealthModeMaxLife.random_value();
                game_values.gamemodesettings.health.percentextralife = self.miHealthModePercentExtraLife.current_value();
            } else if iMode == game_mode_collection {
                // card collection
                game_values.gamemodesettings.collection.quantity = self.miCollectionModeQuantityField.random_value();
                game_values.gamemodesettings.collection.rate = self.miCollectionModeRateField.random_value();
                game_values.gamemodesettings.collection.banktime = self.miCollectionModeBankTimeField.random_value();
                game_values.gamemodesettings.collection.cardlife = self.miCollectionModeCardLifeField.random_value();
            } else if iMode == game_mode_chase {
                // chase (phanto)
                game_values.gamemodesettings.chase.phantospeed = self.miChaseModeSpeedField.random_value();

                for iPhanto in 0..3 {
                    game_values.gamemodesettings.chase.phantoquantity[iPhanto] = self.miChaseModeQuantitySlider[iPhanto].current_value();
                }
            } else if iMode == game_mode_shyguytag {
                // shyguy tag
                game_values.gamemodesettings.shyguytag.tagonsuicide = self.miShyGuyTagModeTagOnSuicideField.random_value();
                game_values.gamemodesettings.shyguytag.tagtransfer = self.miShyGuyTagModeTagOnStompField.random_value();
                // freetime reads the tag transfer field, as in C++.
                game_values.gamemodesettings.shyguytag.freetime = self.miShyGuyTagModeTagOnStompField.random_value();
            } else if iMode == game_mode_boss_minigame {
                // boss
                game_values.gamemodesettings.boss.bosstype = self.miBossModeTypeField.random_value();
                game_values.gamemodesettings.boss.difficulty = self.miBossModeDifficultyField.random_value();
                game_values.gamemodesettings.boss.hitpoints = self.miBossModeHitPointsField.random_value();
            }
        }
    }

    pub fn get_options_menu(&mut self, iMode: i16) -> Ptr<UI_Menu> {
        Ptr::from_mut(&mut self.mModeSettingsMenu[iMode as usize])
    }
    pub fn get_boss_options_menu(&mut self) -> Ptr<UI_Menu> {
        Ptr::from_mut(&mut self.mBossSettingsMenu)
    }

    pub fn health_mode_start_life_changed(&mut self) {
        let iMaxLife: i16 = self.miHealthModeMaxLife.current_value();
        if self.miHealthModeStartLife.current_value() > iMaxLife {
            self.miHealthModeStartLife.set_current_value(iMaxLife);
        }
    }

    pub fn health_mode_max_life_changed(&mut self) {
        let iStartLife: i16 = self.miHealthModeStartLife.current_value();
        if self.miHealthModeMaxLife.current_value() < iStartLife {
            self.miHealthModeMaxLife.set_current_value(iStartLife);
        }
    }

    pub fn refresh(&mut self) {
        for iMode in 0..GAMEMODE_LAST as i16 {
            self.mModeSettingsMenu[iMode as usize].refresh();
        }

        self.mBossSettingsMenu.refresh();
    }
}
