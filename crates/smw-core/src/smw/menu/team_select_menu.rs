//! Port of src/smw/menu/TeamSelectMenu.cpp

use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::ctl_ptr;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_team_select::MI_TeamSelect;

#[derive(Default)]
pub struct UI_TeamSelectMenu {
    pub ui_menu: UI_Menu,

    pub miTeamSelect: Ptr<MI_TeamSelect>,

    pub miTeamSelectLeftHeaderBar: Ptr<MI_Image>,
    pub miTeamSelectRightHeaderBar: Ptr<MI_Image>,
    pub miTeamSelectHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_TeamSelectMenu => ui_menu: UI_Menu);

impl UI_TeamSelectMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        unsafe {
            let menu_plain_field = Ptr::from_mut(&mut g.rm.menu_plain_field);

            this.miTeamSelect = Ptr::new_box(MI_TeamSelect::new(g, Ptr::from_mut(&mut gp.rm.spr_player_select_background), 112, 96));
            this.miTeamSelect.set_auto_modify(true);

            this.miTeamSelectLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miTeamSelectRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miTeamSelectHeaderText = Ptr::new_box(MI_HeaderText::new("Team and Character Selection", 320, 5));
        }

        let ts = ctl_ptr(this.miTeamSelect);
        this.add_control(ts, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());

        let (l, r, h) = (ctl_ptr(this.miTeamSelectLeftHeaderBar), ctl_ptr(this.miTeamSelectRightHeaderBar), ctl_ptr(this.miTeamSelectHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(ts);
        this.set_cancel_code(MENU_CODE_BACK_TO_MATCH_SELECTION_MENU);
        this
    }

    pub fn reset_team_select(&mut self, g: &mut Game) {
        self.miTeamSelect.reset(g);
    }

    pub fn get_team_count(&self, g: &mut Game) -> i16 {
        let mut ts = self.miTeamSelect;
        ts.organize_teams(g)
    }
}
