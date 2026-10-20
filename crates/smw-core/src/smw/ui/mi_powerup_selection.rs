//! The item selection editor: override mode, preset, and a weight slider per powerup.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::game_values::default_powerup_setting;
use crate::common::global_constants::NUM_POWERUPS;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_powerup_slider::MI_PowerupSlider;

/// Powerup each slider sets, in screen order, two sliders per row.
const iPowerupPositionMap: [i16; NUM_POWERUPS as usize] = [1, 0, 2, 6, 3, 8, 4, 20, 18, 7, 5, 10, 11, 22, 19, 9, 23, 16, 21, 12, 17, 13, 24, 15, 25, 14];

const NP: usize = NUM_POWERUPS as usize;

/// A scrolling inner menu of item weight fields, editing `game_values.powerupweights` and the selected preset.
pub struct MI_PowerupSelection {
    pub ui_control: UI_Control,

    /// Focused row, counting the override and preset rows; drives scrolling.
    iIndex: i16,
    /// First slider row shown.
    iOffset: i16,
    /// Slider rows shown at once.
    iNumLines: i16,
    iTopStop: i16,
    iBottomStop: i16,

    /// Inner menu holding every field; input goes to it while this control is modified.
    mMenu: Box<UI_Menu>,

    /// Writes `overridepowerupsettings`: 0 map only, 1 game only, 2 basic average, 3 weighted average.
    miOverride: Ctl<MI_SelectField<i16>>,
    /// Writes `poweruppreset`, 0 to 16; 0 to 4 are the custom sets.
    miPreset: Ctl<MI_SelectField<i16>>,

    /// Weight sliders, 0 to 10; slider `i` sets powerup `iPowerupPositionMap[i]`.
    miPowerupSlider: [Ctl<MI_PowerupSlider>; NP],

    miRestoreDefaultsButton: Ctl<MI_Button>,
    miClearButton: Ctl<MI_Button>,

    /// Are You Sure dialog shared by Defaults and Clear.
    miDialogImage: Ctl<MI_Image>,
    miDialogAreYouText: Ctl<MI_Text>,
    miDialogSureText: Ctl<MI_Text>,
    miDialogYesButton: Ctl<MI_Button>,
    miDialogNoButton: Ctl<MI_Button>,

    miUpArrow: Ctl<MI_Image>,
    miDownArrow: Ctl<MI_Image>,
}
crate::impl_base!(MI_PowerupSelection => ui_control: UI_Control);

