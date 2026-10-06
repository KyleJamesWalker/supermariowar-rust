//! Port of src/smw/menu/MatchSelectionMenu.cpp

use crate::common::match_types::{MatchType, Minigame};
use crate::common::path::get_name_from_file_name;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_world_preview_display::MI_WorldPreviewDisplay;

#[derive(Default)]
pub struct UI_MatchSelectionMenu {
    pub ui_menu: UI_Menu,

    pub miMatchSelectionDisplayImage: Ptr<MI_Image>,

    pub miMatchSelectionField: Ptr<MI_SelectField<MatchType>>,
    pub miTournamentField: Ptr<MI_SelectField<i16>>,
    pub miTourField: Ptr<MI_SelectField<i16>>,
    pub miWorldField: Ptr<MI_SelectField<i16>>,
    pub miMinigameField: Ptr<MI_SelectField<Minigame>>,
    pub miWorldPreviewDisplay: Ptr<MI_WorldPreviewDisplay>,

    pub miMatchSelectionStartButton: Ptr<MI_Button>,

    pub miMatchSelectionMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miMatchSelectionMenuRightHeaderBar: Ptr<MI_Image>,
    pub miMatchSelectionMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_MatchSelectionMenu => ui_menu: UI_Menu);

impl UI_MatchSelectionMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miMatchSelectionStartButton = Ptr::new_box(MI_Button::new(spr_selectfield, 270, 420, "Start", 100, TextAlign::LEFT));
            this.miMatchSelectionStartButton.set_code(MENU_CODE_MATCH_SELECTION_START);

            this.miMatchSelectionField = Ptr::new_box(MI_SelectField::new(spr_selectfield, 130, 340, "Match", 380, 100));
            let mut f = this.miMatchSelectionField;
            f.add("Single Game", MatchType::SingleGame);
            f.add("Tournament", MatchType::Tournament);
            f.add("Tour", MatchType::Tour);
            f.add("World", MatchType::World);
            f.add("Minigame", MatchType::MiniGame);
            f.set_output_ptr(&mut game_values.matchtype);
            f.set_current_value(game_values.matchtype);
            f.set_item_changed_code(MENU_CODE_MATCH_SELECTION_MATCH_CHANGED);

            this.miTournamentField = Ptr::new_box(MI_SelectField::new(spr_selectfield, 130, 380, "Wins", 380, 100));
            let mut f = this.miTournamentField;
            f.add("2", 2);
            f.add("3", 3);
            f.add("4", 4);
            f.add("5", 5);
            f.add("6", 6);
            f.add("7", 7);
            f.add("8", 8);
            f.add("9", 9);
            f.add("10", 10);
            f.set_output_ptr(&mut game_values.tournamentgames);
            f.set_current_value(game_values.tournamentgames);
            f.set_visible(false);

            this.miTourField = Ptr::new_box(MI_SelectField::new(spr_selectfield, 130, 380, "Tour", 380, 100));
            let mut f = this.miTourField;
            for iTour in 0..tourlist.count() {
                let szTemp = get_name_from_file_name(&tourlist.at(iTour).to_string_lossy(), true);
                // strcat(szTemp, " Tour");
                f.add(szTemp, iTour as i16);
            }
            f.set_output_ptr(&mut game_values.tourindex);
            f.set_current_value(game_values.tourindex);
            f.set_visible(false);

            this.miWorldField = Ptr::new_box(MI_SelectField::new(spr_selectfield, 130, 380, "World", 380, 100));
            let mut f = this.miWorldField;
            for iWorld in 0..worldlist.count() {
                let szTemp = get_name_from_file_name(&worldlist.at(iWorld).to_string_lossy(), true);
                f.add(szTemp, iWorld as i16);
            }
            f.set_output_ptr(&mut game_values.worldindex);
            f.set_current_value(game_values.worldindex);
            f.set_item_changed_code(MENU_CODE_WORLD_MAP_CHANGED);
            f.set_visible(false);

            this.miMinigameField = Ptr::new_box(MI_SelectField::new(spr_selectfield, 130, 380, "Game", 380, 100));
            let mut f = this.miMinigameField;
            f.add("Pipe Coin Game", Minigame::PipeCoin);
            f.add("Hammer Boss Game", Minigame::HammerBoss);
            f.add("Bomb Boss Game", Minigame::BombBoss);
            f.add("Fire Boss Game", Minigame::FireBoss);
            f.add("Boxes Game", Minigame::Boxes);
            f.set_output_ptr(&mut game_values.selectedminigame);
            f.set_current_value(game_values.selectedminigame);
            f.set_visible(false);

            this.miMatchSelectionMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miMatchSelectionMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miMatchSelectionMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Match Type Menu", 320, 5));

            this.miMatchSelectionDisplayImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_match_select), 160, 80, 0, 0, 320, 240, 1, 1, 0));
            this.miWorldPreviewDisplay = Ptr::new_box(MI_WorldPreviewDisplay::new(160, 80, 20, 15));
            this.miWorldPreviewDisplay.set_visible(false);
        }

        let c = ctl_ptr(this.miMatchSelectionMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miMatchSelectionMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miMatchSelectionMenuHeaderText);
        this.add_non_control(c);

        let c = ctl_ptr(this.miWorldPreviewDisplay);
        this.add_non_control(c);
        let c = ctl_ptr(this.miMatchSelectionDisplayImage);
        this.add_non_control(c);

        let matchf = ctl_ptr(this.miMatchSelectionField);
        let tournament = ctl_ptr(this.miTournamentField);
        let tour = ctl_ptr(this.miTourField);
        let world = ctl_ptr(this.miWorldField);
        let minigame = ctl_ptr(this.miMinigameField);
        let start = ctl_ptr(this.miMatchSelectionStartButton);
        let null = Ptr::null();

        this.add_control(matchf, start, tournament, null, null);
        this.add_control(tournament, matchf, tour, null, null);
        this.add_control(tour, tournament, world, null, null);
        this.add_control(world, tour, minigame, null, null);
        this.add_control(minigame, world, start, null, null);
        this.add_control(start, minigame, matchf, null, null);

        this.set_initial_focus(start);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }

    pub fn selection_changed(&mut self) {
        unsafe {
            let matchtype = game_values.matchtype;
            self.miTournamentField.set_visible(matchtype == MatchType::Tournament);
            self.miTourField.set_visible(matchtype == MatchType::Tour);
            self.miWorldField.set_visible(matchtype == MatchType::World);
            self.miMinigameField.set_visible(matchtype == MatchType::MiniGame);

            // miMatchSelectionDisplayImage->Show(game_values.matchtype != MatchType::World);
            self.miWorldPreviewDisplay.set_visible(matchtype == MatchType::World);

            if matchtype == MatchType::World {
                self.miMatchSelectionDisplayImage.set_image(320, 0, 320, 240);
            } else {
                self.miMatchSelectionDisplayImage.set_image(0, (240 * matchtype as i32) as i16, 320, 240);
            }
        }
    }

    pub fn world_map_changed(&mut self) {
        self.miWorldPreviewDisplay.set_world();
    }

    pub fn activate_minigame_field(&mut self) {
        self.miMatchSelectionField.hide_item(MatchType::MiniGame, false);
        self.miMatchSelectionField.set_current_value(MatchType::MiniGame);

        self.miTournamentField.set_visible(false);
        self.miTourField.set_visible(false);
        self.miWorldField.set_visible(false);
        self.miMinigameField.set_visible(true);

        self.miMatchSelectionDisplayImage.set_visible(true);
        self.miWorldPreviewDisplay.set_visible(false);
        unsafe {
            self.miMatchSelectionDisplayImage.set_image(0, (240 * game_values.matchtype as i32) as i16, 320, 240);
        }
    }

    pub fn get_minigame(&self) -> Minigame {
        self.miMinigameField.current_value()
    }

    pub fn get_selected_match_type(&self) -> MatchType {
        self.miMatchSelectionField.current_value()
    }
}
