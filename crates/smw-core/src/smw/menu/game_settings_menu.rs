//! The game setup menu: mode, goal, map and map filters, plus the Exit Tournament dialog.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
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
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;

use crate::smw::ui::mi_map_filter_scroll::MI_MapFilterScroll;

/// Names of the built-in map filters, in filter index order.
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
/// Icon index for each built-in map filter.
const g_iAutoFilterIcons: [i16; NUM_AUTO_FILTERS as usize] = [37, 29, 33, 1, 0, 6, 40, 73, 19, 87, 17, 118];

const GM_LAST: usize = GAMEMODE_LAST as usize;

#[derive(Default)]
pub struct UI_GameSettingsMenu {
    pub ui_menu: UI_Menu,

    /// Filter list popup, built-in filters first, then user filters; hidden until [`Self::open_map_filters`].
    pub miMapFilterScroll: Ctl<MI_MapFilterScroll>,
    pub miMapField: Ctl<MI_MapField>,
    /// One goal field per game mode; only the selected mode's is visible.
    pub miGoalField: [Ctl<MI_SelectField<i16>>; GM_LAST],

    /// Game mode picker; writes `currentgamemode`.
    pub miModeField: Ctl<MI_ImageSelectField>,
    pub miModeSettingsButton: Ctl<MI_Button>,
    pub miSettingsStartButton: Ctl<MI_Button>,
    pub miMapFiltersButton: Ctl<MI_Button>,
    pub miMapThumbnailsButton: Ctl<MI_Button>,

    /// Icon next to Filters, visible while `fFiltersOn`.
    pub miMapFiltersOnImage: Ctl<MI_Image>,

    pub miGameSettingsLeftHeaderBar: Ctl<MI_Image>,
    pub miGameSettingsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miGameSettingsMenuHeaderText: Ctl<MI_Text>,

    pub miGameSettingsExitDialogImage: Ctl<MI_Image>,
    pub miGameSettingsExitDialogExitText: Ctl<MI_Text>,
    pub miGameSettingsExitDialogTournamentText: Ctl<MI_Text>,
    pub miGameSettingsExitDialogYesButton: Ctl<MI_Button>,
    pub miGameSettingsExitDialogNoButton: Ctl<MI_Button>,
}
crate::impl_base!(UI_GameSettingsMenu => ui_menu: UI_Menu);

