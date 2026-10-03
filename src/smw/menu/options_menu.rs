//! Port of src/smw/menu/OptionsMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;

/*
    This is the main options menu, where you can select the
    settings category you want to modify, or regenerate the
    map cache.
*/
#[derive(Default)]
pub struct UI_OptionsMenu {
    pub ui_menu: UI_Menu,

    pub miGameplayOptionsMenuButton: Ptr<MI_Button>,
    pub miTeamOptionsMenuButton: Ptr<MI_Button>,
    pub miPowerupOptionsMenuButton: Ptr<MI_Button>,
    pub miProjectilesOptionsMenuButton: Ptr<MI_Button>,
    pub miProjectilesLimitsMenuButton: Ptr<MI_Button>,
    pub miPowerupSettingsMenuButton: Ptr<MI_Button>,
    pub miGraphicsOptionsMenuButton: Ptr<MI_Button>,
    pub miEyeCandyOptionsMenuButton: Ptr<MI_Button>,
    pub miSoundOptionsMenuButton: Ptr<MI_Button>,
    pub miGenerateMapThumbsButton: Ptr<MI_Button>,

    pub miOptionsMenuBackButton: Ptr<MI_Button>,

    pub miOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miOptionsMenuHeaderText: Ptr<MI_Text>,

    pub miGenerateThumbsDialogImage: Ptr<MI_Image>,
    pub miGenerateThumbsDialogAreYouText: Ptr<MI_Text>,
    pub miGenerateThumbsDialogSureText: Ptr<MI_Text>,
    pub miGenerateThumbsDialogYesButton: Ptr<MI_Button>,
    pub miGenerateThumbsDialogNoButton: Ptr<MI_Button>,
}
crate::impl_base!(UI_OptionsMenu => ui_menu: UI_Menu);

