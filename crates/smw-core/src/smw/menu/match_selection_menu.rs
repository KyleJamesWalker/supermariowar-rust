//! Port of src/smw/menu/MatchSelectionMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::match_types::{MatchType, Minigame};
use crate::common::path::get_name_from_file_name;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_world_preview_display::MI_WorldPreviewDisplay;

#[derive(Default)]
pub struct UI_MatchSelectionMenu {
    pub ui_menu: UI_Menu,

    pub miMatchSelectionDisplayImage: Ctl<MI_Image>,

    pub miMatchSelectionField: Ctl<MI_SelectField<MatchType>>,
    pub miTournamentField: Ctl<MI_SelectField<i16>>,
    pub miTourField: Ctl<MI_SelectField<i16>>,
    pub miWorldField: Ctl<MI_SelectField<i16>>,
    pub miMinigameField: Ctl<MI_SelectField<Minigame>>,
    pub miWorldPreviewDisplay: Ctl<MI_WorldPreviewDisplay>,

    pub miMatchSelectionStartButton: Ctl<MI_Button>,

    pub miMatchSelectionMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miMatchSelectionMenuRightHeaderBar: Ctl<MI_Image>,
    pub miMatchSelectionMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_MatchSelectionMenu => ui_menu: UI_Menu);

impl UI_MatchSelectionMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;
        let menu_plain_field = g.rm.menu_plain_field;

        this.miMatchSelectionStartButton = Ctl::new(MI_Button::new(g, spr_selectfield, 270, 420, "Start", 100, TextAlign::LEFT), g);
        g.ctl(this.miMatchSelectionStartButton).set_code(MENU_CODE_MATCH_SELECTION_START);

        this.miMatchSelectionField = Ctl::new(MI_SelectField::new(spr_selectfield, 130, 340, "Match", 380, 100), g);
        let mut f = this.miMatchSelectionField;
        g.ctl(f).add("Single Game", MatchType::SingleGame);
        g.ctl(f).add("Tournament", MatchType::Tournament);
        g.ctl(f).add("Tour", MatchType::Tour);
        g.ctl(f).add("World", MatchType::World);
        g.ctl(f).add("Minigame", MatchType::MiniGame);
        g.ctl(f).set_output_ptr(&mut g.game_values.matchtype);
        g.ctl(f).set_current_value(g.game_values.matchtype);
        g.ctl(f).set_item_changed_code(MENU_CODE_MATCH_SELECTION_MATCH_CHANGED);

        this.miTournamentField = Ctl::new(MI_SelectField::new(spr_selectfield, 130, 380, "Wins", 380, 100), g);
        let mut f = this.miTournamentField;
        g.ctl(f).add("2", 2);
        g.ctl(f).add("3", 3);
        g.ctl(f).add("4", 4);
        g.ctl(f).add("5", 5);
        g.ctl(f).add("6", 6);
        g.ctl(f).add("7", 7);
        g.ctl(f).add("8", 8);
        g.ctl(f).add("9", 9);
        g.ctl(f).add("10", 10);
        g.ctl(f).set_output_ptr(&mut g.game_values.tournamentgames);
        g.ctl(f).set_current_value(g.game_values.tournamentgames);
        g.ctl(f).set_visible(false);

        this.miTourField = Ctl::new(MI_SelectField::new(spr_selectfield, 130, 380, "Tour", 380, 100), g);
        let mut f = this.miTourField;
        for iTour in 0..g.tourlist.count() {
            let szTemp = get_name_from_file_name(&g.tourlist.at(iTour).to_string_lossy(), true);
            // strcat(szTemp, " Tour");
            g.ctl(f).add(szTemp, iTour as i16);
        }
        g.ctl(f).set_output_ptr(&mut g.game_values.tourindex);
        g.ctl(f).set_current_value(g.game_values.tourindex);
        g.ctl(f).set_visible(false);

        this.miWorldField = Ctl::new(MI_SelectField::new(spr_selectfield, 130, 380, "World", 380, 100), g);
        let mut f = this.miWorldField;
        for iWorld in 0..g.worldlist.count() {
            let szTemp = get_name_from_file_name(&g.worldlist.at(iWorld).to_string_lossy(), true);
            g.ctl(f).add(szTemp, iWorld as i16);
        }
        g.ctl(f).set_output_ptr(&mut g.game_values.worldindex);
        g.ctl(f).set_current_value(g.game_values.worldindex);
        g.ctl(f).set_item_changed_code(MENU_CODE_WORLD_MAP_CHANGED);
        g.ctl(f).set_visible(false);

        this.miMinigameField = Ctl::new(MI_SelectField::new(spr_selectfield, 130, 380, "Game", 380, 100), g);
        let mut f = this.miMinigameField;
        g.ctl(f).add("Pipe Coin Game", Minigame::PipeCoin);
        g.ctl(f).add("Hammer Boss Game", Minigame::HammerBoss);
        g.ctl(f).add("Bomb Boss Game", Minigame::BombBoss);
        g.ctl(f).add("Fire Boss Game", Minigame::FireBoss);
        g.ctl(f).add("Boxes Game", Minigame::Boxes);
        g.ctl(f).set_output_ptr(&mut g.game_values.selectedminigame);
        g.ctl(f).set_current_value(g.game_values.selectedminigame);
        g.ctl(f).set_visible(false);

        this.miMatchSelectionMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miMatchSelectionMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miMatchSelectionMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Match Type Menu", 320, 5), g);

        this.miMatchSelectionDisplayImage = Ctl::new(MI_Image::new(g.rm.menu_match_select, 160, 80, 0, 0, 320, 240, 1, 1, 0), g);
        this.miWorldPreviewDisplay = Ctl::new(MI_WorldPreviewDisplay::new(g, 160, 80, 20, 15), g);
        g.ctl(this.miWorldPreviewDisplay).set_visible(false);

        let c = this.miMatchSelectionMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miMatchSelectionMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miMatchSelectionMenuHeaderText.id();
        this.add_non_control(c);

        let c = this.miWorldPreviewDisplay.id();
        this.add_non_control(c);
        let c = this.miMatchSelectionDisplayImage.id();
        this.add_non_control(c);

        let matchf = this.miMatchSelectionField.id();
        let tournament = this.miTournamentField.id();
        let tour = this.miTourField.id();
        let world = this.miWorldField.id();
        let minigame = this.miMinigameField.id();
        let start = this.miMatchSelectionStartButton.id();
        let null = ControlId::NONE;

        this.add_control(g, matchf, start, tournament, null, null);
        this.add_control(g, tournament, matchf, tour, null, null);
        this.add_control(g, tour, tournament, world, null, null);
        this.add_control(g, world, tour, minigame, null, null);
        this.add_control(g, minigame, world, start, null, null);
        this.add_control(g, start, minigame, matchf, null, null);

        this.set_initial_focus(g, start);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }

    pub fn selection_changed(&mut self, g: &mut Game) {
        let matchtype = g.game_values.matchtype;
        g.ctl(self.miTournamentField).set_visible(matchtype == MatchType::Tournament);
        g.ctl(self.miTourField).set_visible(matchtype == MatchType::Tour);
        g.ctl(self.miWorldField).set_visible(matchtype == MatchType::World);
        g.ctl(self.miMinigameField).set_visible(matchtype == MatchType::MiniGame);

        // miMatchSelectionDisplayImage->Show(game_values.matchtype != MatchType::World);
        g.ctl(self.miWorldPreviewDisplay).set_visible(matchtype == MatchType::World);

        if matchtype == MatchType::World {
            g.ctl(self.miMatchSelectionDisplayImage).set_image(320, 0, 320, 240);
        } else {
            g.ctl(self.miMatchSelectionDisplayImage).set_image(0, (240 * matchtype as i32) as i16, 320, 240);
        }
    }

    pub fn world_map_changed(&mut self, g: &mut Game) {
        g.ctl(self.miWorldPreviewDisplay).set_world(g);
    }

    pub fn activate_minigame_field(&mut self, g: &mut Game) {
        g.ctl(self.miMatchSelectionField).hide_item(MatchType::MiniGame, false);
        g.ctl(self.miMatchSelectionField).set_current_value(MatchType::MiniGame);

        g.ctl(self.miTournamentField).set_visible(false);
        g.ctl(self.miTourField).set_visible(false);
        g.ctl(self.miWorldField).set_visible(false);
        g.ctl(self.miMinigameField).set_visible(true);

        g.ctl(self.miMatchSelectionDisplayImage).set_visible(true);
        g.ctl(self.miWorldPreviewDisplay).set_visible(false);
        g.ctl(self.miMatchSelectionDisplayImage).set_image(0, (240 * g.game_values.matchtype as i32) as i16, 320, 240);
    }

    pub fn get_minigame(&self, g: &Game) -> Minigame {
        g.ctl(self.miMinigameField).current_value()
    }

    pub fn get_selected_match_type(&self, g: &Game) -> MatchType {
        g.ctl(self.miMatchSelectionField).current_value()
    }
}