impl MI_PowerupSelection {
    /// `numlines` is the slider rows shown at once; `width` is unused.
    pub fn new(g: &mut Game, x: i16, y: i16, width: i16, numlines: i16) -> Self {
        let _ = width;
        let iNumLines = numlines;
        let iTopStop = ((iNumLines as i32 - 1) / 2 + 2) as i16;
        let iBottomStop = ((NUM_POWERUPS / 2) - iNumLines as i32 + iTopStop as i32) as i16;

        let mut this = MI_PowerupSelection {
            ui_control: UI_Control::new(x, y),
            iIndex: 0,
            iOffset: 0,
            iNumLines,
            iTopStop,
            iBottomStop,
            mMenu: Box::new(UI_Menu::new()),
            miOverride: Default::default(),
            miPreset: Default::default(),
            miPowerupSlider: [Default::default(); NP],
            miRestoreDefaultsButton: Default::default(),
            miClearButton: Default::default(),
            miDialogImage: Default::default(),
            miDialogAreYouText: Default::default(),
            miDialogSureText: Default::default(),
            miDialogYesButton: Default::default(),
            miDialogNoButton: Default::default(),
            miUpArrow: Default::default(),
            miDownArrow: Default::default(),
        };

        let spr_selectfield = g.rm.spr_selectfield;

        this.miOverride = Ctl::new(MI_SelectField::<i16>::new(spr_selectfield, 70, this.m_pos.y, "Use Settings From", 500, 250), g);
        g.ctl(this.miOverride).add("Map Only", 0);
        g.ctl(this.miOverride).add("Game Only", 1);
        g.ctl(this.miOverride).add("Basic Average", 2);
        g.ctl(this.miOverride).add("Weighted Average", 3);
        g.ctl(this.miOverride).set_output_ptr(&mut g.game_values.overridepowerupsettings as *mut i16);
        g.ctl(this.miOverride).set_current_value(g.game_values.overridepowerupsettings);

        this.miPreset = Ctl::new(MI_SelectField::<i16>::new(spr_selectfield, 70, this.m_pos.y + 40, "Item Set", 500, 250), g);
        g.ctl(this.miPreset).add("Custom Set 1", 0);
        g.ctl(this.miPreset).add("Custom Set 2", 1);
        g.ctl(this.miPreset).add("Custom Set 3", 2);
        g.ctl(this.miPreset).add("Custom Set 4", 3);
        g.ctl(this.miPreset).add("Custom Set 5", 4);
        g.ctl(this.miPreset).add("Balanced Set", 5);
        g.ctl(this.miPreset).add("Weapons Only", 6);
        g.ctl(this.miPreset).add("Koopa Bros Weapons", 7);
        g.ctl(this.miPreset).add("Support Items", 8);
        g.ctl(this.miPreset).add("Booms and Shakes", 9);
        g.ctl(this.miPreset).add("Fly and Glide", 10);
        g.ctl(this.miPreset).add("Shells Only", 11);
        g.ctl(this.miPreset).add("Mushrooms Only", 12);
        g.ctl(this.miPreset).add("Super Mario Bros 1", 13);
        g.ctl(this.miPreset).add("Super Mario Bros 2", 14);
        g.ctl(this.miPreset).add("Super Mario Bros 3", 15);
        g.ctl(this.miPreset).add("Super Mario World", 16);
        g.ctl(this.miPreset).set_output_ptr(&mut g.game_values.poweruppreset as *mut i16);
        g.ctl(this.miPreset).set_current_value(g.game_values.poweruppreset);
        g.ctl(this.miPreset).set_item_changed_code(MENU_CODE_POWERUP_PRESET_CHANGED);

        for iPowerup in 0..NP {
            let pos = iPowerupPositionMap[iPowerup] as usize;
            let mut slider = Ctl::new(MI_PowerupSlider::new(
                spr_selectfield,
                g.rm.menu_slider_bar,
                g.rm.spr_storedpoweruplarge,
                0,
                0,
                245,
                iPowerupPositionMap[iPowerup],
            ), g);
            for v in 0..=10i16 {
                g.ctl(slider).add("", v);
            }
            g.ctl(slider).allow_wrap(false);
            g.ctl(slider).set_output_ptr(&mut g.game_values.powerupweights[pos] as *mut i16);
            g.ctl(slider).set_current_value(g.game_values.powerupweights[pos]);
            g.ctl(slider).set_item_changed_code(MENU_CODE_POWERUP_SETTING_CHANGED);
            this.miPowerupSlider[iPowerup] = slider;
        }

        this.miRestoreDefaultsButton = Ctl::new(MI_Button::new(g, spr_selectfield, 160, 432, "Defaults", 150, TextAlign::CENTER), g);
        g.ctl(this.miRestoreDefaultsButton).set_code(MENU_CODE_RESTORE_DEFAULT_POWERUP_WEIGHTS);

        this.miClearButton = Ctl::new(MI_Button::new(g, spr_selectfield, 330, 432, "Clear", 150, TextAlign::CENTER), g);
        g.ctl(this.miClearButton).set_code(MENU_CODE_CLEAR_POWERUP_WEIGHTS);

        //Are You Sure dialog box
        this.miDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miDialogAreYouText = Ctl::new(MI_HeaderText::new(g, "Are You", 320, 195), g);
        this.miDialogSureText = Ctl::new(MI_HeaderText::new(g, "Sure?", 320, 220), g);
        this.miDialogYesButton = Ctl::new(MI_Button::new(g, spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        this.miDialogNoButton = Ctl::new(MI_Button::new(g, spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);

        g.ctl(this.miDialogYesButton).set_code(MENU_CODE_POWERUP_RESET_YES);
        g.ctl(this.miDialogNoButton).set_code(MENU_CODE_POWERUP_RESET_NO);

        g.ctl(this.miDialogImage).set_visible(false);
        g.ctl(this.miDialogAreYouText).set_visible(false);
        g.ctl(this.miDialogSureText).set_visible(false);
        g.ctl(this.miDialogYesButton).set_visible(false);
        g.ctl(this.miDialogNoButton).set_visible(false);

        let null = ControlId::NONE;
        let sliders: [ControlId; NP] = std::array::from_fn(|i| this.miPowerupSlider[i].id());

        this.mMenu.add_control(g, this.miOverride.id(), null, this.miPreset.id(), null, null);
        this.mMenu.add_control(g, this.miPreset.id(), this.miOverride.id(), sliders[0], null, null);

        this.miUpArrow = Ctl::new(MI_Image::new(g.rm.menu_verticalarrows, 310, 128, 20, 0, 20, 20, 1, 4, 8), g);
        this.miDownArrow = Ctl::new(MI_Image::new(g.rm.menu_verticalarrows, 310, 406, 0, 0, 20, 20, 1, 4, 8), g);
        g.ctl(this.miUpArrow).set_visible(false);

        let mut iPowerup = 0usize;
        while iPowerup < NP {
            let upcontrol = if iPowerup == 0 { this.miPreset.id() } else { sliders[iPowerup - 2] };

            let downcontrol = if iPowerup >= NP - 2 { this.miRestoreDefaultsButton.id() } else { sliders[iPowerup + 2] };

            this.mMenu.add_control(g, sliders[iPowerup], upcontrol, downcontrol, null, sliders[iPowerup + 1]);

            iPowerup += 1;
            if iPowerup < NP {
                let upcontrol = if iPowerup == 1 { this.miPreset.id() } else { sliders[iPowerup - 2] };

                let downcontrol = if iPowerup >= NP - 2 { this.miClearButton.id() } else { sliders[iPowerup + 2] };

                this.mMenu.add_control(g, sliders[iPowerup], upcontrol, downcontrol, sliders[iPowerup - 1], null);
            }
            iPowerup += 1;
        }

        this.mMenu.add_control(g, this.miRestoreDefaultsButton.id(), sliders[NP - 2], null, null, this.miClearButton.id());
        this.mMenu.add_control(g, this.miClearButton.id(), sliders[NP - 1], null, this.miRestoreDefaultsButton.id(), null);

        //Setup positions and visible powerups
        this.setup_powerup_fields(g);


        this.mMenu.add_non_control(this.miDialogImage.id());
        this.mMenu.add_non_control(this.miDialogAreYouText.id());
        this.mMenu.add_non_control(this.miDialogSureText.id());

        this.mMenu.add_control(g, this.miDialogYesButton.id(), null, null, null, this.miDialogNoButton.id());
        this.mMenu.add_control(g, this.miDialogNoButton.id(), null, null, this.miDialogYesButton.id(), null);

        this.mMenu.add_non_control(this.miUpArrow.id());
        this.mMenu.add_non_control(this.miDownArrow.id());

        this.mMenu.set_initial_focus(g, this.miOverride.id());
        this.mMenu.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

        this
    }

    /// Shows and positions the slider rows in view and hides the rest.
    fn setup_powerup_fields(&mut self, g: &mut Game) {
        for iPowerup in 0..NP {
            let iPosition = iPowerupPositionMap[iPowerup];
            let mut slider = self.miPowerupSlider[iPosition as usize];

            if (iPosition >> 1) < self.iOffset || (iPosition >> 1) as i32 >= self.iOffset as i32 + self.iNumLines as i32 {
                g.ctl(slider).set_visible(false);
            } else {
                g.ctl(slider).set_visible(true);
                g.ctl(slider).set_position(
                    (self.m_pos.x as i32 + (iPosition % 2) as i32 * 295) as i16,
                    (self.m_pos.y as i32 + 84 + 38 * (iPosition as i32 / 2 - self.iOffset as i32)) as i16,
                );
            }
        }
    }

    /// Enables or disables every slider and the preset field's link down to them.
    fn enable_powerup_fields(&mut self, g: &mut Game, fEnable: bool) {
        for iPowerup in 0..NP {
            g.ctl(self.miPowerupSlider[iPowerup]).disable(!fEnable);
        }

        let down = if fEnable { self.miPowerupSlider[0].id() } else { Default::default() };
        g.ctl(self.miPreset).set_neighbor(MenuNavDirection::Down, down);
    }

    /// Moves the focused row down, scrolling the sliders between the stops.
    pub fn move_next(&mut self, g: &mut Game) {
        self.iIndex += 1;

        if self.iIndex > self.iTopStop && self.iIndex <= self.iBottomStop {
            self.iOffset += 1;
            self.setup_powerup_fields(g);
        }

        self.adjust_display_arrows(g);
    }

    /// Moves the focused row up, scrolling the sliders between the stops.
    pub fn move_prev(&mut self, g: &mut Game) {
        self.iIndex -= 1;

        if self.iIndex >= self.iTopStop && self.iIndex < self.iBottomStop {
            self.iOffset -= 1;
            self.setup_powerup_fields(g);
        }

        self.adjust_display_arrows(g);
    }

    /// Shows each scroll arrow while rows are hidden that way.
    fn adjust_display_arrows(&mut self, g: &mut Game) {
        if self.iIndex > self.iTopStop {
            g.ctl(self.miUpArrow).set_visible(true);
        } else {
            g.ctl(self.miUpArrow).set_visible(false);
        }

        if self.iIndex < self.iBottomStop {
            g.ctl(self.miDownArrow).set_visible(true);
        } else {
            g.ctl(self.miDownArrow).set_visible(false);
        }
    }
}

impl UI_ControlTrait for MI_PowerupSelection {
    crate::impl_ctl!();

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.mMenu.reset_menu(g);
        self.iOffset = 0;
        self.iIndex = 0;
        self.setup_powerup_fields(g);


        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let mut gp = g.ptr();
        let ret = self.mMenu.send_input(g, playerInput);

        if MENU_CODE_CANCEL_INPUT == ret {
            self.fModifying = false;
            return MENU_CODE_UNSELECT_ITEM;
        } else if MENU_CODE_POWERUP_PRESET_CHANGED == ret {
            for iPowerup in 0..NP {
                let pos = iPowerupPositionMap[iPowerup] as usize;
                let iCurrentValue = g.game_values.allPowerupPresets[g.game_values.poweruppreset as usize][pos];
                g.ctl(self.miPowerupSlider[iPowerup]).set_current_value(iCurrentValue);
                g.game_values.powerupweights[pos] = iCurrentValue;
            }

        } else if MENU_CODE_POWERUP_SETTING_CHANGED == ret {
            //Save the weights into the selected preset
            for iPowerup in 0..NP {
                let pos = iPowerupPositionMap[iPowerup] as usize;
                gp.get().game_values.allPowerupPresets[gp.get().game_values.poweruppreset as usize][pos] = gp.get().game_values.powerupweights[pos];
            }
        } else if MENU_CODE_RESTORE_DEFAULT_POWERUP_WEIGHTS == ret || MENU_CODE_CLEAR_POWERUP_WEIGHTS == ret {
            g.ctl(self.miDialogImage).set_visible(true);
            g.ctl(self.miDialogAreYouText).set_visible(true);
            g.ctl(self.miDialogSureText).set_visible(true);
            g.ctl(self.miDialogYesButton).set_visible(true);
            g.ctl(self.miDialogNoButton).set_visible(true);

            self.mMenu.remember_current();

            self.mMenu.set_initial_focus(g, self.miDialogNoButton.id());
            self.mMenu.set_cancel_code(MENU_CODE_POWERUP_RESET_NO);
            self.mMenu.reset_menu(g);

            if MENU_CODE_CLEAR_POWERUP_WEIGHTS == ret {
                g.ctl(self.miDialogYesButton).set_code(MENU_CODE_POWERUP_CLEAR_YES);
            } else {
                g.ctl(self.miDialogYesButton).set_code(MENU_CODE_POWERUP_RESET_YES);
            }
        } else if MENU_CODE_POWERUP_RESET_YES == ret || MENU_CODE_POWERUP_RESET_NO == ret || MENU_CODE_POWERUP_CLEAR_YES == ret {
            g.ctl(self.miDialogImage).set_visible(false);
            g.ctl(self.miDialogAreYouText).set_visible(false);
            g.ctl(self.miDialogSureText).set_visible(false);
            g.ctl(self.miDialogYesButton).set_visible(false);
            g.ctl(self.miDialogNoButton).set_visible(false);

            self.mMenu.set_initial_focus(g, self.miOverride.id());
            self.mMenu.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            self.mMenu.restore_current(g);

            if MENU_CODE_POWERUP_RESET_YES == ret {
                //restore default powerup weights for powerup selection menu
                for iPowerup in 0..NP {
                    let pos = iPowerupPositionMap[iPowerup] as usize;
                    let iDefaultValue = default_powerup_setting(g.game_values.poweruppreset as usize, pos);
                    g.ctl(self.miPowerupSlider[iPowerup]).set_current_value(iDefaultValue);
                    g.game_values.powerupweights[pos] = iDefaultValue;

                    gp.get().game_values.allPowerupPresets[gp.get().game_values.poweruppreset as usize][pos] = iDefaultValue;
                }
            } else if MENU_CODE_POWERUP_CLEAR_YES == ret {
                //Zero every weight in the selected preset
                for iPowerup in 0..NP {
                    g.ctl(self.miPowerupSlider[iPowerup]).set_current_value(0);
                    g.game_values.powerupweights[iPowerup] = 0;

                    gp.get().game_values.allPowerupPresets[gp.get().game_values.poweruppreset as usize][iPowerup] = 0;
                }
            }
        } else if MENU_CODE_NEIGHBOR_UP == ret {
            self.move_prev(g);
        } else if MENU_CODE_NEIGHBOR_DOWN == ret {
            self.move_next(g);
        }

        ret
    }

    fn update(&mut self, g: &mut Game) {
        self.mMenu.update(g);
    }

    fn draw(&mut self, g: &mut Game) {
        if !self.m_visible {
            return;
        }

        self.mMenu.draw(g);
    }
}
