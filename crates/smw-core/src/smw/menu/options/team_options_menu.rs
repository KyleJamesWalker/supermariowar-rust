//! Team options: friendly collision, team colors and tournament control.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::gameplay_styles::{TeamCollisionStyle, TournamentControlStyle};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;

/// Team preferences such as friendly collision and team colors.
#[derive(Default)]
pub struct UI_TeamOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miTeamKillsField: Ctl<MI_SelectField<TeamCollisionStyle>>,
    pub miTeamColorsField: Ctl<MI_SelectField<bool>>,
    pub miTournamentControlField: Ctl<MI_SelectField<TournamentControlStyle>>,
    pub miTeamOptionsMenuBackButton: Ctl<MI_Button>,

    pub miTeamOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miTeamOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miTeamOptionsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_TeamOptionsMenu => ui_menu: UI_Menu);

impl UI_TeamOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miTeamKillsField = select_field(spr, 70, 180, "Player Collision", 500, 220,
                &[("Off", TeamCollisionStyle::Off), ("Assist", TeamCollisionStyle::Assist), ("On", TeamCollisionStyle::On)],
                &mut g.game_values.teamcollision, g);

            this.miTeamColorsField = select_field(spr, 70, 220, "Colors", 500, 220,
                &[("Individual", false), ("Team", true)],
                &mut g.game_values.teamcolors, g);
            g.ctl(this.miTeamColorsField).set_auto_advance(true);

            this.miTournamentControlField = select_field(spr, 70, 260, "Tournament Control", 500, 220,
                &[
                    ("All", TournamentControlStyle::All),
                    ("Game Winner", TournamentControlStyle::GameWinner),
                    ("Game Loser", TournamentControlStyle::GameLoser),
                    ("Leading Teams", TournamentControlStyle::LeadingTeams),
                    ("Trailing Teams", TournamentControlStyle::TrailingTeams),
                    ("Random", TournamentControlStyle::Random),
                    ("Random Loser", TournamentControlStyle::RandomLoser),
                    ("Round Robin", TournamentControlStyle::RoundRobin),
                ],
                &mut g.game_values.tournamentcontrolstyle, g);

            this.miTeamOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miTeamOptionsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miTeamOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miTeamOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miTeamOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Team Options Menu", 320, 5), g);
        }

        let kills = this.miTeamKillsField.id();
        let colors = this.miTeamColorsField.id();
        let control = this.miTournamentControlField.id();
        let back = this.miTeamOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, kills, back, colors, null, back);
        this.add_control(g, colors, kills, control, null, back);
        this.add_control(g, control, colors, back, null, back);
        this.add_control(g, back, control, kills, control, null);

        let c = this.miTeamOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miTeamOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miTeamOptionsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, kills);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
