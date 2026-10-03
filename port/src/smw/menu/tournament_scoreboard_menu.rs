//! Port of src/smw/menu/TournamentScoreboardMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_tournament_scoreboard::MI_TournamentScoreboard;

#[derive(Default)]
pub struct UI_TournamentScoreboardMenu {
    pub ui_menu: UI_Menu,

    pub miTournamentScoreboard: Ptr<MI_TournamentScoreboard>,

    pub miTournamentScoreboardNextButton: Ptr<MI_Button>,
    pub miTournamentScoreboardImage: Ptr<MI_Image>,
}
crate::impl_base!(UI_TournamentScoreboardMenu => ui_menu: UI_Menu);

impl UI_TournamentScoreboardMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            this.miTournamentScoreboard = Ptr::new_box(MI_TournamentScoreboard::new(Ptr::from_mut(&mut rm.spr_tournament_background), 70, 98));

            this.miTournamentScoreboardNextButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 220, 416, "Next", 200, TextAlign::CENTER));
            this.miTournamentScoreboardNextButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU);

            this.miTournamentScoreboardImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_scoreboard), 124, 12, 0, 0, 386, 59, 1, 1, 0));
        }

        let sb = ctl_ptr(this.miTournamentScoreboard);
        let img = ctl_ptr(this.miTournamentScoreboardImage);
        let next = ctl_ptr(this.miTournamentScoreboardNextButton);
        this.add_non_control(sb);
        this.add_non_control(img);
        this.add_control(next, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());
        this.set_initial_focus(next);
        this.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU);
        this
    }
}
