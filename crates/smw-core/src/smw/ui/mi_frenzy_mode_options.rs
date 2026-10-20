//! Frenzy mode settings: card limit, spawn rate, stored shells and a weight slider per card.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::global_constants::NUMFRENZYCARDS;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_powerup_slider::MI_PowerupSlider;

const NF: usize = NUMFRENZYCARDS as usize;

/// Screen slot of each card's slider, two per row; currently in card order.
pub static iFrenzyCardPositionMap: [i16; NF] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];

/// A scrolling inner menu writing `gamemodemenusettings.frenzy`.
pub struct MI_FrenzyModeOptions {
    pub ui_control: UI_Control,

    /// Focused row, counting the three fields above the sliders; drives scrolling.
    iIndex: i16,
    /// First slider row shown.
    iOffset: i16,
    iTopStop: i16,
    iBottomStop: i16,
    /// Slider rows shown at once.
    iNumLines: i16,
    iWidth: i16,

    mMenu: Box<UI_Menu>,

    /// Card limit: 0 one at a time, 1 to 5 fixed, 6 to 10 the player count -1 to +3.
    miQuantityField: Ctl<MI_SelectField<i16>>,
    /// Spawn interval in frames, 60 per labeled second.
    miRateField: Ctl<MI_SelectField<i16>>,
    miStoredShellsField: Ctl<MI_SelectField<bool>>,
    /// Weight of each frenzy card, 0 to 10.
    miPowerupSlider: [Ctl<MI_PowerupSlider>; NF],
    miBackButton: Ctl<MI_Button>,

    miUpArrow: Ctl<MI_Image>,
    miDownArrow: Ctl<MI_Image>,
}
crate::impl_base!(MI_FrenzyModeOptions => ui_control: UI_Control);

