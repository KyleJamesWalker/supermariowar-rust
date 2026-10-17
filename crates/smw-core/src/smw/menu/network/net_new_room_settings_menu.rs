//! Port of src/smw/menu/network/NetNewRoomSettingsMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::game_mode::GAMEMODE_LAST;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_image_select_field::MI_ImageSelectField;
use crate::common::ui::mi_map_field::MI_MapField;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uimenu::UI_Menu;
use crate::globals::Ptr;
use crate::smw::menu::game_settings_menu::UI_GameSettingsMenu;

const GM_LAST: usize = GAMEMODE_LAST as usize;

#[derive(Default)]
pub struct UI_NetNewRoomSettingsMenu {
    pub ui_menu: UI_Menu,

    miModeField: Ctl<MI_ImageSelectField>,
    miGoalField: [Ctl<MI_SelectField<i16>>; GM_LAST],
    miModeSettingsButton: Ctl<MI_Button>,
    /// Not in the C++: roll the mode settings when the match starts, as upstream always did.
    miRandomizeField: Ctl<MI_SelectField<bool>>,
    miMapField: Ctl<MI_MapField>,
    miContinueButton: Ctl<MI_Button>,

    miLeftHeaderBar: Ctl<MI_Image>,
    miRightHeaderBar: Ctl<MI_Image>,
    miHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_NetNewRoomSettingsMenu => ui_menu: UI_Menu);

impl UI_NetNewRoomSettingsMenu {
    pub fn new(g: &mut Game, gsm: &UI_GameSettingsMenu) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let selectfield = g.rm.spr_selectfield;

        this.miContinueButton = Ctl::new(MI_Button::new_left(g, selectfield, 70, 45, "Continue", 500), g);
        g.ctl(this.miContinueButton).set_code(MENU_CODE_TO_NET_NEW_ROOM_SETTINGS_MENU);

        this.miModeField = Ctl::new(MI_ImageSelectField::copy_from(g.ctl(gsm.miModeField)), g);
        for iGoalField in 0..GM_LAST {
            this.miGoalField[iGoalField] = Ctl::new(MI_SelectField::copy_from(g.ctl(gsm.miGoalField[iGoalField])), g);
        }

        this.miMapField = Ctl::new(MI_MapField::new(g, selectfield, 70, 165, "Map", 400, 120, true), g);
        g.netplay.mapfilepath = this.get_current_map_path(g);

        this.miModeSettingsButton = Ctl::new(MI_Button::new_left(g, selectfield, 430, 125, "Settings", 140), g);
        g.ctl(this.miModeSettingsButton).set_code(MENU_CODE_TO_MODE_SETTINGS_MENU);

        this.miRandomizeField = Ctl::new(MI_SelectField::<bool>::new(selectfield, 440, 205, "Random", 130, 80), g);
        g.ctl(this.miRandomizeField).add("Off", false);
        g.ctl(this.miRandomizeField).add("On", true);
        g.ctl(this.miRandomizeField).set_output_ptr(&mut g.netplay.randomize_settings as *mut _);
        g.ctl(this.miRandomizeField).set_current_value(g.netplay.randomize_settings);

        let cont = this.miContinueButton.id();
        let mode = this.miModeField.id();
        let goals: Vec<_> = this.miGoalField.iter().map(|c| c.id()).collect();
        let settings = this.miModeSettingsButton.id();
        let map = this.miMapField.id();
        let randomize = this.miRandomizeField.id();
        let null = ControlId::NONE;

        this.add_control(g, cont, map, mode, null, null);
        this.add_control(g, mode, cont, goals[0], null, null);
        this.add_control(g, goals[0], mode, goals[1], null, settings);
        for iGoalField in 1..GM_LAST - 1 {
            this.add_control(g, goals[iGoalField], goals[iGoalField - 1], goals[iGoalField + 1], goals[iGoalField - 1], settings);
        }
        this.add_control(g, goals[GM_LAST - 1], goals[GM_LAST - 2], map, goals[GM_LAST - 2], null);
        this.add_control(g, settings, mode, randomize, goals[GM_LAST - 1], randomize);
        this.add_control(g, randomize, settings, map, settings, null);
        this.add_control(g, map, goals[GM_LAST - 1], cont, null, null);

        let plain = g.rm.menu_plain_field;
        this.miLeftHeaderBar = Ctl::new(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.miRightHeaderBar = Ctl::new(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.miHeaderText = Ctl::new(MI_HeaderText::new(g, "Multiplayer Level Select Menu", 320, 5), g);

        let (l, r, h) = (this.miLeftHeaderBar.id(), this.miRightHeaderBar.id(), this.miHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(g, cont);
        this.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
        this
    }

    pub fn refresh_game_mode_buttons(&mut self, g: &mut Game) {
        let current = g.ctl(self.miModeField).current_value();
        for iMode in 0..GM_LAST {
            g.ctl(self.miGoalField[iMode]).set_visible(current == iMode as i16);
        }
    }

    pub fn get_current_map_path(&self, g: &mut Game) -> String {
        g.ctl(self.miMapField).get_map_file_path(g)
    }

    pub fn get_selected_game_mode_id(&self, g: &Game) -> i16 {
        for iGameMode in 0..GM_LAST {
            if g.ctl(self.miGoalField[iGameMode]).is_visible() {
                return iGameMode as i16;
            }
        }

        0
    }
}
