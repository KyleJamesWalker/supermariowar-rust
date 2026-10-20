//! The controls menu: one button per player into their control settings.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;

/*
    This menu leads to the control settings of player 1-4.
*/
#[derive(Default)]
pub struct UI_PlayerControlsSelectMenu {
    pub ui_menu: UI_Menu,

    pub miPlayer1ControlsButton: Ctl<MI_Button>,
    pub miPlayer2ControlsButton: Ctl<MI_Button>,
    pub miPlayer3ControlsButton: Ctl<MI_Button>,
    pub miPlayer4ControlsButton: Ctl<MI_Button>,

    pub miPlayerControlsBackButton: Ctl<MI_Button>,

    pub miPlayerControlsLeftHeaderBar: Ctl<MI_Image>,
    pub miPlayerControlsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miPlayerControlsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_PlayerControlsSelectMenu => ui_menu: UI_Menu);

impl UI_PlayerControlsSelectMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;
        let menu_plain_field = g.rm.menu_plain_field;

        this.miPlayer1ControlsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 140, "Player 1", 400, TextAlign::CENTER), g);
        g.ctl(this.miPlayer1ControlsButton).set_code(MENU_CODE_TO_PLAYER_1_CONTROLS);

        this.miPlayer2ControlsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 180, "Player 2", 400, TextAlign::CENTER), g);
        g.ctl(this.miPlayer2ControlsButton).set_code(MENU_CODE_TO_PLAYER_2_CONTROLS);

        this.miPlayer3ControlsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 220, "Player 3", 400, TextAlign::CENTER), g);
        g.ctl(this.miPlayer3ControlsButton).set_code(MENU_CODE_TO_PLAYER_3_CONTROLS);

        this.miPlayer4ControlsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 120, 260, "Player 4", 400, TextAlign::CENTER), g);
        g.ctl(this.miPlayer4ControlsButton).set_code(MENU_CODE_TO_PLAYER_4_CONTROLS);

        this.miPlayerControlsBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miPlayerControlsBackButton).set_code(MENU_CODE_TO_MAIN_MENU);

        this.miPlayerControlsLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miPlayerControlsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miPlayerControlsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Player Controls Menu", 320, 5), g);

        let p1 = this.miPlayer1ControlsButton.id();
        let p2 = this.miPlayer2ControlsButton.id();
        let p3 = this.miPlayer3ControlsButton.id();
        let p4 = this.miPlayer4ControlsButton.id();
        let back = this.miPlayerControlsBackButton.id();

        this.add_control(g, p1, back, p2, Default::default(), back);
        this.add_control(g, p2, p1, p3, Default::default(), back);
        this.add_control(g, p3, p2, p4, Default::default(), back);
        this.add_control(g, p4, p3, back, Default::default(), back);
        this.add_control(g, back, p4, p1, p1, Default::default());

        let (l, r, h) = (this.miPlayerControlsLeftHeaderBar.id(), this.miPlayerControlsMenuRightHeaderBar.id(), this.miPlayerControlsMenuHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, p1);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }
}
