//! Port of src/smw/menu/GameSettingsMenu.cpp

use crate::common::game_mode::{game_mode_owned, GAMEMODE_LAST, GAMEMODE_NUM_OPTIONS};
use crate::common::global_constants::NUM_AUTO_FILTERS;
use crate::common::path::get_name_from_file_name;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_image_select_field::MI_ImageSelectField;
use crate::common::ui::mi_map_field::MI_MapField;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, ControlPtr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::main::{currentgamemode, gamemodes};
use crate::smw::ui::mi_map_filter_scroll::MI_MapFilterScroll;

const g_szAutoFilterNames: [&str; NUM_AUTO_FILTERS as usize] = [
    "Death Tiles",
    "Warps",
    "Ice",
    "Item Boxes",
    "Breakable Blocks",
    "Throwable Blocks",
    "On/Off Blocks",
    "Platforms",
    "Hazards",
    "Item Destroyable Blocks",
    "Hidden Blocks",
    "Map Items",
];
const g_iAutoFilterIcons: [i16; NUM_AUTO_FILTERS as usize] = [37, 29, 33, 1, 0, 6, 40, 73, 19, 87, 17, 118];

const GM_LAST: usize = GAMEMODE_LAST as usize;

#[derive(Default)]
pub struct UI_GameSettingsMenu {
    pub ui_menu: UI_Menu,

    pub miMapFilterScroll: Ptr<MI_MapFilterScroll>,
    pub miMapField: Ptr<MI_MapField>,
    pub miGoalField: [Ptr<MI_SelectField<i16>>; GM_LAST],

    pub miModeField: Ptr<MI_ImageSelectField>,
    pub miModeSettingsButton: Ptr<MI_Button>,
    pub miSettingsStartButton: Ptr<MI_Button>,
    pub miMapFiltersButton: Ptr<MI_Button>,
    pub miMapThumbnailsButton: Ptr<MI_Button>,

    pub miMapFiltersOnImage: Ptr<MI_Image>,

    pub miGameSettingsLeftHeaderBar: Ptr<MI_Image>,
    pub miGameSettingsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miGameSettingsMenuHeaderText: Ptr<MI_Text>,

    pub miGameSettingsExitDialogImage: Ptr<MI_Image>,
    pub miGameSettingsExitDialogExitText: Ptr<MI_Text>,
    pub miGameSettingsExitDialogTournamentText: Ptr<MI_Text>,
    pub miGameSettingsExitDialogYesButton: Ptr<MI_Button>,
    pub miGameSettingsExitDialogNoButton: Ptr<MI_Button>,
}
crate::impl_base!(UI_GameSettingsMenu => ui_menu: UI_Menu);

