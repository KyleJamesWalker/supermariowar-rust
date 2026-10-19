//! Port of src/smw/menu/MainMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::MI_Text;
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_player_select::MI_PlayerSelect;

/*
    This is the main menu you see at the start of the game.
*/
#[derive(Default)]
pub struct UI_MainMenu {
    pub ui_menu: UI_Menu,

    miSMWTitle: Ctl<MI_Image>,
    miSMWVersion: Ctl<MI_Image>,
    miBuildVersion: Ctl<MI_Text>,

    miMainStartButton: Ctl<MI_Button>,
    miQuickGameButton: Ctl<MI_Button>,
    miMultiplayerButton: Ctl<MI_Button>,

    miPlayerSelect: Ctl<MI_PlayerSelect>,

    miOptionsButton: Ctl<MI_Button>,
    miControlsButton: Ctl<MI_Button>,

    miExitButton: Ctl<MI_Button>,
}
crate::impl_base!(UI_MainMenu => ui_menu: UI_Menu);

impl UI_MainMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let selectfield = g.rm.spr_selectfield;

        this.miSMWTitle = Ctl::new(MI_Image::new(
            g.rm.menu_smw,
            (320 - ((g.rm.sprite(g.rm.menu_smw).get_width() as i16) >> 1) as i32) as i16,
            30,
            0,
            0,
            372,
            140,
            1,
            1,
            0,
        ), g);
        this.miSMWVersion = Ctl::new(MI_Image::new(
            g.rm.menu_version,
            (628 - g.rm.sprite(g.rm.menu_version).get_width()) as i16,
            10,
            0,
            0,
            g.rm.sprite(g.rm.menu_version).get_width() as i16,
            g.rm.sprite(g.rm.menu_version).get_height() as i16,
            1,
            1,
            0,
        ), g);
        this.miBuildVersion = Ctl::new(MI_Text::new(g, build_version(), 628, 45, 0, false, TextAlign::RIGHT), g);

        this.miMainStartButton = Ctl::new(MI_Button::new(g, selectfield, 120, 210, "Start", 310, TextAlign::LEFT), g);
        g.ctl(this.miMainStartButton).set_code(MENU_CODE_TO_MATCH_SELECTION_MENU);

        this.miQuickGameButton = Ctl::new(MI_Button::new(g, selectfield, 440, 210, "Go!", 80, TextAlign::LEFT), g);
        g.ctl(this.miQuickGameButton).set_code(MENU_CODE_QUICK_GAME_START);

        this.miPlayerSelect = Ctl::new(MI_PlayerSelect::new(g.rm.menu_player_select, 120, 250, "Players", 400, 140), g);

        // Upstream's web build disables Multiplayer, stacks Options over Controls and has no Exit. Here every
        // build uses the native layout, so replays navigate the same; on the web Exit ends the session.
        this.miMultiplayerButton = Ctl::new(MI_Button::new(g, selectfield, 120, 322, "Multiplayer", 400, TextAlign::LEFT), g);
        g.ctl(this.miMultiplayerButton).set_code(MENU_CODE_TO_NET_SERVERS_MENU);

        this.miOptionsButton = Ctl::new(MI_Button::new(g, selectfield, 120, 362, "Options", 200, TextAlign::LEFT), g);
        this.miControlsButton = Ctl::new(MI_Button::new(g, selectfield, 320, 362, "Controls", 200, TextAlign::LEFT), g);

        g.ctl(this.miOptionsButton).set_code(MENU_CODE_TO_OPTIONS_MENU);
        g.ctl(this.miControlsButton).set_code(MENU_CODE_TO_CONTROLS_MENU);

        this.miExitButton = Ctl::new(MI_Button::new(g, selectfield, 120, 402, "Exit", (640.0f32 * 0.625f32) as i16, TextAlign::LEFT), g);
        g.ctl(this.miExitButton).set_code(MENU_CODE_EXIT_APPLICATION);

        let start = this.miMainStartButton.id();
        let quick = this.miQuickGameButton.id();
        let players = this.miPlayerSelect.id();
        let multi = this.miMultiplayerButton.id();
        let options = this.miOptionsButton.id();
        let controls = this.miControlsButton.id();
        let null = ControlId::NONE;

        let exit = this.miExitButton.id();
        this.add_control(g, start, exit, players, null, quick);
        this.add_control(g, quick, exit, players, start, null);
        this.add_control(g, players, start, multi, null, null);
        this.add_control(g, multi, players, options, null, null);
        this.add_control(g, options, multi, exit, controls, controls);
        this.add_control(g, controls, multi, exit, options, options);
        this.add_control(g, exit, options, start, null, null);

        this.set_initial_focus(g, start);
        // A browser tab cannot be closed from the page, so Escape on the web main menu does nothing.
        #[cfg(not(target_os = "emscripten"))]
        this.set_cancel_code(MENU_CODE_EXIT_APPLICATION);

        let (title, version, build) = (this.miSMWTitle.id(), this.miSMWVersion.id(), this.miBuildVersion.id());
        this.add_non_control(title);
        this.add_non_control(version);
        this.add_non_control(build);

        this
    }
}

/// The release tag, else the short commit, that CI built this binary from.
fn build_version() -> &'static str {
    match (option_env!("SMW_VERSION"), option_env!("SMW_COMMIT")) {
        (Some(tag), _) if !tag.is_empty() => tag,
        (_, Some(sha)) if sha.len() >= 7 => &sha[..7],
        _ => "dev",
    }
}
