//! Port of src/smw/ui/MI_FrenzyModeOptions.cpp

use crate::common::global_constants::NUMFRENZYCARDS;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::uicontrol::{ctl_ptr, ControlPtr, TextAlign, UI_Control, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_powerup_slider::MI_PowerupSlider;

const NF: usize = NUMFRENZYCARDS as usize;

//Rearrange display of powerups
pub static mut iFrenzyCardPositionMap: [i16; NF] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];

pub struct MI_FrenzyModeOptions {
    pub ui_control: UI_Control,

    iIndex: i16,
    iOffset: i16,
    iTopStop: i16,
    iBottomStop: i16,
    iNumLines: i16,
    iWidth: i16,

    mMenu: Box<UI_Menu>,

    miQuantityField: Ptr<MI_SelectField<i16>>,
    miRateField: Ptr<MI_SelectField<i16>>,
    miStoredShellsField: Ptr<MI_SelectField<bool>>,
    miPowerupSlider: [Ptr<MI_PowerupSlider>; NF],
    miBackButton: Ptr<MI_Button>,

    miUpArrow: Ptr<MI_Image>,
    miDownArrow: Ptr<MI_Image>,
}
crate::impl_base!(MI_FrenzyModeOptions => ui_control: UI_Control);

impl MI_FrenzyModeOptions {
    pub fn new(x: i16, y: i16, width: i16, numlines: i16) -> Self {
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
            miQuantityField: Ptr::null(),
            miRateField: Ptr::null(),
            miStoredShellsField: Ptr::null(),
            miPowerupSlider: [Ptr::null(); NF],
            miBackButton: Ptr::null(),
            miUpArrow: Ptr::null(),
            miDownArrow: Ptr::null(),
        };

        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);

            this.miQuantityField = Ptr::new_box(MI_SelectField::<i16>::new(spr_selectfield, 120, 40, "Limit", 400, 180));
            this.miQuantityField.add("Single Powerup", 0);
            this.miQuantityField.add("1 Powerup", 1);
            this.miQuantityField.add("2 Powerups", 2);
            this.miQuantityField.add("3 Powerups", 3);
            this.miQuantityField.add("4 Powerups", 4);
            this.miQuantityField.add("5 Powerups", 5);
            this.miQuantityField.add("# Players - 1", 6);
            this.miQuantityField.add("# Players", 7);
            this.miQuantityField.add_random("# Players + 1", 8, false);
            this.miQuantityField.add_random("# Players + 2", 9, false);
            this.miQuantityField.add_random("# Players + 3", 10, false);
            this.miQuantityField.set_output_ptr(&mut game_values.gamemodemenusettings.frenzy.quantity as *mut i16);
            this.miQuantityField.set_current_value(game_values.gamemodemenusettings.frenzy.quantity);

            this.miRateField = Ptr::new_box(MI_SelectField::<i16>::new(spr_selectfield, 120, 80, "Rate", 400, 180));
            this.miRateField.add("Instant", 0);
            this.miRateField.add("1 Second", 62);
            this.miRateField.add("2 Seconds", 124);
            this.miRateField.add("3 Seconds", 186);
            this.miRateField.add("5 Seconds", 310);
            this.miRateField.add_random("10 Seconds", 620, false);
            this.miRateField.add_random("15 Seconds", 930, false);
            this.miRateField.add_random("20 Seconds", 1240, false);
            this.miRateField.add_random("25 Seconds", 1550, false);
            this.miRateField.add_random("30 Seconds", 1860, false);
            this.miRateField.set_output_ptr(&mut game_values.gamemodemenusettings.frenzy.rate as *mut i16);
            this.miRateField.set_current_value(game_values.gamemodemenusettings.frenzy.rate);

            this.miStoredShellsField = Ptr::new_box(MI_SelectField::<bool>::new(spr_selectfield, 120, 120, "Store Shells", 400, 180));
            this.miStoredShellsField.add("Off", false);
            this.miStoredShellsField.add("On", true);
            this.miStoredShellsField.set_output_ptr(&mut game_values.gamemodemenusettings.frenzy.storedshells as *mut bool);
            this.miStoredShellsField.set_current_value(game_values.gamemodemenusettings.frenzy.storedshells);
            this.miStoredShellsField.set_auto_advance(true);

            let iPowerupMap: [i16; NF] = [8, 5, 11, 17, 19, 21, 23, 24, 25, 20, 9, 16, 10, 22, 12, 13, 14, 15, 27];
            for iPowerup in 0..NF {
                let mut slider = Ptr::new_box(MI_PowerupSlider::new(
                    spr_selectfield,
                    Ptr::from_mut(&mut rm.menu_slider_bar),
                    Ptr::from_mut(&mut rm.spr_storedpoweruplarge),
                    if iPowerup < 10 { 65 } else { 330 },
                    0,
                    245,
                    iPowerupMap[iPowerup],
                ));
                for v in 0..=10i16 {
                    slider.add("", v);
                }
                slider.allow_wrap(false);
                slider.set_output_ptr(&mut game_values.gamemodemenusettings.frenzy.powerupweight[iPowerup] as *mut i16);
                slider.set_current_value(game_values.gamemodemenusettings.frenzy.powerupweight[iPowerup]);
                this.miPowerupSlider[iPowerup] = slider;
            }

            this.miBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miBackButton.set_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

            this.miUpArrow = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_verticalarrows), 310, 162, 20, 0, 20, 20, 1, 4, 8));
            this.miDownArrow = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_verticalarrows), 310, 402, 0, 0, 20, 20, 1, 4, 8));
            this.miUpArrow.set_visible(false);

            let null: ControlPtr = Ptr::null();
            let sliders: [ControlPtr; NF] = std::array::from_fn(|i| ctl_ptr(this.miPowerupSlider[i]));
            let back = ctl_ptr(this.miBackButton);

            this.mMenu.add_control(ctl_ptr(this.miQuantityField), null, ctl_ptr(this.miRateField), null, null);
            this.mMenu.add_control(ctl_ptr(this.miRateField), ctl_ptr(this.miQuantityField), ctl_ptr(this.miStoredShellsField), null, null);
            this.mMenu.add_control(ctl_ptr(this.miStoredShellsField), ctl_ptr(this.miRateField), sliders[0], null, null);

            let mut iPowerup = 0usize;
            while iPowerup < NF {
                let upcontrol = if iPowerup == 0 { ctl_ptr(this.miStoredShellsField) } else { sliders[iPowerup - 2] };

                let lDownControl1 = if iPowerup >= NF - 2 { back } else { sliders[iPowerup + 2] };

                let rightcontrol = if iPowerup + 1 < NF { sliders[iPowerup + 1] } else { back };

                this.mMenu.add_control(sliders[iPowerup], upcontrol, lDownControl1, null, rightcontrol);

                iPowerup += 1;
                if iPowerup < NF {
                    let upcontrol = if iPowerup == 1 { ctl_ptr(this.miStoredShellsField) } else { sliders[iPowerup - 2] };

                    let lDownControl2 = if iPowerup >= NF - 2 { back } else { sliders[iPowerup + 2] };

                    this.mMenu.add_control(sliders[iPowerup], upcontrol, lDownControl2, sliders[iPowerup - 1], null);
                }
                iPowerup += 1;
            }

            //Setup positions and visible powerups
            this.setup_powerup_fields();

            this.mMenu.add_non_control(ctl_ptr(this.miUpArrow));
            this.mMenu.add_non_control(ctl_ptr(this.miDownArrow));

            this.mMenu.add_control(back, sliders[NF - 1], null, sliders[NF - 1], null);

            this.mMenu.set_initial_focus(ctl_ptr(this.miQuantityField));
            this.mMenu.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);
        }

        this
    }

    fn setup_powerup_fields(&mut self) {
        for iPowerup in 0..NF {
            let iPosition = unsafe { iFrenzyCardPositionMap[iPowerup] };
            let mut slider = self.miPowerupSlider[iPosition as usize];

            if (iPosition >> 1) < self.iOffset || (iPosition >> 1) as i32 >= self.iOffset as i32 + self.iNumLines as i32 {
                slider.set_visible(false);
            } else {
                slider.set_visible(true);
                slider.set_position(
                    (self.m_pos.x as i32 + (iPosition % 2) as i32 * 295) as i16,
                    (self.m_pos.y as i32 + 118 + 38 * (iPosition as i32 / 2 - self.iOffset as i32)) as i16,
                );
            }
        }
    }

    pub fn move_next(&mut self) {
        self.iIndex += 1;

        if self.iIndex > self.iTopStop && self.iIndex <= self.iBottomStop {
            self.iOffset += 1;
            self.setup_powerup_fields();
        }

        self.adjust_display_arrows();
    }

    pub fn move_prev(&mut self) {
        self.iIndex -= 1;

        if self.iIndex >= self.iTopStop && self.iIndex < self.iBottomStop {
            self.iOffset -= 1;
            self.setup_powerup_fields();
        }

        self.adjust_display_arrows();
    }

    pub fn set_random_game_mode_settings(&mut self) {
        unsafe {
            game_values.gamemodesettings.frenzy.quantity = self.miQuantityField.random_value();
            game_values.gamemodesettings.frenzy.rate = self.miRateField.random_value();
            game_values.gamemodesettings.frenzy.storedshells = self.miStoredShellsField.random_value();

            for iPowerup in 0..NF {
                game_values.gamemodesettings.frenzy.powerupweight[iPowerup] = self.miPowerupSlider[iPowerup].random_value();
            }
        }
    }

    fn adjust_display_arrows(&mut self) {
        if self.iIndex > self.iTopStop {
            self.miUpArrow.set_visible(true);
        } else {
            self.miUpArrow.set_visible(false);
        }

        if self.iIndex < self.iBottomStop {
            self.miDownArrow.set_visible(true);
        } else {
            self.miDownArrow.set_visible(false);
        }
    }
}

