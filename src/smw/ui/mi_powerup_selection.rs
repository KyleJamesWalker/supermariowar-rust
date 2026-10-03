//! Port of src/smw/ui/MI_PowerupSelection.cpp

use crate::common::game_values::default_powerup_setting;
use crate::common::global_constants::NUM_POWERUPS;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, ControlPtr, TextAlign, UI_Control, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_powerup_slider::MI_PowerupSlider;

//Rearrange display of powerups
//short iPowerupDisplayMap[NUM_POWERUPS] = { 4, 0, 1, 2, 3, 6, 10, 12, 11, 14, 13, 7, 16, 17, 18, 19, 15, 9, 5, 8, 20, 21, 22, 23, 24, 25};
//                                          0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25
const iPowerupPositionMap: [i16; NUM_POWERUPS as usize] = [1, 0, 2, 6, 3, 8, 4, 20, 18, 7, 5, 10, 11, 22, 19, 9, 23, 16, 21, 12, 17, 13, 24, 15, 25, 14];

const NP: usize = NUM_POWERUPS as usize;

pub struct MI_PowerupSelection {
    pub ui_control: UI_Control,

    iIndex: i16,
    iOffset: i16,
    iNumLines: i16,
    iTopStop: i16,
    iBottomStop: i16,

    mMenu: Box<UI_Menu>,

    miOverride: Ptr<MI_SelectField<i16>>,
    miPreset: Ptr<MI_SelectField<i16>>,

    miPowerupSlider: [Ptr<MI_PowerupSlider>; NP],

    miRestoreDefaultsButton: Ptr<MI_Button>,
    miClearButton: Ptr<MI_Button>,

    miDialogImage: Ptr<MI_Image>,
    miDialogAreYouText: Ptr<MI_Text>,
    miDialogSureText: Ptr<MI_Text>,
    miDialogYesButton: Ptr<MI_Button>,
    miDialogNoButton: Ptr<MI_Button>,

    miUpArrow: Ptr<MI_Image>,
    miDownArrow: Ptr<MI_Image>,
}
crate::impl_base!(MI_PowerupSelection => ui_control: UI_Control);

