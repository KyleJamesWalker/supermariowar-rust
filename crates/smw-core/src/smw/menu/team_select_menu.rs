//! The team and character selection screen before match setup.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_team_select::MI_TeamSelect;

#[derive(Default)]
/// Holds the team select control; cancel returns to match selection.
pub struct UI_TeamSelectMenu {
    pub ui_menu: UI_Menu,

    pub miTeamSelect: Ctl<MI_TeamSelect>,

    pub miTeamSelectLeftHeaderBar: Ctl<MI_Image>,
    pub miTeamSelectRightHeaderBar: Ctl<MI_Image>,
    pub miTeamSelectHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_TeamSelectMenu => ui_menu: UI_Menu);

impl UI_TeamSelectMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();
        let menu_plain_field = g.rm.menu_plain_field;

        this.miTeamSelect = Ctl::new(MI_TeamSelect::new(g, gp.rm.spr_player_select_background, 112, 96), g);
        g.ctl(this.miTeamSelect).set_auto_modify(true);

        this.miTeamSelectLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miTeamSelectRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miTeamSelectHeaderText = Ctl::new(MI_HeaderText::new(g, "Team and Character Selection", 320, 5), g);

        let ts = this.miTeamSelect.id();
        this.add_control(g, ts, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);

        let (l, r, h) = (this.miTeamSelectLeftHeaderBar.id(), this.miTeamSelectRightHeaderBar.id(), this.miTeamSelectHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, ts);
        this.set_cancel_code(MENU_CODE_BACK_TO_MATCH_SELECTION_MENU);
        this
    }

    /// Syncs teams with who is playing, splits a lone team and clears human players' ready state.
    pub fn reset_team_select(&mut self, g: &mut Game) {
        g.ctl(self.miTeamSelect).reset(g);
    }

    /// As [`Self::reset_team_select`], but players already on a team keep their ready state.
    pub fn players_changed(&mut self, g: &mut Game) {
        g.ctl(self.miTeamSelect).players_changed(g);
    }

    /// Packs the chosen teams into `game_values.teamids`/`teamcounts` and returns how many are non-empty.
    pub fn get_team_count(&self, g: &mut Game) -> i16 {
        let mut ts = self.miTeamSelect;
        g.ctl(ts).organize_teams(g)
    }
}