impl UI_GameSettingsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;
        let menu_plain_field = g.rm.menu_plain_field;

        this.miSettingsStartButton = Ctl::new(MI_Button::new(g, spr_selectfield, 70, 45, "Start", 500, TextAlign::LEFT), g);
        g.ctl(this.miSettingsStartButton).set_code(MENU_CODE_START_GAME);

        this.miModeField = Ctl::new(MI_ImageSelectField::new(spr_selectfield, g.rm.menu_mode_small, 70, 85, "Mode", 500, 120, 16, 16), g);

        for iGameMode in 0..GAMEMODE_LAST as i16 {
            let name = g.gamemodes[iGameMode as usize].gm().get_mode_name().to_string();
            g.ctl(this.miModeField).add(name, iGameMode);
        }
        g.ctl(this.miModeField).set_output_ptr(&mut g.currentgamemode);
        g.ctl(this.miModeField).set_current_value(0);
        g.ctl(this.miModeField).set_item_changed_code(MENU_CODE_MODE_CHANGED);

        for iGameMode in 0..GM_LAST {
            let mut mode = g.gamemodes[iGameMode].ptr();
            let mut field = Ctl::new(MI_SelectField::<i16>::new(spr_selectfield, 70, 125, mode.gm().get_goal_name(), 352, 120), g);
            this.miGoalField[iGameMode] = field;
            g.ctl(field).set_visible(iGameMode == 0);

            for iGameModeOption in 0..GAMEMODE_NUM_OPTIONS {
                let option = &mode.gm_mut().get_options()[iGameModeOption];
                g.ctl(field).add(option.szName.clone(), option.iValue);
            }

            g.ctl(field).set_output_ptr(&mut mode.gm_mut().goal);
            g.ctl(field).set_current_value(mode.gm().goal);
        }

        this.miModeSettingsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 430, 125, "Settings", 140, TextAlign::LEFT), g);
        g.ctl(this.miModeSettingsButton).set_code(MENU_CODE_TO_MODE_SETTINGS_MENU);

        this.miMapField = Ctl::new(MI_MapField::new(g, spr_selectfield, 70, 165, "Map", 500, 120, true), g);

        this.miMapFiltersButton = Ctl::new(MI_Button::new(g, spr_selectfield, 430, 205, "Filters", 140, TextAlign::LEFT), g);
        g.ctl(this.miMapFiltersButton).set_code(MENU_CODE_TO_MAP_FILTERS);

        this.miMapFiltersOnImage = Ctl::new(MI_Image::new(g.rm.menu_map_filter, 530, 213, 0, 48, 16, 16, 1, 1, 0), g);
        g.ctl(this.miMapFiltersOnImage).set_visible(false);

        this.miMapThumbnailsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 430, 245, "Thumbs", 140, TextAlign::LEFT), g);
        g.ctl(this.miMapThumbnailsButton).set_code(MENU_CODE_TO_MAP_BROWSER_THUMBNAILS);

        this.miMapFilterScroll = Ctl::new(MI_MapFilterScroll::new(menu_plain_field, 120, 72, 400, 9), g);
        g.ctl(this.miMapFilterScroll).set_auto_modify(true);
        g.ctl(this.miMapFilterScroll).set_visible(false);

        // Add auto map filters
        for iFilter in 0..NUM_AUTO_FILTERS as usize {
            g.ctl(this.miMapFilterScroll).add(g_szAutoFilterNames[iFilter].to_string(), g_iAutoFilterIcons[iFilter]);
        }

        // Add user defined filters
        for iFilter in 0..g.filterslist.count() {
            let szTemp = get_name_from_file_name(&g.filterslist.at(iFilter).to_string_lossy(), true);
            g.ctl(this.miMapFilterScroll).add(szTemp, g.game_values.piFilterIcons[NUM_AUTO_FILTERS as usize + iFilter]);
        }

        this.miGameSettingsLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miGameSettingsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miGameSettingsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Single Game Menu", 320, 5), g);

        // Exit tournament dialog box
        this.miGameSettingsExitDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miGameSettingsExitDialogExitText = Ctl::new(MI_HeaderText::new(g, "Exit", 320, 195), g);
        this.miGameSettingsExitDialogTournamentText = Ctl::new(MI_HeaderText::new(g, "Tournament", 320, 220), g);
        this.miGameSettingsExitDialogYesButton = Ctl::new(MI_Button::new(g, spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        this.miGameSettingsExitDialogNoButton = Ctl::new(MI_Button::new(g, spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);

        g.ctl(this.miGameSettingsExitDialogYesButton).set_code(MENU_CODE_EXIT_TOURNAMENT_YES);
        g.ctl(this.miGameSettingsExitDialogNoButton).set_code(MENU_CODE_EXIT_TOURNAMENT_NO);

        g.ctl(this.miGameSettingsExitDialogImage).set_visible(false);
        g.ctl(this.miGameSettingsExitDialogTournamentText).set_visible(false);
        g.ctl(this.miGameSettingsExitDialogExitText).set_visible(false);
        g.ctl(this.miGameSettingsExitDialogYesButton).set_visible(false);
        g.ctl(this.miGameSettingsExitDialogNoButton).set_visible(false);

        let start = this.miSettingsStartButton.id();
        let mode = this.miModeField.id();
        let goal: Vec<ControlId> = this.miGoalField.iter().map(|c| c.id()).collect();
        let settings = this.miModeSettingsButton.id();
        let map = this.miMapField.id();
        let filters = this.miMapFiltersButton.id();
        let thumbs = this.miMapThumbnailsButton.id();
        let null = ControlId::NONE;

        this.add_control(g, start, thumbs, mode, null, null);
        this.add_control(g, mode, start, goal[0], null, null);

        this.add_control(g, goal[0], mode, goal[1], null, settings);

        for iGoalField in 1..GM_LAST - 1 {
            this.add_control(g, goal[iGoalField], goal[iGoalField - 1], goal[iGoalField + 1], goal[iGoalField - 1], settings);
        }

        this.add_control(g, goal[GM_LAST - 1], goal[GM_LAST - 2], map, goal[GM_LAST - 2], settings);

        this.add_control(g, settings, mode, map, goal[GM_LAST - 1], null);
        this.add_control(g, map, goal[GM_LAST - 1], filters, null, null);
        this.add_control(g, filters, map, thumbs, null, null);
        this.add_control(g, thumbs, filters, start, null, null);

        let c = this.miMapFilterScroll.id();
        this.add_control(g, c, null, null, null, null);

        let c = this.miGameSettingsLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miGameSettingsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miGameSettingsMenuHeaderText.id();
        this.add_non_control(c);

        let c = this.miGameSettingsExitDialogImage.id();
        this.add_non_control(c);
        let c = this.miGameSettingsExitDialogExitText.id();
        this.add_non_control(c);
        let c = this.miGameSettingsExitDialogTournamentText.id();
        this.add_non_control(c);

        let c = this.miMapFiltersOnImage.id();
        this.add_non_control(c);

        let yes = this.miGameSettingsExitDialogYesButton.id();
        let no = this.miGameSettingsExitDialogNoButton.id();
        this.add_control(g, yes, null, null, null, no);
        this.add_control(g, no, null, null, yes, null);

        this.set_initial_focus(g, start);

        this.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);
        this
    }

    /// Shows the selected mode's goal field, and Settings for every mode but Owned.
    pub fn refresh_game_mode_buttons(&mut self, g: &mut Game) {
        // Unhide/hide the settings button
        let current = g.ctl(self.miModeField).current_value();
        g.ctl(self.miModeSettingsButton).set_visible(current as i32 != game_mode_owned);

        // Show the approprate goal field
        for iMode in 0..GM_LAST {
            g.ctl(self.miGoalField[iMode]).set_visible(g.ctl(self.miModeField).current_value() as usize == iMode);
        }
    }

    /// Shows the filter list popup and focuses it, remembering the focus to restore.
    pub fn open_map_filters(&mut self, g: &mut Game) {
        g.ctl(self.miMapFilterScroll).set_visible(true);
        self.remember_current();

        let focus = self.miMapFilterScroll.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu(g);
    }

    /// Hides the filter list, restores focus and updates the filters-on icon.
    pub fn close_map_filters(&mut self, g: &mut Game) {
        g.ctl(self.miMapFilterScroll).set_visible(false);

        let focus = self.miSettingsStartButton.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current(g);

        g.ctl(self.miMapFiltersOnImage).set_visible(g.game_values.fFiltersOn);
    }

    /// Opens the Exit Tournament dialog with No focused and cancel disabled.
    pub fn open_exit_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miGameSettingsExitDialogImage).set_visible(true);
        g.ctl(self.miGameSettingsExitDialogTournamentText).set_visible(true);
        g.ctl(self.miGameSettingsExitDialogExitText).set_visible(true);
        g.ctl(self.miGameSettingsExitDialogYesButton).set_visible(true);
        g.ctl(self.miGameSettingsExitDialogNoButton).set_visible(true);

        self.remember_current();

        let focus = self.miGameSettingsExitDialogNoButton.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu(g);
    }

    /// Hides the Exit Tournament dialog and restores the remembered focus.
    pub fn close_exit_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miGameSettingsExitDialogImage).set_visible(false);
        g.ctl(self.miGameSettingsExitDialogTournamentText).set_visible(false);
        g.ctl(self.miGameSettingsExitDialogExitText).set_visible(false);
        g.ctl(self.miGameSettingsExitDialogYesButton).set_visible(false);
        g.ctl(self.miGameSettingsExitDialogNoButton).set_visible(false);

        let focus = self.miSettingsStartButton.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current(g);
    }

    pub fn set_header_text(&mut self, g: &mut Game, string: &str) {
        g.ctl(self.miGameSettingsMenuHeaderText).set_text(string);
    }

    /// Selects game mode `gmID` and refreshes the goal field and Settings button.
    pub fn game_mode_changed(&mut self, g: &mut Game, gmID: i16) {
        g.ctl(self.miModeField).set_current_value(gmID);

        self.refresh_game_mode_buttons(g);
    }

    pub fn hide_gm_settings_btn(&mut self, g: &mut Game) {
        g.ctl(self.miModeSettingsButton).set_visible(false);
    }

    pub fn get_current_game_mode_id(&self, g: &Game) -> i16 {
        g.ctl(self.miModeField).current_value()
    }

    pub fn get_current_map_name(&self, g: &Game) -> String {
        g.ctl(self.miMapField).get_map_name().to_string()
    }

    /// True when Start is the menu's initial focus.
    pub fn is_on_start_btn(&self) -> bool {
        self.initial_focus() == self.miSettingsStartButton.id()
    }
}
