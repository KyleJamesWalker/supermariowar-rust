//! Port of src/smw/menu/OptionsMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
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

    pub miGameplayOptionsMenuButton: Ctl<MI_Button>,
    pub miTeamOptionsMenuButton: Ctl<MI_Button>,
    pub miPowerupOptionsMenuButton: Ctl<MI_Button>,
    pub miProjectilesOptionsMenuButton: Ctl<MI_Button>,
    pub miProjectilesLimitsMenuButton: Ctl<MI_Button>,
    pub miPowerupSettingsMenuButton: Ctl<MI_Button>,
    pub miGraphicsOptionsMenuButton: Ctl<MI_Button>,
    pub miEyeCandyOptionsMenuButton: Ctl<MI_Button>,
    pub miSoundOptionsMenuButton: Ctl<MI_Button>,
    pub miGenerateMapThumbsButton: Ctl<MI_Button>,

    pub miOptionsMenuBackButton: Ctl<MI_Button>,

    pub miOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miOptionsMenuHeaderText: Ctl<MI_Text>,

    pub miGenerateThumbsDialogImage: Ctl<MI_Image>,
    pub miGenerateThumbsDialogAreYouText: Ctl<MI_Text>,
    pub miGenerateThumbsDialogSureText: Ctl<MI_Text>,
    pub miGenerateThumbsDialogYesButton: Ctl<MI_Button>,
    pub miGenerateThumbsDialogNoButton: Ctl<MI_Button>,
}
crate::impl_base!(UI_OptionsMenu => ui_menu: UI_Menu);

