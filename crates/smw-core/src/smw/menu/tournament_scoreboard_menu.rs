//! Port of src/smw/menu/TournamentScoreboardMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_tournament_scoreboard::MI_TournamentScoreboard;

#[derive(Default)]
pub struct UI_TournamentScoreboardMenu {
    pub ui_menu: UI_Menu,

    pub miTournamentScoreboard: Ctl<MI_TournamentScoreboard>,

    pub miTournamentScoreboardNextButton: Ctl<MI_Button>,
    pub miTournamentScoreboardImage: Ctl<MI_Image>,
}
crate::impl_base!(UI_TournamentScoreboardMenu => ui_menu: UI_Menu);

impl UI_TournamentScoreboardMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        this.miTournamentScoreboard = Ctl::new(MI_TournamentScoreboard::new(g.rm.spr_tournament_background, 70, 98), g);

        this.miTournamentScoreboardNextButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 220, 416, "Next", 200, TextAlign::CENTER), g);
        g.ctl(this.miTournamentScoreboardNextButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU);

        this.miTournamentScoreboardImage = Ctl::new(MI_Image::new(g.rm.spr_scoreboard, 124, 12, 0, 0, 386, 59, 1, 1, 0), g);

        let sb = this.miTournamentScoreboard.id();
        let img = this.miTournamentScoreboardImage.id();
        let next = this.miTournamentScoreboardNextButton.id();
        this.add_non_control(sb);
        this.add_non_control(img);
        this.add_control(g, next, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);
        this.set_initial_focus(g, next);
        this.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU);
        this
    }
}