impl MI_FrenzyModeOptions {
    pub fn new(g: &mut Game, x: i16, y: i16, width: i16, numlines: i16) -> Self {
        let iNumLines = numlines;
        let iTopStop = (((iNumLines as i32 - 1) >> 1) + 3) as i16; // Plus 3 for the 3 fields at the top
        let iBottomStop = (((NUMFRENZYCARDS + 1) >> 1) - iNumLines as i32 + iTopStop as i32) as i16;

        let mut this = MI_FrenzyModeOptions {
            ui_control: UI_Control::new(x, y),
            iIndex: 0,
            iOffset: 0,
            iTopStop,
            iBottomStop,
            iNumLines,
            iWidth: width,
            mMenu: Box::new(UI_Menu::new()),
            miQuantityField: Default::default(),
            miRateField: Default::default(),
            miStoredShellsField: Default::default(),
            miPowerupSlider: [Default::default(); NF],
            miBackButton: Default::default(),
            miUpArrow: Default::default(),
            miDownArrow: Default::default(),
        };

        let spr_selectfield = g.rm.spr_selectfield;

        this.miQuantityField = Ctl::new(MI_SelectField::<i16>::new(spr_selectfield, 120, 40, "Limit", 400, 180), g);
        g.ctl(this.miQuantityField).add("Single Powerup", 0);
        g.ctl(this.miQuantityField).add("1 Powerup", 1);
        g.ctl(this.miQuantityField).add("2 Powerups", 2);
        g.ctl(this.miQuantityField).add("3 Powerups", 3);
        g.ctl(this.miQuantityField).add("4 Powerups", 4);
        g.ctl(this.miQuantityField).add("5 Powerups", 5);
        g.ctl(this.miQuantityField).add("# Players - 1", 6);
        g.ctl(this.miQuantityField).add("# Players", 7);
        g.ctl(this.miQuantityField).add_random("# Players + 1", 8, false);
        g.ctl(this.miQuantityField).add_random("# Players + 2", 9, false);
        g.ctl(this.miQuantityField).add_random("# Players + 3", 10, false);
        g.ctl(this.miQuantityField).set_output_ptr(&mut g.game_values.gamemodemenusettings.frenzy.quantity as *mut i16);
        g.ctl(this.miQuantityField).set_current_value(g.game_values.gamemodemenusettings.frenzy.quantity);

        this.miRateField = Ctl::new(MI_SelectField::<i16>::new(spr_selectfield, 120, 80, "Rate", 400, 180), g);
        g.ctl(this.miRateField).add("Instant", 0);
        g.ctl(this.miRateField).add("1 Second", 60);
        g.ctl(this.miRateField).add("2 Seconds", 120);
        g.ctl(this.miRateField).add("3 Seconds", 180);
        g.ctl(this.miRateField).add("5 Seconds", 300);
        g.ctl(this.miRateField).add_random("10 Seconds", 600, false);
        g.ctl(this.miRateField).add_random("15 Seconds", 900, false);
        g.ctl(this.miRateField).add_random("20 Seconds", 1200, false);
        g.ctl(this.miRateField).add_random("25 Seconds", 1500, false);
        g.ctl(this.miRateField).add_random("30 Seconds", 1800, false);
        g.ctl(this.miRateField).set_output_ptr(&mut g.game_values.gamemodemenusettings.frenzy.rate as *mut i16);
        g.ctl(this.miRateField).set_current_value(g.game_values.gamemodemenusettings.frenzy.rate);

        this.miStoredShellsField = Ctl::new(MI_SelectField::<bool>::new(spr_selectfield, 120, 120, "Store Shells", 400, 180), g);
        g.ctl(this.miStoredShellsField).add("Off", false);
        g.ctl(this.miStoredShellsField).add("On", true);
        g.ctl(this.miStoredShellsField).set_output_ptr(&mut g.game_values.gamemodemenusettings.frenzy.storedshells as *mut bool);
        g.ctl(this.miStoredShellsField).set_current_value(g.game_values.gamemodemenusettings.frenzy.storedshells);
        g.ctl(this.miStoredShellsField).set_auto_advance(true);

        // Stored powerup icon for each card.
        let iPowerupMap: [i16; NF] = [8, 5, 11, 17, 19, 21, 23, 24, 25, 20, 9, 16, 10, 22, 12, 13, 14, 15, 27];
        for iPowerup in 0..NF {
            let mut slider = Ctl::new(MI_PowerupSlider::new(
                spr_selectfield,
                g.rm.menu_slider_bar,
                g.rm.spr_storedpoweruplarge,
                if iPowerup < 10 { 65 } else { 330 },
                0,
                245,
                iPowerupMap[iPowerup],
            ), g);
            for v in 0..=10i16 {
                g.ctl(slider).add("", v);
            }
            g.ctl(slider).allow_wrap(false);
            g.ctl(slider).set_output_ptr(&mut g.game_values.gamemodemenusettings.frenzy.powerupweight[iPowerup] as *mut i16);
            g.ctl(slider).set_current_value(g.game_values.gamemodemenusettings.frenzy.powerupweight[iPowerup]);
            this.miPowerupSlider[iPowerup] = slider;
        }

        this.miBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.miBackButton).set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this.miUpArrow = Ctl::new(MI_Image::new(g.rm.menu_verticalarrows, 310, 162, 20, 0, 20, 20, 1, 4, 8), g);
        this.miDownArrow = Ctl::new(MI_Image::new(g.rm.menu_verticalarrows, 310, 402, 0, 0, 20, 20, 1, 4, 8), g);
        g.ctl(this.miUpArrow).set_visible(false);

        let null = ControlId::NONE;
        let sliders: [ControlId; NF] = std::array::from_fn(|i| this.miPowerupSlider[i].id());
        let back = this.miBackButton.id();

        this.mMenu.add_control(g, this.miQuantityField.id(), null, this.miRateField.id(), null, null);
        this.mMenu.add_control(g, this.miRateField.id(), this.miQuantityField.id(), this.miStoredShellsField.id(), null, null);
        this.mMenu.add_control(g, this.miStoredShellsField.id(), this.miRateField.id(), sliders[0], null, null);

        let mut iPowerup = 0usize;
        while iPowerup < NF {
            let upcontrol = if iPowerup == 0 { this.miStoredShellsField.id() } else { sliders[iPowerup - 2] };

            let lDownControl1 = if iPowerup >= NF - 2 { back } else { sliders[iPowerup + 2] };

            let rightcontrol = if iPowerup + 1 < NF { sliders[iPowerup + 1] } else { back };

            this.mMenu.add_control(g, sliders[iPowerup], upcontrol, lDownControl1, null, rightcontrol);

            iPowerup += 1;
            if iPowerup < NF {
                let upcontrol = if iPowerup == 1 { this.miStoredShellsField.id() } else { sliders[iPowerup - 2] };

                let lDownControl2 = if iPowerup >= NF - 2 { back } else { sliders[iPowerup + 2] };

                this.mMenu.add_control(g, sliders[iPowerup], upcontrol, lDownControl2, sliders[iPowerup - 1], null);
            }
            iPowerup += 1;
        }

