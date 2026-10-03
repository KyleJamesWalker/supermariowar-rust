//! Port of src/smw/menu/network/NetNewRoomSettingsMenu.cpp

use crate::common::game_mode::GAMEMODE_LAST;
use crate::common::global::rm;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_image_select_field::MI_ImageSelectField;
use crate::common::ui::mi_map_field::MI_MapField;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::ctl_ptr;
use crate::common::uimenu::UI_Menu;
use crate::globals::Ptr;
use crate::smw::menu::game_settings_menu::UI_GameSettingsMenu;
use crate::smw::net::netplay;

const GM_LAST: usize = GAMEMODE_LAST as usize;

#[derive(Default)]
pub struct UI_NetNewRoomSettingsMenu {
    pub ui_menu: UI_Menu,

    miModeField: Ptr<MI_ImageSelectField>,
    miGoalField: [Ptr<MI_SelectField<i16>>; GM_LAST],
    miModeSettingsButton: Ptr<MI_Button>,
    miMapField: Ptr<MI_MapField>,
    miContinueButton: Ptr<MI_Button>,

    miLeftHeaderBar: Ptr<MI_Image>,
    miRightHeaderBar: Ptr<MI_Image>,
    miHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_NetNewRoomSettingsMenu => ui_menu: UI_Menu);

impl UI_NetNewRoomSettingsMenu {
    pub fn new(gsm: &UI_GameSettingsMenu) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let selectfield = Ptr::from_mut(&mut rm.spr_selectfield);

            this.miContinueButton = Ptr::new_box(MI_Button::new_left(selectfield, 70, 45, "Continue", 500));
            this.miContinueButton.set_code(MENU_CODE_TO_NET_NEW_ROOM_SETTINGS_MENU);

            this.miModeField = Ptr::new_box(MI_ImageSelectField::copy_from(&gsm.miModeField));
            for iGoalField in 0..GM_LAST {
                this.miGoalField[iGoalField] = Ptr::new_box(MI_SelectField::copy_from(&gsm.miGoalField[iGoalField]));
            }

            this.miMapField = Ptr::new_box(MI_MapField::new(selectfield, 70, 165, "Map", 400, 120, true));
            netplay.mapfilepath = this.get_current_map_path();

            this.miModeSettingsButton = Ptr::new_box(MI_Button::new_left(selectfield, 430, 125, "Settings", 140));
            this.miModeSettingsButton.set_code(MENU_CODE_TO_MODE_SETTINGS_MENU);
        }

        let cont = ctl_ptr(this.miContinueButton);
        let mode = ctl_ptr(this.miModeField);
        let goals: Vec<_> = this.miGoalField.iter().map(|g| ctl_ptr(*g)).collect();
        let settings = ctl_ptr(this.miModeSettingsButton);
        let map = ctl_ptr(this.miMapField);
        let null = Ptr::null();

        this.add_control(cont, map, mode, null, null);
        this.add_control(mode, cont, goals[0], null, null);
        this.add_control(goals[0], mode, goals[1], null, settings);
        for iGoalField in 1..GM_LAST - 1 {
            this.add_control(goals[iGoalField], goals[iGoalField - 1], goals[iGoalField + 1], goals[iGoalField - 1], settings);
        }
        this.add_control(goals[GM_LAST - 1], goals[GM_LAST - 2], map, goals[GM_LAST - 2], null);
        this.add_control(settings, mode, map, goals[GM_LAST - 1], null);
        this.add_control(map, goals[GM_LAST - 1], cont, null, null);

        unsafe {
            let plain = Ptr::from_mut(&mut rm.menu_plain_field);
            this.miLeftHeaderBar = Ptr::new_box(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miRightHeaderBar = Ptr::new_box(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miHeaderText = Ptr::new_box(MI_HeaderText::new("Multiplayer Level Select Menu", 320, 5));
        }

        let (l, r, h) = (ctl_ptr(this.miLeftHeaderBar), ctl_ptr(this.miRightHeaderBar), ctl_ptr(this.miHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);

        this.set_initial_focus(cont);
        this.set_cancel_code(MENU_CODE_TO_NET_LOBBY_MENU);
        this
    }

    pub fn refresh_game_mode_buttons(&mut self) {
        let current = self.miModeField.current_value();
        for iMode in 0..GM_LAST {
            self.miGoalField[iMode].set_visible(current == iMode as i16);
        }
    }

    pub fn get_current_map_path(&self) -> String {
        self.miMapField.get_map_file_path()
    }

    pub fn get_selected_game_mode_id(&self) -> i16 {
        for iGameMode in 0..GM_LAST {
            if self.miGoalField[iGameMode].is_visible() {
                return iGameMode as i16;
            }
        }

        0
    }
}
