//! Port of src/smw/menu/options/TeamOptionsMenu.cpp

use crate::common::gameplay_styles::{TeamCollisionStyle, TournamentControlStyle};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;

/*
    In this menu, you can set team-relates preferences,
    such as friendly collision or team color drawing.
*/
#[derive(Default)]
pub struct UI_TeamOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miTeamKillsField: Ptr<MI_SelectField<TeamCollisionStyle>>,
    pub miTeamColorsField: Ptr<MI_SelectField<bool>>,
    pub miTournamentControlField: Ptr<MI_SelectField<TournamentControlStyle>>,
    pub miTeamOptionsMenuBackButton: Ptr<MI_Button>,

    pub miTeamOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miTeamOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miTeamOptionsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_TeamOptionsMenu => ui_menu: UI_Menu);

impl UI_TeamOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miTeamKillsField = select_field(spr, 70, 180, "Player Collision", 500, 220,
                &[("Off", TeamCollisionStyle::Off), ("Assist", TeamCollisionStyle::Assist), ("On", TeamCollisionStyle::On)],
                &mut game_values.teamcollision);

            this.miTeamColorsField = select_field(spr, 70, 220, "Colors", 500, 220,
                &[("Individual", false), ("Team", true)],
                &mut game_values.teamcolors);
            this.miTeamColorsField.set_auto_advance(true);

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
                &mut game_values.tournamentcontrolstyle);

            this.miTeamOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miTeamOptionsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miTeamOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miTeamOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miTeamOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Team Options Menu", 320, 5));
        }

        let kills = ctl_ptr(this.miTeamKillsField);
        let colors = ctl_ptr(this.miTeamColorsField);
        let control = ctl_ptr(this.miTournamentControlField);
        let back = ctl_ptr(this.miTeamOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(kills, back, colors, null, back);
        this.add_control(colors, kills, control, null, back);
        this.add_control(control, colors, back, null, back);
        this.add_control(back, control, kills, control, null);

        let c = ctl_ptr(this.miTeamOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miTeamOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miTeamOptionsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(kills);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