        //Setup positions and visible powerups
        this.setup_powerup_fields(g);

        this.mMenu.add_non_control(this.miUpArrow.id());
        this.mMenu.add_non_control(this.miDownArrow.id());

        this.mMenu.add_control(g, back, sliders[NF - 1], null, sliders[NF - 1], null);

        this.mMenu.set_initial_focus(g, this.miQuantityField.id());
        this.mMenu.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        this
    }

    /// Shows and positions the slider rows in view and hides the rest.
    fn setup_powerup_fields(&mut self, g: &mut Game) {
        for iPowerup in 0..NF {
            let iPosition = iFrenzyCardPositionMap[iPowerup];
            let mut slider = self.miPowerupSlider[iPosition as usize];

            if (iPosition >> 1) < self.iOffset || (iPosition >> 1) as i32 >= self.iOffset as i32 + self.iNumLines as i32 {
                g.ctl(slider).set_visible(false);
            } else {
                g.ctl(slider).set_visible(true);
                g.ctl(slider).set_position(
                    (self.m_pos.x as i32 + (iPosition % 2) as i32 * 295) as i16,
                    (self.m_pos.y as i32 + 118 + 38 * (iPosition as i32 / 2 - self.iOffset as i32)) as i16,
                );
            }
        }
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

    /// Rolls every field's random value into `gamemodesettings.frenzy`.
    pub fn set_random_game_mode_settings(&mut self, g: &mut Game) {
        g.game_values.gamemodesettings.frenzy.quantity = g.ctl(self.miQuantityField).random_value(g);
        g.game_values.gamemodesettings.frenzy.rate = g.ctl(self.miRateField).random_value(g);
        g.game_values.gamemodesettings.frenzy.storedshells = g.ctl(self.miStoredShellsField).random_value(g);

        for iPowerup in 0..NF {
            g.game_values.gamemodesettings.frenzy.powerupweight[iPowerup] = g.ctl(self.miPowerupSlider[iPowerup]).random_value(g);
        }
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

impl UI_ControlTrait for MI_FrenzyModeOptions {
    crate::impl_ctl!();

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.mMenu.reset_menu(g);
        self.iOffset = 0;
        self.iIndex = 0;
        self.setup_powerup_fields(g);
        self.adjust_display_arrows(g);

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let prevControl = self.mMenu.current_focus();

        let ret = self.mMenu.send_input(g, playerInput);

        let nextControl = self.mMenu.current_focus();

        if MENU_CODE_CANCEL_INPUT == ret {
            self.fModifying = false;
            return MENU_CODE_UNSELECT_ITEM;
        } else if MENU_CODE_NEIGHBOR_UP == ret {
            if prevControl != self.miBackButton.id() {
                self.move_prev(g);
            }
        } else if MENU_CODE_NEIGHBOR_DOWN == ret {
            if nextControl != self.miBackButton.id() || prevControl == self.miPowerupSlider[NF - 2].id() {
                self.move_next(g);
            }
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

    fn mouse_click(&mut self, g: &mut Game, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_NONE;
        }

        //Loop through all controls to see if one was clicked on
        let ret = self.mMenu.mouse_click(g, iMouseX, iMouseY);

        if ret == MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS {
            return ret;
        }

        for iPowerup in 0..NF {
            let slider = self.miPowerupSlider[iPowerup];

            if slider.id() == self.mMenu.current_focus() {
                self.iIndex = ((iPowerup as i16) >> 1) + 3;

                if self.iIndex <= self.iTopStop {
                    self.iOffset = 0;
                } else if self.iIndex >= self.iBottomStop {
                    self.iOffset = self.iBottomStop - self.iTopStop;
                } else {
                    self.iOffset = self.iIndex - self.iTopStop;
                }

                self.setup_powerup_fields(g);
                self.adjust_display_arrows(g);

                return ret;
            }
        }

        self.iOffset = 0;
        self.iIndex = 0;
        self.setup_powerup_fields(g);
        self.adjust_display_arrows(g);

        ret
    }

    fn refresh(&mut self, g: &mut Game) {
        self.mMenu.refresh(g);
    }
}