impl UI_GameSettingsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miSettingsStartButton = Ptr::new_box(MI_Button::new(spr_selectfield, 70, 45, "Start", 500, TextAlign::LEFT));
            this.miSettingsStartButton.set_code(MENU_CODE_START_GAME);

            this.miModeField = Ptr::new_box(MI_ImageSelectField::new(spr_selectfield, Ptr::from_mut(&mut rm.menu_mode_small), 70, 85, "Mode", 500, 120, 16, 16));

            for iGameMode in 0..GAMEMODE_LAST as i16 {
                let name = gamemodes[iGameMode as usize].gm().get_mode_name().to_string();
                this.miModeField.add(name, iGameMode);
            }
            this.miModeField.set_output_ptr(&mut currentgamemode);
            this.miModeField.set_current_value(0);
            this.miModeField.set_item_changed_code(MENU_CODE_MODE_CHANGED);

            for iGameMode in 0..GM_LAST {
                let mut mode = gamemodes[iGameMode];
                let mut field = Ptr::new_box(MI_SelectField::<i16>::new(spr_selectfield, 70, 125, mode.gm().get_goal_name(), 352, 120));
                this.miGoalField[iGameMode] = field;
                // miGoalField[iGameMode]->SetKey(gamemodes[iGameMode]->goal);
                field.set_visible(iGameMode == 0);

                for iGameModeOption in 0..GAMEMODE_NUM_OPTIONS {
                    let option = &mode.gm_mut().get_options()[iGameModeOption];
                    field.add(option.szName.clone(), option.iValue);
                }

                field.set_output_ptr(&mut mode.gm_mut().goal);
                field.set_current_value(mode.gm().goal);
            }

            this.miModeSettingsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 430, 125, "Settings", 140, TextAlign::LEFT));
            this.miModeSettingsButton.set_code(MENU_CODE_TO_MODE_SETTINGS_MENU);

            this.miMapField = Ptr::new_box(MI_MapField::new(spr_selectfield, 70, 165, "Map", 500, 120, true));

            this.miMapFiltersButton = Ptr::new_box(MI_Button::new(spr_selectfield, 430, 205, "Filters", 140, TextAlign::LEFT));
            this.miMapFiltersButton.set_code(MENU_CODE_TO_MAP_FILTERS);

            this.miMapFiltersOnImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_map_filter), 530, 213, 0, 48, 16, 16, 1, 1, 0));
            this.miMapFiltersOnImage.set_visible(false);

            this.miMapThumbnailsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 430, 245, "Thumbs", 140, TextAlign::LEFT));
            this.miMapThumbnailsButton.set_code(MENU_CODE_TO_MAP_BROWSER_THUMBNAILS);

            this.miMapFilterScroll = Ptr::new_box(MI_MapFilterScroll::new(menu_plain_field, 120, 72, 400, 9));
            this.miMapFilterScroll.set_auto_modify(true);
            this.miMapFilterScroll.set_visible(false);

            // Add auto map filters
            for iFilter in 0..NUM_AUTO_FILTERS as usize {
                this.miMapFilterScroll.add(g_szAutoFilterNames[iFilter].to_string(), g_iAutoFilterIcons[iFilter]);
            }

            // Add user defined filters
            for iFilter in 0..filterslist.count() {
                let szTemp = get_name_from_file_name(&filterslist.at(iFilter).to_string_lossy(), true);
                this.miMapFilterScroll.add(szTemp, game_values.piFilterIcons[NUM_AUTO_FILTERS as usize + iFilter]);
            }

            this.miGameSettingsLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miGameSettingsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miGameSettingsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Single Game Menu", 320, 5));

            // Exit tournament dialog box
            this.miGameSettingsExitDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miGameSettingsExitDialogExitText = Ptr::new_box(MI_HeaderText::new("Exit", 320, 195));
            this.miGameSettingsExitDialogTournamentText = Ptr::new_box(MI_HeaderText::new("Tournament", 320, 220));
            this.miGameSettingsExitDialogYesButton = Ptr::new_box(MI_Button::new(spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER));
            this.miGameSettingsExitDialogNoButton = Ptr::new_box(MI_Button::new(spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER));
        }

        this.miGameSettingsExitDialogYesButton.set_code(MENU_CODE_EXIT_TOURNAMENT_YES);
        this.miGameSettingsExitDialogNoButton.set_code(MENU_CODE_EXIT_TOURNAMENT_NO);

        this.miGameSettingsExitDialogImage.set_visible(false);
        this.miGameSettingsExitDialogTournamentText.set_visible(false);
        this.miGameSettingsExitDialogExitText.set_visible(false);
        this.miGameSettingsExitDialogYesButton.set_visible(false);
        this.miGameSettingsExitDialogNoButton.set_visible(false);

        let start = ctl_ptr(this.miSettingsStartButton);
        let mode = ctl_ptr(this.miModeField);
        let goal: Vec<ControlPtr> = this.miGoalField.iter().map(|g| ctl_ptr(*g)).collect();
        let settings = ctl_ptr(this.miModeSettingsButton);
        let map = ctl_ptr(this.miMapField);
        let filters = ctl_ptr(this.miMapFiltersButton);
        let thumbs = ctl_ptr(this.miMapThumbnailsButton);
        let null = Ptr::null();

        this.add_control(start, thumbs, mode, null, null);
        this.add_control(mode, start, goal[0], null, null);

        this.add_control(goal[0], mode, goal[1], null, settings);

        for iGoalField in 1..GM_LAST - 1 {
            this.add_control(goal[iGoalField], goal[iGoalField - 1], goal[iGoalField + 1], goal[iGoalField - 1], settings);
        }

        this.add_control(goal[GM_LAST - 1], goal[GM_LAST - 2], map, goal[GM_LAST - 2], settings);

        this.add_control(settings, mode, map, goal[GM_LAST - 1], null);
        this.add_control(map, goal[GM_LAST - 1], filters, null, null);
        this.add_control(filters, map, thumbs, null, null);
        this.add_control(thumbs, filters, start, null, null);

        let c = ctl_ptr(this.miMapFilterScroll);
        this.add_control(c, null, null, null, null);

        let c = ctl_ptr(this.miGameSettingsLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGameSettingsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGameSettingsMenuHeaderText);
        this.add_non_control(c);

        let c = ctl_ptr(this.miGameSettingsExitDialogImage);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGameSettingsExitDialogExitText);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGameSettingsExitDialogTournamentText);
        this.add_non_control(c);

        let c = ctl_ptr(this.miMapFiltersOnImage);
        this.add_non_control(c);

        let yes = ctl_ptr(this.miGameSettingsExitDialogYesButton);
        let no = ctl_ptr(this.miGameSettingsExitDialogNoButton);
        this.add_control(yes, null, null, null, no);
        this.add_control(no, null, null, yes, null);

        this.set_initial_focus(start);

        this.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);
        this
    }

    pub fn refresh_game_mode_buttons(&mut self) {
        // Unhide/hide the settings button
        let current = self.miModeField.current_value();
        self.miModeSettingsButton.set_visible(current as i32 != game_mode_owned);

        // Show the approprate goal field
        for iMode in 0..GM_LAST {
            self.miGoalField[iMode].set_visible(self.miModeField.current_value() as usize == iMode);
        }
    }

    pub fn open_map_filters(&mut self) {
        self.miMapFilterScroll.set_visible(true);
        self.remember_current();

        let focus = ctl_ptr(self.miMapFilterScroll);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu();
    }

    pub fn close_map_filters(&mut self) {
        self.miMapFilterScroll.set_visible(false);

        let focus = ctl_ptr(self.miSettingsStartButton);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current();

        unsafe {
            self.miMapFiltersOnImage.set_visible(game_values.fFiltersOn);
        }
    }

    pub fn open_exit_dialog(&mut self) {
        self.miGameSettingsExitDialogImage.set_visible(true);
        self.miGameSettingsExitDialogTournamentText.set_visible(true);
        self.miGameSettingsExitDialogExitText.set_visible(true);
        self.miGameSettingsExitDialogYesButton.set_visible(true);
        self.miGameSettingsExitDialogNoButton.set_visible(true);

        self.remember_current();

        let focus = ctl_ptr(self.miGameSettingsExitDialogNoButton);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu();
    }

    pub fn close_exit_dialog(&mut self) {
        self.miGameSettingsExitDialogImage.set_visible(false);
        self.miGameSettingsExitDialogTournamentText.set_visible(false);
        self.miGameSettingsExitDialogExitText.set_visible(false);
        self.miGameSettingsExitDialogYesButton.set_visible(false);
        self.miGameSettingsExitDialogNoButton.set_visible(false);

        let focus = ctl_ptr(self.miSettingsStartButton);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current();
    }

    pub fn set_header_text(&mut self, string: &str) {
        self.miGameSettingsMenuHeaderText.set_text(string);
    }

    pub fn game_mode_changed(&mut self, gmID: i16) {
        self.miModeField.set_current_value(gmID);

        self.refresh_game_mode_buttons();
    }

    pub fn hide_gm_settings_btn(&mut self) {
        self.miModeSettingsButton.set_visible(false);
    }

    pub fn get_current_game_mode_id(&self) -> i16 {
        self.miModeField.current_value()
    }

    pub fn get_current_map_name(&self) -> String {
        self.miMapField.get_map_name().to_string()
    }

    pub fn is_on_start_btn(&self) -> bool {
        self.initial_focus() == ctl_ptr(self.miSettingsStartButton)
    }
}
