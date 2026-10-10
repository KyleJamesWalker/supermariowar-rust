//! Port of src/smw/menu/MainMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_player_select::MI_PlayerSelect;

/*
    This is the main menu you see at the start of the game.
*/
#[derive(Default)]
pub struct UI_MainMenu {
    pub ui_menu: UI_Menu,

    miSMWTitle: Ptr<MI_Image>,
    miSMWVersion: Ptr<MI_Image>,

    miMainStartButton: Ptr<MI_Button>,
    miQuickGameButton: Ptr<MI_Button>,
    miMultiplayerButton: Ptr<MI_Button>,

    miPlayerSelect: Ptr<MI_PlayerSelect>,

    miOptionsButton: Ptr<MI_Button>,
    miControlsButton: Ptr<MI_Button>,

    miExitButton: Ptr<MI_Button>,
}
crate::impl_base!(UI_MainMenu => ui_menu: UI_Menu);

impl UI_MainMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let selectfield = Ptr::from_mut(&mut rm.spr_selectfield);

            this.miSMWTitle = Ptr::new_box(MI_Image::new(
                Ptr::from_mut(&mut rm.menu_smw),
                (320 - ((rm.menu_smw.get_width() as i16) >> 1) as i32) as i16,
                30,
                0,
                0,
                372,
                140,
                1,
                1,
                0,
            ));
            this.miSMWVersion = Ptr::new_box(MI_Image::new(
                Ptr::from_mut(&mut rm.menu_version),
                (628 - rm.menu_version.get_width()) as i16,
                10,
                0,
                0,
                rm.menu_version.get_width() as i16,
                rm.menu_version.get_height() as i16,
                1,
                1,
                0,
            ));
            // miSMWVersionText = new MI_Text("Beta 2", 630, 45, 0, 2, 2);

            this.miMainStartButton = Ptr::new_box(MI_Button::new(selectfield, 120, 210, "Start", 310, TextAlign::LEFT));
            this.miMainStartButton.set_code(MENU_CODE_TO_MATCH_SELECTION_MENU);

            this.miQuickGameButton = Ptr::new_box(MI_Button::new(selectfield, 440, 210, "Go!", 80, TextAlign::LEFT));
            this.miQuickGameButton.set_code(MENU_CODE_QUICK_GAME_START);

            this.miPlayerSelect = Ptr::new_box(MI_PlayerSelect::new(Ptr::from_mut(&mut rm.menu_player_select), 120, 250, "Players", 400, 140));

            // Upstream's web build disables Multiplayer, stacks Options over Controls and has no Exit. Here every
            // build uses the native layout, so replays navigate the same; on the web Exit ends the session.
            this.miMultiplayerButton = Ptr::new_box(MI_Button::new(selectfield, 120, 322, "Multiplayer", 400, TextAlign::LEFT));
            this.miMultiplayerButton.set_code(MENU_CODE_TO_NET_SERVERS_MENU);

            this.miOptionsButton = Ptr::new_box(MI_Button::new(selectfield, 120, 362, "Options", 200, TextAlign::LEFT));
            this.miControlsButton = Ptr::new_box(MI_Button::new(selectfield, 320, 362, "Controls", 200, TextAlign::LEFT));

            this.miOptionsButton.set_code(MENU_CODE_TO_OPTIONS_MENU);
            this.miControlsButton.set_code(MENU_CODE_TO_CONTROLS_MENU);

            this.miExitButton = Ptr::new_box(MI_Button::new(selectfield, 120, 402, "Exit", (640.0f32 * 0.625f32) as i16, TextAlign::LEFT));
            this.miExitButton.set_code(MENU_CODE_EXIT_APPLICATION);
        }

        let start = ctl_ptr(this.miMainStartButton);
        let quick = ctl_ptr(this.miQuickGameButton);
        let players = ctl_ptr(this.miPlayerSelect);
        let multi = ctl_ptr(this.miMultiplayerButton);
        let options = ctl_ptr(this.miOptionsButton);
        let controls = ctl_ptr(this.miControlsButton);
        let null = Ptr::null();

        let exit = ctl_ptr(this.miExitButton);
        this.add_control(start, exit, players, null, quick);
        this.add_control(quick, exit, players, start, null);
        this.add_control(players, start, multi, null, null);
        this.add_control(multi, players, options, null, null);
        this.add_control(options, multi, exit, controls, controls);
        this.add_control(controls, multi, exit, options, options);
        this.add_control(exit, options, start, null, null);

        this.set_initial_focus(start);
        // A browser tab cannot be closed from the page, so Escape on the web main menu does nothing.
        #[cfg(not(target_os = "emscripten"))]
        this.set_cancel_code(MENU_CODE_EXIT_APPLICATION);

        let (title, version) = (ctl_ptr(this.miSMWTitle), ctl_ptr(this.miSMWVersion));
        this.add_non_control(title);
        this.add_non_control(version);

        this
    }
}