impl MI_PowerupSelection {
    pub fn new(x: i16, y: i16, width: i16, numlines: i16) -> Self {
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
            miOverride: Ptr::null(),
            miPreset: Ptr::null(),
            miPowerupSlider: [Ptr::null(); NP],
            miRestoreDefaultsButton: Ptr::null(),
            miClearButton: Ptr::null(),
            miDialogImage: Ptr::null(),
            miDialogAreYouText: Ptr::null(),
            miDialogSureText: Ptr::null(),
            miDialogYesButton: Ptr::null(),
            miDialogNoButton: Ptr::null(),
            miUpArrow: Ptr::null(),
            miDownArrow: Ptr::null(),
        };

        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);

            this.miOverride = Ptr::new_box(MI_SelectField::<i16>::new(spr_selectfield, 70, this.m_pos.y, "Use Settings From", 500, 250));
            this.miOverride.add("Map Only", 0);
            this.miOverride.add("Game Only", 1);
            this.miOverride.add("Basic Average", 2);
            this.miOverride.add("Weighted Average", 3);
            this.miOverride.set_output_ptr(&mut game_values.overridepowerupsettings as *mut i16);
            this.miOverride.set_current_value(game_values.overridepowerupsettings);
            //miOverride->SetItemChangedCode(MENU_CODE_POWERUP_OVERRIDE_CHANGED);

            this.miPreset = Ptr::new_box(MI_SelectField::<i16>::new(spr_selectfield, 70, this.m_pos.y + 40, "Item Set", 500, 250));
            this.miPreset.add("Custom Set 1", 0);
            this.miPreset.add("Custom Set 2", 1);
            this.miPreset.add("Custom Set 3", 2);
            this.miPreset.add("Custom Set 4", 3);
            this.miPreset.add("Custom Set 5", 4);
            this.miPreset.add("Balanced Set", 5);
            this.miPreset.add("Weapons Only", 6);
            this.miPreset.add("Koopa Bros Weapons", 7);
            this.miPreset.add("Support Items", 8);
            this.miPreset.add("Booms and Shakes", 9);
            this.miPreset.add("Fly and Glide", 10);
            this.miPreset.add("Shells Only", 11);
            this.miPreset.add("Mushrooms Only", 12);
            this.miPreset.add("Super Mario Bros 1", 13);
            this.miPreset.add("Super Mario Bros 2", 14);
            this.miPreset.add("Super Mario Bros 3", 15);
            this.miPreset.add("Super Mario World", 16);
            this.miPreset.set_output_ptr(&mut game_values.poweruppreset as *mut i16);
            this.miPreset.set_current_value(game_values.poweruppreset);
            this.miPreset.set_item_changed_code(MENU_CODE_POWERUP_PRESET_CHANGED);

            for iPowerup in 0..NP {
                let pos = iPowerupPositionMap[iPowerup] as usize;
                let mut slider = Ptr::new_box(MI_PowerupSlider::new(
                    spr_selectfield,
                    Ptr::from_mut(&mut rm.menu_slider_bar),
                    Ptr::from_mut(&mut rm.spr_storedpoweruplarge),
                    0,
                    0,
                    245,
                    iPowerupPositionMap[iPowerup],
                ));
                for v in 0..=10i16 {
                    slider.add("", v);
                }
                slider.allow_wrap(false);
                slider.set_output_ptr(&mut game_values.powerupweights[pos] as *mut i16);
                slider.set_current_value(game_values.powerupweights[pos]);
                slider.set_item_changed_code(MENU_CODE_POWERUP_SETTING_CHANGED);
                this.miPowerupSlider[iPowerup] = slider;
            }

            this.miRestoreDefaultsButton = Ptr::new_box(MI_Button::new(spr_selectfield, 160, 432, "Defaults", 150, TextAlign::CENTER));
            this.miRestoreDefaultsButton.set_code(MENU_CODE_RESTORE_DEFAULT_POWERUP_WEIGHTS);

            this.miClearButton = Ptr::new_box(MI_Button::new(spr_selectfield, 330, 432, "Clear", 150, TextAlign::CENTER));
            this.miClearButton.set_code(MENU_CODE_CLEAR_POWERUP_WEIGHTS);

            //Are You Sure dialog box
            this.miDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miDialogAreYouText = Ptr::new_box(MI_HeaderText::new("Are You", 320, 195));
            this.miDialogSureText = Ptr::new_box(MI_HeaderText::new("Sure?", 320, 220));
            this.miDialogYesButton = Ptr::new_box(MI_Button::new(spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER));
            this.miDialogNoButton = Ptr::new_box(MI_Button::new(spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER));

            this.miDialogYesButton.set_code(MENU_CODE_POWERUP_RESET_YES);
            this.miDialogNoButton.set_code(MENU_CODE_POWERUP_RESET_NO);

            this.miDialogImage.set_visible(false);
            this.miDialogAreYouText.set_visible(false);
            this.miDialogSureText.set_visible(false);
            this.miDialogYesButton.set_visible(false);
            this.miDialogNoButton.set_visible(false);

            let null: ControlPtr = Ptr::null();
            let sliders: [ControlPtr; NP] = std::array::from_fn(|i| ctl_ptr(this.miPowerupSlider[i]));

            this.mMenu.add_control(ctl_ptr(this.miOverride), null, ctl_ptr(this.miPreset), null, null);
            this.mMenu.add_control(ctl_ptr(this.miPreset), ctl_ptr(this.miOverride), sliders[0], null, null);

            this.miUpArrow = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_verticalarrows), 310, 128, 20, 0, 20, 20, 1, 4, 8));
            this.miDownArrow = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.menu_verticalarrows), 310, 406, 0, 0, 20, 20, 1, 4, 8));
            this.miUpArrow.set_visible(false);

            let mut iPowerup = 0usize;
            while iPowerup < NP {
                let upcontrol = if iPowerup == 0 { ctl_ptr(this.miPreset) } else { sliders[iPowerup - 2] };

                let downcontrol = if iPowerup >= NP - 2 { ctl_ptr(this.miRestoreDefaultsButton) } else { sliders[iPowerup + 2] };

                this.mMenu.add_control(sliders[iPowerup], upcontrol, downcontrol, null, sliders[iPowerup + 1]);

                iPowerup += 1;
                if iPowerup < NP {
                    let upcontrol = if iPowerup == 1 { ctl_ptr(this.miPreset) } else { sliders[iPowerup - 2] };

                    let downcontrol = if iPowerup >= NP - 2 { ctl_ptr(this.miClearButton) } else { sliders[iPowerup + 2] };

                    this.mMenu.add_control(sliders[iPowerup], upcontrol, downcontrol, sliders[iPowerup - 1], null);
                }
                iPowerup += 1;
            }

            this.mMenu.add_control(ctl_ptr(this.miRestoreDefaultsButton), sliders[NP - 2], null, null, ctl_ptr(this.miClearButton));
            this.mMenu.add_control(ctl_ptr(this.miClearButton), sliders[NP - 1], null, ctl_ptr(this.miRestoreDefaultsButton), null);

            //Setup positions and visible powerups
            this.setup_powerup_fields();

            //Set enabled based on if we are overriding or not
            //EnablePowerupFields();

            this.mMenu.add_non_control(ctl_ptr(this.miDialogImage));
            this.mMenu.add_non_control(ctl_ptr(this.miDialogAreYouText));
            this.mMenu.add_non_control(ctl_ptr(this.miDialogSureText));

            this.mMenu.add_control(ctl_ptr(this.miDialogYesButton), null, null, null, ctl_ptr(this.miDialogNoButton));
            this.mMenu.add_control(ctl_ptr(this.miDialogNoButton), null, null, ctl_ptr(this.miDialogYesButton), null);

            this.mMenu.add_non_control(ctl_ptr(this.miUpArrow));
            this.mMenu.add_non_control(ctl_ptr(this.miDownArrow));

            this.mMenu.set_initial_focus(ctl_ptr(this.miOverride));
            this.mMenu.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        }

        this
    }

    fn setup_powerup_fields(&mut self) {
        for iPowerup in 0..NP {
            let iPosition = iPowerupPositionMap[iPowerup];
            let mut slider = self.miPowerupSlider[iPosition as usize];

            if (iPosition >> 1) < self.iOffset || (iPosition >> 1) as i32 >= self.iOffset as i32 + self.iNumLines as i32 {
                slider.set_visible(false);
            } else {
                slider.set_visible(true);
                slider.set_position(
                    (self.m_pos.x as i32 + (iPosition % 2) as i32 * 295) as i16,
                    (self.m_pos.y as i32 + 84 + 38 * (iPosition as i32 / 2 - self.iOffset as i32)) as i16,
                );
            }
        }
    }

    fn enable_powerup_fields(&mut self, fEnable: bool) {
        for iPowerup in 0..NP {
            self.miPowerupSlider[iPowerup].disable(!fEnable);
        }

        let down = if fEnable { ctl_ptr(self.miPowerupSlider[0]) } else { Ptr::null() };
        self.miPreset.set_neighbor(MenuNavDirection::Down, down);
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

impl UI_ControlTrait for MI_PowerupSelection {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.mMenu.reset_menu();
        self.iOffset = 0;
        self.iIndex = 0;
        self.setup_powerup_fields();

        //EnablePowerupFields(game_values.poweruppreset >= 1 && game_values.poweruppreset <= 3);

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let ret = self.mMenu.send_input(playerInput);

        unsafe {
            if MENU_CODE_CANCEL_INPUT == ret {
                self.fModifying = false;
                return MENU_CODE_UNSELECT_ITEM;
            } else if MENU_CODE_POWERUP_PRESET_CHANGED == ret {
                for iPowerup in 0..NP {
                    let pos = iPowerupPositionMap[iPowerup] as usize;
                    let iCurrentValue = game_values.allPowerupPresets[game_values.poweruppreset as usize][pos];
                    self.miPowerupSlider[iPowerup].set_current_value(iCurrentValue);
                    game_values.powerupweights[pos] = iCurrentValue;
                }

                //If it is a custom preset, then allow modification
                //EnablePowerupFields(game_values.poweruppreset >= 1 && game_values.poweruppreset <= 3);
            } else if MENU_CODE_POWERUP_SETTING_CHANGED == ret {
                //Update the custom presets
                for iPowerup in 0..NP {
                    let pos = iPowerupPositionMap[iPowerup] as usize;
                    game_values.allPowerupPresets[game_values.poweruppreset as usize][pos] = game_values.powerupweights[pos];
                }
            } else if MENU_CODE_RESTORE_DEFAULT_POWERUP_WEIGHTS == ret || MENU_CODE_CLEAR_POWERUP_WEIGHTS == ret {
                self.miDialogImage.set_visible(true);
                self.miDialogAreYouText.set_visible(true);
                self.miDialogSureText.set_visible(true);
                self.miDialogYesButton.set_visible(true);
                self.miDialogNoButton.set_visible(true);

                self.mMenu.remember_current();

                self.mMenu.set_initial_focus(ctl_ptr(self.miDialogNoButton));
                self.mMenu.set_cancel_code(MENU_CODE_POWERUP_RESET_NO);
                self.mMenu.reset_menu();

                if MENU_CODE_CLEAR_POWERUP_WEIGHTS == ret {
                    self.miDialogYesButton.set_code(MENU_CODE_POWERUP_CLEAR_YES);
                } else {
                    self.miDialogYesButton.set_code(MENU_CODE_POWERUP_RESET_YES);
                }
            } else if MENU_CODE_POWERUP_RESET_YES == ret || MENU_CODE_POWERUP_RESET_NO == ret || MENU_CODE_POWERUP_CLEAR_YES == ret {
                self.miDialogImage.set_visible(false);
                self.miDialogAreYouText.set_visible(false);
                self.miDialogSureText.set_visible(false);
                self.miDialogYesButton.set_visible(false);
                self.miDialogNoButton.set_visible(false);

                self.mMenu.set_initial_focus(ctl_ptr(self.miOverride));
                self.mMenu.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

                self.mMenu.restore_current();

                if MENU_CODE_POWERUP_RESET_YES == ret {
                    //restore default powerup weights for powerup selection menu
                    for iPowerup in 0..NP {
                        let pos = iPowerupPositionMap[iPowerup] as usize;
                        let iDefaultValue = default_powerup_setting(game_values.poweruppreset as usize, pos);
                        self.miPowerupSlider[iPowerup].set_current_value(iDefaultValue);
                        game_values.powerupweights[pos] = iDefaultValue;

                        game_values.allPowerupPresets[game_values.poweruppreset as usize][pos] = iDefaultValue;
                    }
                } else if MENU_CODE_POWERUP_CLEAR_YES == ret {
                    //restore default powerup weights for powerup selection menu
                    for iPowerup in 0..NP {
                        self.miPowerupSlider[iPowerup].set_current_value(0);
                        game_values.powerupweights[iPowerup] = 0;

                        game_values.allPowerupPresets[game_values.poweruppreset as usize][iPowerup] = 0;
                    }
                }
            } else if MENU_CODE_NEIGHBOR_UP == ret {
                self.move_prev();
            } else if MENU_CODE_NEIGHBOR_DOWN == ret {
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
}