impl UI_OptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miGameplayOptionsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 40, "Gameplay", 400, TextAlign::CENTER));
            this.miGameplayOptionsMenuButton.set_code(MENU_CODE_TO_GAMEPLAY_OPTIONS_MENU);

            this.miTeamOptionsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 80, "Team", 400, TextAlign::CENTER));
            this.miTeamOptionsMenuButton.set_code(MENU_CODE_TO_TEAM_OPTIONS_MENU);

            this.miPowerupOptionsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 120, "Item Selection", 400, TextAlign::CENTER));
            this.miPowerupOptionsMenuButton.set_code(MENU_CODE_TO_POWERUP_SELECTION_MENU);

            this.miPowerupSettingsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 160, "Item Settings", 400, TextAlign::CENTER));
            this.miPowerupSettingsMenuButton.set_code(MENU_CODE_TO_POWERUP_SETTINGS_MENU);

            this.miProjectilesOptionsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 200, "Weapons & Projectiles", 400, TextAlign::CENTER));
            this.miProjectilesOptionsMenuButton.set_code(MENU_CODE_TO_PROJECTILES_OPTIONS_MENU);

            this.miProjectilesLimitsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 240, "Weapon Use Limits", 400, TextAlign::CENTER));
            this.miProjectilesLimitsMenuButton.set_code(MENU_CODE_TO_PROJECTILES_LIMITS_MENU);

            this.miGraphicsOptionsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 280, "Graphics", 400, TextAlign::CENTER));
            this.miGraphicsOptionsMenuButton.set_code(MENU_CODE_TO_GRAPHICS_OPTIONS_MENU);

            this.miEyeCandyOptionsMenuButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 320, "Eye Candy", 400, TextAlign::CENTER));
            this.miEyeCandyOptionsMenuButton.set_code(MENU_CODE_TO_EYECANDY_OPTIONS_MENU);

            let sound_spr = if game_values.soundcapable { spr_selectfield } else { Ptr::from_mut(&mut rm.spr_selectfielddisabled) };
            this.miSoundOptionsMenuButton = Ptr::new_box(MI_Button::new(sound_spr, 120, 360, "Music & Sound", 400, TextAlign::CENTER));

            if game_values.soundcapable {
                this.miSoundOptionsMenuButton.set_code(MENU_CODE_TO_SOUND_OPTIONS_MENU);
            }

            this.miGenerateMapThumbsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 400, "Refresh Maps", 400, TextAlign::CENTER));
            this.miGenerateMapThumbsButton.set_code(MENU_CODE_SAVE_ALL_MAP_THUMBNAILS);

            this.miOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miOptionsMenuBackButton.set_code(MENU_CODE_TO_MAIN_MENU);

            this.miOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Options Menu", 320, 5));

            this.miGenerateThumbsDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miGenerateThumbsDialogAreYouText = Ptr::new_box(MI_HeaderText::new("Are You", 320, 195));
            this.miGenerateThumbsDialogSureText = Ptr::new_box(MI_HeaderText::new("Sure?", 320, 220));
            this.miGenerateThumbsDialogYesButton = Ptr::new_box(MI_Button::new(spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER));
            this.miGenerateThumbsDialogNoButton = Ptr::new_box(MI_Button::new(spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER));
        }

        this.miGenerateThumbsDialogYesButton.set_code(MENU_CODE_GENERATE_THUMBS_RESET_YES);
        this.miGenerateThumbsDialogNoButton.set_code(MENU_CODE_GENERATE_THUMBS_RESET_NO);

        this.miGenerateThumbsDialogImage.set_visible(false);
        this.miGenerateThumbsDialogAreYouText.set_visible(false);
        this.miGenerateThumbsDialogSureText.set_visible(false);
        this.miGenerateThumbsDialogYesButton.set_visible(false);
        this.miGenerateThumbsDialogNoButton.set_visible(false);

        let gameplay = ctl_ptr(this.miGameplayOptionsMenuButton);
        let team = ctl_ptr(this.miTeamOptionsMenuButton);
        let powerup = ctl_ptr(this.miPowerupOptionsMenuButton);
        let powerup_settings = ctl_ptr(this.miPowerupSettingsMenuButton);
        let projectiles = ctl_ptr(this.miProjectilesOptionsMenuButton);
        let limits = ctl_ptr(this.miProjectilesLimitsMenuButton);
        let graphics = ctl_ptr(this.miGraphicsOptionsMenuButton);
        let eyecandy = ctl_ptr(this.miEyeCandyOptionsMenuButton);
        let sound = ctl_ptr(this.miSoundOptionsMenuButton);
        let thumbs = ctl_ptr(this.miGenerateMapThumbsButton);
        let back = ctl_ptr(this.miOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(gameplay, back, team, null, back);
        this.add_control(team, gameplay, powerup, null, back);
        this.add_control(powerup, team, powerup_settings, null, back);
        this.add_control(powerup_settings, powerup, projectiles, null, back);
        this.add_control(projectiles, powerup_settings, limits, null, back);
        this.add_control(limits, projectiles, graphics, null, back);
        this.add_control(graphics, limits, eyecandy, null, back);
        this.add_control(eyecandy, graphics, sound, null, back);
        this.add_control(sound, eyecandy, thumbs, null, back);
        this.add_control(thumbs, sound, back, null, back);

        this.add_control(back, thumbs, gameplay, thumbs, null);

        let c = ctl_ptr(this.miOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miOptionsMenuHeaderText);
        this.add_non_control(c);

        let c = ctl_ptr(this.miGenerateThumbsDialogImage);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGenerateThumbsDialogAreYouText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGenerateThumbsDialogSureText);
        this.add_non_control(c);

        let yes = ctl_ptr(this.miGenerateThumbsDialogYesButton);
        let no = ctl_ptr(this.miGenerateThumbsDialogNoButton);
        this.add_control(yes, null, null, null, no);
        this.add_control(no, null, null, yes, null);

        this.set_initial_focus(gameplay);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }

    pub fn show_thumbnails_popup(&mut self) {
        self.miGenerateThumbsDialogImage.set_visible(true);
        self.miGenerateThumbsDialogAreYouText.set_visible(true);
        self.miGenerateThumbsDialogSureText.set_visible(true);
        self.miGenerateThumbsDialogYesButton.set_visible(true);
        self.miGenerateThumbsDialogNoButton.set_visible(true);

        self.remember_current();

        let focus = ctl_ptr(self.miGenerateThumbsDialogNoButton);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_GENERATE_THUMBS_RESET_NO);
    }

    pub fn hide_thumbnails_popup(&mut self) {
        self.miGenerateThumbsDialogImage.set_visible(false);
        self.miGenerateThumbsDialogAreYouText.set_visible(false);
        self.miGenerateThumbsDialogSureText.set_visible(false);
        self.miGenerateThumbsDialogYesButton.set_visible(false);
        self.miGenerateThumbsDialogNoButton.set_visible(false);

        let focus = ctl_ptr(self.miGameplayOptionsMenuButton);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_TO_MAIN_MENU);

        self.restore_current();
    }
}
