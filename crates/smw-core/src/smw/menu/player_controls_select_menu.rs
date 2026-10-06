//! Port of src/smw/menu/PlayerControlsSelectMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;

/*
    This menu leads to the control settings of player 1-4.
*/
#[derive(Default)]
pub struct UI_PlayerControlsSelectMenu {
    pub ui_menu: UI_Menu,

    pub miPlayer1ControlsButton: Ptr<MI_Button>,
    pub miPlayer2ControlsButton: Ptr<MI_Button>,
    pub miPlayer3ControlsButton: Ptr<MI_Button>,
    pub miPlayer4ControlsButton: Ptr<MI_Button>,

    pub miPlayerControlsBackButton: Ptr<MI_Button>,

    pub miPlayerControlsLeftHeaderBar: Ptr<MI_Image>,
    pub miPlayerControlsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miPlayerControlsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_PlayerControlsSelectMenu => ui_menu: UI_Menu);

impl UI_PlayerControlsSelectMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miPlayer1ControlsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 140, "Player 1", 400, TextAlign::CENTER));
            this.miPlayer1ControlsButton.set_code(MENU_CODE_TO_PLAYER_1_CONTROLS);

            this.miPlayer2ControlsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 180, "Player 2", 400, TextAlign::CENTER));
            this.miPlayer2ControlsButton.set_code(MENU_CODE_TO_PLAYER_2_CONTROLS);

            this.miPlayer3ControlsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 220, "Player 3", 400, TextAlign::CENTER));
            this.miPlayer3ControlsButton.set_code(MENU_CODE_TO_PLAYER_3_CONTROLS);

            this.miPlayer4ControlsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 120, 260, "Player 4", 400, TextAlign::CENTER));
            this.miPlayer4ControlsButton.set_code(MENU_CODE_TO_PLAYER_4_CONTROLS);

            this.miPlayerControlsBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miPlayerControlsBackButton.set_code(MENU_CODE_TO_MAIN_MENU);

            this.miPlayerControlsLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miPlayerControlsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miPlayerControlsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Player Controls Menu", 320, 5));
        }

        let p1 = ctl_ptr(this.miPlayer1ControlsButton);
        let p2 = ctl_ptr(this.miPlayer2ControlsButton);
        let p3 = ctl_ptr(this.miPlayer3ControlsButton);
        let p4 = ctl_ptr(this.miPlayer4ControlsButton);
        let back = ctl_ptr(this.miPlayerControlsBackButton);

        this.add_control(p1, back, p2, Ptr::null(), back);
        this.add_control(p2, p1, p3, Ptr::null(), back);
        this.add_control(p3, p2, p4, Ptr::null(), back);
        this.add_control(p4, p3, back, Ptr::null(), back);
        this.add_control(back, p4, p1, p1, Ptr::null());

        let (l, r, h) = (ctl_ptr(this.miPlayerControlsLeftHeaderBar), ctl_ptr(this.miPlayerControlsMenuRightHeaderBar), ctl_ptr(this.miPlayerControlsMenuHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(p1);
        this.set_cancel_code(MENU_CODE_TO_MAIN_MENU);
        this
    }
}