impl UI_ControlTrait for MI_FrenzyModeOptions {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.mMenu.reset_menu();
        self.iOffset = 0;
        self.iIndex = 0;
        self.setup_powerup_fields();
        self.adjust_display_arrows();

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let prevControl = self.mMenu.current_focus();

        let ret = self.mMenu.send_input(playerInput);

        let nextControl = self.mMenu.current_focus();

        if MENU_CODE_CANCEL_INPUT == ret {
            self.fModifying = false;
            return MENU_CODE_UNSELECT_ITEM;
        } else if MENU_CODE_NEIGHBOR_UP == ret {
            if prevControl != self.miBackButton {
                self.move_prev();
            }
        } else if MENU_CODE_NEIGHBOR_DOWN == ret {
            if nextControl != self.miBackButton || prevControl == self.miPowerupSlider[NF - 2] {
                self.move_next();
            }
        }

        ret
    }

    fn update(&mut self) {
        self.mMenu.update();
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        self.mMenu.draw();
    }

    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_NONE;
        }

        //Loop through all controls to see if one was clicked on
        let ret = self.mMenu.mouse_click(iMouseX, iMouseY);

        if ret == MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS {
            return ret;
        }

        for iPowerup in 0..NF {
            let slider = self.miPowerupSlider[iPowerup];

            if slider == self.mMenu.current_focus() {
                self.iIndex = ((iPowerup as i16) >> 1) + 3;

                if self.iIndex <= self.iTopStop {
                    self.iOffset = 0;
                } else if self.iIndex >= self.iBottomStop {
                    self.iOffset = self.iBottomStop - self.iTopStop;
                } else {
                    self.iOffset = self.iIndex - self.iTopStop;
                }

                self.setup_powerup_fields();
                self.adjust_display_arrows();

                return ret;
            }
        }

        self.iOffset = 0;
        self.iIndex = 0;
        self.setup_powerup_fields();
        self.adjust_display_arrows();

        ret
    }

    fn refresh(&mut self) {
        self.mMenu.refresh();
    }
}