impl UI_OptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;
        let menu_plain_field = g.rm.menu_plain_field;

        this.miGameplayOptionsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 40, "Gameplay", 400, TextAlign::CENTER), g);
        g.ctl(this.miGameplayOptionsMenuButton).set_code(MENU_CODE_TO_GAMEPLAY_OPTIONS_MENU);

        this.miTeamOptionsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 80, "Team", 400, TextAlign::CENTER), g);
        g.ctl(this.miTeamOptionsMenuButton).set_code(MENU_CODE_TO_TEAM_OPTIONS_MENU);

        this.miPowerupOptionsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 120, "Item Selection", 400, TextAlign::CENTER), g);
        g.ctl(this.miPowerupOptionsMenuButton).set_code(MENU_CODE_TO_POWERUP_SELECTION_MENU);

        this.miPowerupSettingsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 160, "Item Settings", 400, TextAlign::CENTER), g);
        g.ctl(this.miPowerupSettingsMenuButton).set_code(MENU_CODE_TO_POWERUP_SETTINGS_MENU);

        this.miProjectilesOptionsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 200, "Weapons & Projectiles", 400, TextAlign::CENTER), g);
        g.ctl(this.miProjectilesOptionsMenuButton).set_code(MENU_CODE_TO_PROJECTILES_OPTIONS_MENU);

        this.miProjectilesLimitsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 240, "Weapon Use Limits", 400, TextAlign::CENTER), g);
        g.ctl(this.miProjectilesLimitsMenuButton).set_code(MENU_CODE_TO_PROJECTILES_LIMITS_MENU);

        this.miGraphicsOptionsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 280, "Graphics", 400, TextAlign::CENTER), g);
        g.ctl(this.miGraphicsOptionsMenuButton).set_code(MENU_CODE_TO_GRAPHICS_OPTIONS_MENU);

        this.miEyeCandyOptionsMenuButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 320, "Eye Candy", 400, TextAlign::CENTER), g);
        g.ctl(this.miEyeCandyOptionsMenuButton).set_code(MENU_CODE_TO_EYECANDY_OPTIONS_MENU);

        let sound_spr = if g.game_values.soundcapable { spr_selectfield } else { g.rm.spr_selectfielddisabled };
        this.miSoundOptionsMenuButton = Ctl::new(MI_Button::new(g, sound_spr, 120, 360, "Music & Sound", 400, TextAlign::CENTER), g);

        if g.game_values.soundcapable {
            g.ctl(this.miSoundOptionsMenuButton).set_code(MENU_CODE_TO_SOUND_OPTIONS_MENU);
        }

        this.miGenerateMapThumbsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 400, "Refresh Maps", 400, TextAlign::CENTER), g);
        g.ctl(this.miGenerateMapThumbsButton).set_code(MENU_CODE_SAVE_ALL_MAP_THUMBNAILS);

        this.miOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miOptionsMenuBackButton).set_code(MENU_CODE_TO_MAIN_MENU);

        this.miOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Options Menu", 320, 5), g);

        this.miGenerateThumbsDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miGenerateThumbsDialogAreYouText = Ctl::new(MI_HeaderText::new(g, "Are You", 320, 195), g);
        this.miGenerateThumbsDialogSureText = Ctl::new(MI_HeaderText::new(g, "Sure?", 320, 220), g);
        this.miGenerateThumbsDialogYesButton = Ctl::new(MI_Button::new(g, spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        this.miGenerateThumbsDialogNoButton = Ctl::new(MI_Button::new(g, spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);

        g.ctl(this.miGenerateThumbsDialogYesButton).set_code(MENU_CODE_GENERATE_THUMBS_RESET_YES);
        g.ctl(this.miGenerateThumbsDialogNoButton).set_code(MENU_CODE_GENERATE_THUMBS_RESET_NO);

        g.ctl(this.miGenerateThumbsDialogImage).set_visible(false);
        g.ctl(this.miGenerateThumbsDialogAreYouText).set_visible(false);
        g.ctl(this.miGenerateThumbsDialogSureText).set_visible(false);
        g.ctl(this.miGenerateThumbsDialogYesButton).set_visible(false);
        g.ctl(this.miGenerateThumbsDialogNoButton).set_visible(false);

        let gameplay = this.miGameplayOptionsMenuButton.id();
        let team = this.miTeamOptionsMenuButton.id();
        let powerup = this.miPowerupOptionsMenuButton.id();
        let powerup_settings = this.miPowerupSettingsMenuButton.id();
        let projectiles = this.miProjectilesOptionsMenuButton.id();
        let limits = this.miProjectilesLimitsMenuButton.id();
        let graphics = this.miGraphicsOptionsMenuButton.id();
        let eyecandy = this.miEyeCandyOptionsMenuButton.id();
        let sound = this.miSoundOptionsMenuButton.id();
        let thumbs = this.miGenerateMapThumbsButton.id();
        let back = this.miOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, gameplay, back, team, null, back);
        this.add_control(g, team, gameplay, powerup, null, back);
        this.add_control(g, powerup, team, powerup_settings, null, back);
        this.add_control(g, powerup_settings, powerup, projectiles, null, back);
        this.add_control(g, projectiles, powerup_settings, limits, null, back);
        this.add_control(g, limits, projectiles, graphics, null, back);
        this.add_control(g, graphics, limits, eyecandy, null, back);
        this.add_control(g, eyecandy, graphics, sound, null, back);
        this.add_control(g, sound, eyecandy, thumbs, null, back);
        this.add_control(g, thumbs, sound, back, null, back);

        this.add_control(g, back, thumbs, gameplay, thumbs, null);

        let c = this.miOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miOptionsMenuHeaderText.id();
        this.add_non_control(c);

        let c = this.miGenerateThumbsDialogImage.id();
        this.add_non_control(c);
        let c = this.miGenerateThumbsDialogAreYouText.id();
        this.add_non_control(c);
        let c = this.miGenerateThumbsDialogSureText.id();
        this.add_non_control(c);

        let yes = this.miGenerateThumbsDialogYesButton.id();
        let no = this.miGenerateThumbsDialogNoButton.id();
        this.add_control(g, yes, null, null, null, no);
        this.add_control(g, no, null, null, yes, null);

        this.set_initial_focus(g, gameplay);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }

    pub fn show_thumbnails_popup(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.ctl(self.miGenerateThumbsDialogImage).set_visible(true);
        g.ctl(self.miGenerateThumbsDialogAreYouText).set_visible(true);
        g.ctl(self.miGenerateThumbsDialogSureText).set_visible(true);
        g.ctl(self.miGenerateThumbsDialogYesButton).set_visible(true);
        g.ctl(self.miGenerateThumbsDialogNoButton).set_visible(true);

        self.remember_current();

        let focus = self.miGenerateThumbsDialogNoButton.id();
        self.set_initial_focus(gp.get(), focus);
        self.set_cancel_code(MENU_CODE_GENERATE_THUMBS_RESET_NO);
    }

    pub fn hide_thumbnails_popup(&mut self, g: &mut Game) {
        g.ctl(self.miGenerateThumbsDialogImage).set_visible(false);
        g.ctl(self.miGenerateThumbsDialogAreYouText).set_visible(false);
        g.ctl(self.miGenerateThumbsDialogSureText).set_visible(false);
        g.ctl(self.miGenerateThumbsDialogYesButton).set_visible(false);
        g.ctl(self.miGenerateThumbsDialogNoButton).set_visible(false);

        let focus = self.miGameplayOptionsMenuButton.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_TO_MAIN_MENU);

        self.restore_current(g);
    }
}
