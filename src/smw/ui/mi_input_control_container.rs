//! Port of src/smw/ui/MI_InputControlContainer.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::*;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign, UI_Control, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::harness;
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_GetKeyName, SDL_JoystickNameForIndex, SDL_Keycode, SDL_Rect, SDL_KeyCode, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP, SDL_PRESSED};
use std::ffi::CStr;

const GameInputNames: [&str; NUM_KEYS] = ["Left", "Right", "Jump", "Down", "Turbo", "Use Item", "Pause", "Exit"];
const MenuInputNames: [&str; NUM_KEYS] = ["Up", "Down", "Left", "Right", "Select", "Cancel", "Random", "Fast Map"];

/**************************************
 * MI_InputControlField Class
 **************************************/

pub struct MI_InputControlField {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,
    pub szName: String,

    pub iWidth: i16,
    pub iIndent: i16,

    pub iDevice: i16,
    pub iKey: *mut SDL_Keycode,
    pub iType: i16,
    pub iKeyIndex: i16,
    pub iPlayerIndex: i16,
}
crate::impl_base!(MI_InputControlField => ui_control: UI_Control);

impl MI_InputControlField {
    pub const Joynames: [&'static str; 30] = [
        "Joystick Up", "Joystick Down", "Joystick Left", "Joystick Right", "Stick 2 Up", "Stick 2 Down", "Stick 2 Left", "Stick 2 Right", "Pad Up", "Pad Down",
        "Pad Left", "Pad Right", "Button 1", "Button 2", "Button 3", "Button 4", "Button 5", "Button 6", "Button 7", "Button 8",
        "Button 9", "Button 10", "Button 11", "Button 12", "Button 13", "Button 14", "Button 15", "Button 16", "Button 17", "Button 18",
    ];

    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16) -> Self {
        let mut this = MI_InputControlField {
            ui_control: UI_Control::new(x, y),
            spr: nspr,
            szName: name.into(),
            iWidth: width,
            iIndent: indent,
            iDevice: DEVICE_KEYBOARD,
            iKey: std::ptr::null_mut(),
            iType: 0,
            iKeyIndex: 0,
            iPlayerIndex: 0,
        };
        this.fSelected = false;
        this
    }

    pub fn set_device(&mut self, device: i16) {
        self.iDevice = device;
    }
    pub fn set_key(&mut self, key: *mut SDL_Keycode) {
        self.iKey = key;
    }
    pub fn set_type(&mut self, type_: i16) {
        self.iType = type_;
    }
    pub fn set_key_index(&mut self, keyindex: i16) {
        self.iKeyIndex = keyindex;
    }
    pub fn set_player_index(&mut self, playerindex: i16) {
        self.iPlayerIndex = playerindex;
    }

    /// `SetKey(SDL_Keycode* iSetKey, SDL_Keycode key, short device)`
    pub fn set_key_device(&mut self, iSetKey: *mut SDL_Keycode, key: SDL_Keycode, device: i16) {
        unsafe {
            let mut fNeedSwap = false;
            let mut iSwapPlayer: i16 = 0;
            let mut iSwapKey: i16 = 0;

            for iPlayer in 0..4i32 {
                if game_values.playerInput.inputControls[iPlayer as usize].iDevice != device {
                    continue;
                }

                for selKey in 0..NUM_KEYS as i32 {
                    if selKey == self.iKeyIndex as i32 && iPlayer == self.iPlayerIndex as i32 {
                        continue;
                    }

                    if game_values.playerInput.inputControls[iPlayer as usize].inputGameControls[self.iType as usize].keys[selKey as usize] == key {
                        fNeedSwap = true;
                        iSwapPlayer = iPlayer as i16;
                        iSwapKey = selKey as i16;
                        break;
                    }
                }

                if fNeedSwap {
                    break;
                }
            }

            if fNeedSwap {
                game_values.playerInput.inputControls[iSwapPlayer as usize].inputGameControls[self.iType as usize].keys[iSwapKey as usize] = *iSetKey;
            }

            *iSetKey = key;
        }
    }
}

impl UI_ControlTrait for MI_InputControlField {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, _playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            let mut event: SDL_Event = std::mem::zeroed();
            let mut done = false;
            #[cfg(target_os = "emscripten")]
            let mut events = std::mem::take(&mut crate::smw::gs_menu::frame_events).into_iter();

            while !done {
                #[cfg(not(target_os = "emscripten"))]
                harness::wait_event(&mut event);
                #[cfg(target_os = "emscripten")]
                match events.next() {
                    Some(next) => event = next,
                    None => return MENU_CODE_NONE,
                }

                game_values.playerInput.update(event, 1);

                let event_type = event.type_;
                let is = |t: SDL_EventType| event_type == t as u32;

                if self.iDevice == DEVICE_KEYBOARD {
                    if is(SDL_EventType::SDL_KEYDOWN) {
                        let key: SDL_Keycode = event.key.keysym.sym;

                        self.set_key_device(self.iKey, key, self.iDevice);
                        done = true;
                    } else if is(SDL_EventType::SDL_MOUSEMOTION) {
                        let xmag = event.motion.xrel.abs() as i16;
                        let ymag = event.motion.yrel.abs() as i16;

                        if (xmag as i32) < MOUSE_X_DEAD_ZONE && (ymag as i32) < MOUSE_Y_DEAD_ZONE {
                            continue;
                        }

                        let key: SDL_Keycode = if xmag > ymag {
                            if event.motion.xrel < 0 {
                                MOUSE_LEFT
                            } else {
                                MOUSE_RIGHT
                            }
                        } else if event.motion.yrel < 0 {
                            MOUSE_UP
                        } else {
                            MOUSE_DOWN
                        };

                        if key != KEY_NONE {
                            self.set_key_device(self.iKey, key, self.iDevice);
                            done = true;
                        }
                    } else if is(SDL_EventType::SDL_MOUSEBUTTONDOWN) {
                        let key: SDL_Keycode = event.button.button as i32 + MOUSE_BUTTON_START;
                        self.set_key_device(self.iKey, key, self.iDevice);
                        done = true;
                    }
                } else if is(SDL_EventType::SDL_KEYDOWN) {
                    if event.key.keysym.sym == SDL_KeyCode::SDLK_ESCAPE as i32 {
                        done = true;
                    }
                } else if is(SDL_EventType::SDL_JOYHATMOTION) {
                    let mut key: SDL_Keycode = KEY_NONE;
                    let value = event.jhat.value as u32;

                    if value & SDL_HAT_UP as u32 != 0 {
                        key = JOY_HAT_UP;
                    } else if value & SDL_HAT_DOWN as u32 != 0 {
                        key = JOY_HAT_DOWN;
                    } else if value & SDL_HAT_LEFT as u32 != 0 {
                        key = JOY_HAT_LEFT;
                    } else if value & SDL_HAT_RIGHT as u32 != 0 {
                        key = JOY_HAT_RIGHT;
                    }

                    if key != KEY_NONE {
                        self.set_key_device(self.iKey, key, self.iDevice);
                        done = true;
                    }
                } else if is(SDL_EventType::SDL_JOYAXISMOTION) {
                    let mut key: SDL_Keycode = KEY_NONE;
                    let value = event.jaxis.value as i32;

                    if event.jaxis.axis == 0 {
                        if value < -JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_1_LEFT;
                        } else if value > JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_1_RIGHT;
                        }
                    } else if event.jaxis.axis == 1 {
                        if value < -JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_1_UP;
                        } else if value > JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_1_DOWN;
                        }
                    } else if event.jaxis.axis == 2 {
                        if value < -JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_2_LEFT;
                        } else if value > JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_2_RIGHT;
                        }
                    } else if event.jaxis.axis == 3 {
                        if value < -JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_2_UP;
                        } else if value > JOYSTICK_DEAD_ZONE {
                            key = JOY_STICK_2_DOWN;
                        }
                    }

                    if key != KEY_NONE {
                        self.set_key_device(self.iKey, key, self.iDevice);
                        done = true;
                    }
                } else if is(SDL_EventType::SDL_JOYBUTTONDOWN) && event.jbutton.state as u32 == SDL_PRESSED as u32 {
                    let key: SDL_Keycode = event.jbutton.button as i32 + JOY_BUTTON_START;
                    self.set_key_device(self.iKey, key, self.iDevice);
                    done = true;
                }
            }

            //Need to clear down keys when coming into this (we'll ignore key up events in here)
            game_values.playerInput.reset_keys();
        }

        self.fModifying = false;
        MENU_CODE_UNSELECT_ITEM
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let iIndent = self.iIndent as i32;
        let iWidth = self.iWidth as i32;

        unsafe {
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: if self.fSelected { 32 } else { 0 }, w: iIndent - 16, h: 32 });
            self.spr.draw_src(x + iIndent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            self.spr.draw_src(x + iIndent + 16, y, &SDL_Rect { x: 528 - iWidth + iIndent, y: if self.fSelected { 32 } else { 0 }, w: iWidth - iIndent - 16, h: 32 });

            rm.menu_font_large.draw_chop_right(x + 16, y + 5, iIndent - 8, &self.szName);

            if self.iKey.is_null() {
                rm.menu_font_large.draw_chop_right(x + iIndent + 8, y + 5, iWidth - iIndent - 16, "Unassigned");
            } else if self.fModifying {
                rm.menu_font_large.draw_chop_right(x + iIndent + 8, y + 5, iWidth - iIndent - 16, "(Press Button)");
            } else if self.iDevice == DEVICE_KEYBOARD {
                let name = CStr::from_ptr(SDL_GetKeyName(*self.iKey)).to_string_lossy();
                rm.menu_font_large.draw_chop_right(x + iIndent + 8, y + 5, iWidth - iIndent - 16, &name);
            } else {
                rm.menu_font_large.draw_chop_right(x + iIndent + 8, y + 5, iWidth - iIndent - 16, Self::Joynames[*self.iKey as usize]);
            }
        }
    }
}

/**************************************
 * MI_InputControlContainer Class
 **************************************/

pub struct MI_InputControlContainer {
    pub ui_control: UI_Control,

    pub iPlayerID: i16,
    pub iDevice: i16,
    pub iSelectedInputType: i16,

    pub mInputMenu: Box<UI_Menu>,

    pub miImage: [Ptr<MI_Image>; 2],
    pub miText: Ptr<MI_Text>,
    pub miDeviceSelectField: Ptr<MI_SelectField<i16>>,
    pub miInputTypeButton: Ptr<MI_Button>,
    pub miGameInputControlFields: [Ptr<MI_InputControlField>; NUM_KEYS],
    pub miMenuInputControlFields: [Ptr<MI_InputControlField>; NUM_KEYS],

    pub miBackButton: Ptr<MI_Button>,
}
crate::impl_base!(MI_InputControlContainer => ui_control: UI_Control);

fn key_ptr(iPlayerID: i16, iType: usize, iKey: usize) -> *mut SDL_Keycode {
    unsafe { &mut game_values.playerInput.inputControls[iPlayerID as usize].inputGameControls[iType].keys[iKey] as *mut SDL_Keycode }
}

impl MI_InputControlContainer {
    //call with x = 94, y = 19
    pub fn new(spr_button: Ptr<gfxSprite>, x: i16, y: i16, playerID: i16) -> Self {
        unsafe {
            let iPlayerID = playerID;
            let mut iDevice = game_values.playerInput.inputControls[iPlayerID as usize].iDevice;
            let iSelectedInputType: i16 = 0;

            let szTitle = format!("Player {} Controls", iPlayerID as i32 + 1);
            let miText = Ptr::new_box(MI_HeaderText::new(szTitle, 320, 5));

            let miImage = [
                Ptr::new_box(MI_Image::new(spr_button, 0, 0, 0, 0, 320, 32, 1, 1, 0)),
                Ptr::new_box(MI_Image::new(spr_button, 320, 0, 192, 0, 320, 32, 1, 1, 0)),
            ];

            let mut miDeviceSelectField = Ptr::new_box(MI_SelectField::<i16>::new(spr_button, x + 16, y + 38, "Device", 420, 150));
            miDeviceSelectField.set_item_changed_code(MENU_CODE_INPUT_DEVICE_CHANGED);
            miDeviceSelectField.add("Keyboard", -1);

            for iJoystick in 0..joystickcount {
                let p = SDL_JoystickNameForIndex(iJoystick as i32);
                let name = if p.is_null() { String::new() } else { CStr::from_ptr(p).to_string_lossy().into_owned() };
                miDeviceSelectField.add_random(name, iJoystick, false);
            }

            //If the device is not found, default to the keyboard
            if !miDeviceSelectField.set_current_value(iDevice) {
                iDevice = DEVICE_KEYBOARD;
                miDeviceSelectField.set_current_value(iDevice);
            }

            let mut miInputTypeButton = Ptr::new_box(MI_Button::new(spr_button, x + 336, y + 84, "Game", 100, TextAlign::CENTER));
            miInputTypeButton.set_code(MENU_CODE_INPUT_TYPE_CHANGED);

            let mut miGameInputControlFields: [Ptr<MI_InputControlField>; NUM_KEYS] = [Ptr::null(); NUM_KEYS];
            for iKey in 0..NUM_KEYS {
                let mut f = Ptr::new_box(MI_InputControlField::new(spr_button, x + 16, y + 118 + iKey as i16 * 34, GameInputNames[iKey], 420, 150));
                f.set_device(iDevice);
                f.set_type(0);
                f.set_key_index(iKey as i16);
                f.set_player_index(iPlayerID);
                f.set_key(key_ptr(iPlayerID, 0, iKey));
                miGameInputControlFields[iKey] = f;
            }

            let mut miMenuInputControlFields: [Ptr<MI_InputControlField>; NUM_KEYS] = [Ptr::null(); NUM_KEYS];
            for iKey in 0..NUM_KEYS {
                let mut f = Ptr::new_box(MI_InputControlField::new(spr_button, x + 16, y + 118 + iKey as i16 * 34, MenuInputNames[iKey], 420, 150));
                f.set_device(iDevice);
                f.set_type(1);
                f.set_key_index(iKey as i16);
                f.set_player_index(iPlayerID);
                f.set_key(key_ptr(iPlayerID, 1, iKey));
                miMenuInputControlFields[iKey] = f;
            }

            let mut miBackButton = Ptr::new_box(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 544, 432, "Back", 80, TextAlign::CENTER));
            miBackButton.set_code(MENU_CODE_BACK_TO_CONTROLS_MENU);

            let mut this = MI_InputControlContainer {
                ui_control: UI_Control::new(x, y),
                iPlayerID,
                iDevice,
                iSelectedInputType,
                mInputMenu: Box::new(UI_Menu::new()),
                miImage,
                miText,
                miDeviceSelectField,
                miInputTypeButton,
                miGameInputControlFields,
                miMenuInputControlFields,
                miBackButton,
            };

            let g = |i: usize| ctl_ptr(miGameInputControlFields[i]);
            let m = |i: usize| ctl_ptr(miMenuInputControlFields[i]);
            let dev = ctl_ptr(miDeviceSelectField);
            let typ = ctl_ptr(miInputTypeButton);
            let back = ctl_ptr(miBackButton);
            let null = Ptr::null();

            let menu = &mut this.mInputMenu;
            menu.set_cancel_code(MENU_CODE_BACK_TO_CONTROLS_MENU);
            menu.add_non_control(ctl_ptr(miImage[0]));
            menu.add_non_control(ctl_ptr(miImage[1]));
            menu.add_non_control(ctl_ptr(miText));

            menu.add_control(dev, back, typ, null, back);
            menu.add_control(typ, dev, g(0), null, back);
            menu.add_control(g(0), typ, g(1), null, back);
            menu.add_control(g(1), g(0), g(2), null, back);
            menu.add_control(g(2), g(1), g(3), null, back);
            menu.add_control(g(3), g(2), g(4), null, back);
            menu.add_control(g(4), g(3), g(5), null, back);
            menu.add_control(g(5), g(4), g(6), null, back);
            menu.add_control(g(6), g(5), g(7), null, back);
            menu.add_control(g(7), g(6), m(0), null, back);
            menu.add_control(m(0), g(7), m(1), null, back);
            menu.add_control(m(1), m(0), m(2), null, back);
            menu.add_control(m(2), m(1), m(3), null, back);
            menu.add_control(m(3), m(2), m(4), null, back);
            menu.add_control(m(4), m(3), m(5), null, back);
            menu.add_control(m(5), m(4), m(6), null, back);
            menu.add_control(m(6), m(5), m(7), null, back);
            menu.add_control(m(7), m(6), back, null, back);

            menu.add_control(back, m(7), dev, dev, null);

            menu.set_initial_focus(dev);

            this.set_visible_input_fields();
            this
        }
    }

    fn set_visible_input_fields(&mut self) {
        let selDevice = self.miDeviceSelectField.current_value() as i32;

        for iKey in 0..NUM_KEYS {
            let visible = 0 == self.iSelectedInputType && (iKey < 6 || DEVICE_KEYBOARD as i32 != selDevice || self.iPlayerID == 0);
            self.miGameInputControlFields[iKey].set_visible(visible);
        }

        for iKey in 0..NUM_KEYS {
            let visible = 1 == self.iSelectedInputType && (iKey < 6 || DEVICE_KEYBOARD as i32 != selDevice || self.iPlayerID == 0);
            self.miMenuInputControlFields[iKey].set_visible(visible);
        }
    }

    pub fn set_player(&mut self, playerID: i16) {
        unsafe {
            //Hide input options that other players are using
            self.miDeviceSelectField.hide_all_items(false);

            for iPlayer in 0..4i16 {
                if iPlayer == playerID {
                    continue;
                }

                let device = game_values.playerInput.inputControls[iPlayer as usize].iDevice;
                if device > -1 {
                    self.miDeviceSelectField.hide_item(device, true);
                }
            }

            self.iPlayerID = playerID;
            self.iDevice = game_values.playerInput.inputControls[self.iPlayerID as usize].iDevice;
            let iDevice = self.iDevice;
            self.miDeviceSelectField.set_current_value(iDevice);

            for iKey in 0..NUM_KEYS {
                let mut f = self.miGameInputControlFields[iKey];
                f.set_device(iDevice);
                f.set_type(0);
                f.set_key_index(iKey as i16);
                f.set_player_index(self.iPlayerID);
                f.set_key(key_ptr(self.iPlayerID, 0, iKey));
            }

            for iKey in 0..NUM_KEYS {
                let mut f = self.miMenuInputControlFields[iKey];
                f.set_device(iDevice);
                f.set_type(1);
                f.set_key_index(iKey as i16);
                f.set_player_index(self.iPlayerID);
                f.set_key(key_ptr(self.iPlayerID, 1, iKey));
            }

            self.set_visible_input_fields();

            let szNewTitle = format!("Player {} Controls", self.iPlayerID as i32 + 1);
            self.miText.set_text(szNewTitle);
            self.mInputMenu.reset_menu();
        }
    }

    pub fn update_device_keys(&mut self, lDevice: i16) {
        unsafe {
            let p = self.iPlayerID as usize;
            // Not in upstream: a joystick keeps its own bindings, and a keyboard player takes the first
            // keyboard set no other player has (player p's own unless joysticks moved them, see assign_inputs).
            let control = if lDevice == DEVICE_KEYBOARD {
                let taken = |q: usize| (0..4).any(|o| o != p && game_values.playerInput.inputControls[o].as_ptr() == &mut game_values.inputConfiguration[q][0] as *mut CInputPlayerControl);
                let q = (0..4).find(|&q| !taken(q)).unwrap_or(p);
                &mut game_values.inputConfiguration[q][0]
            } else {
                &mut game_values.inputConfiguration[lDevice as usize][1]
            };
            game_values.playerInput.inputControls[p] = Ptr::from_mut(control);
            game_values.playerInput.inputControls[p].iDevice = lDevice;

            for iKey in 0..NUM_KEYS {
                let mut f = self.miGameInputControlFields[iKey];
                f.set_device(lDevice);
                f.set_key(key_ptr(self.iPlayerID, 0, iKey));
            }

            for iKey in 0..NUM_KEYS {
                let mut f = self.miMenuInputControlFields[iKey];
                f.set_device(lDevice);
                f.set_key(key_ptr(self.iPlayerID, 1, iKey));
            }

            self.set_visible_input_fields();
        }
    }
}

impl UI_ControlTrait for MI_InputControlContainer {
    crate::impl_ctl!();

    fn update(&mut self) {
        self.mInputMenu.update();
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        self.mInputMenu.draw();
    }

    fn send_input(&mut self, mut playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let ret = self.mInputMenu.send_input(playerInput);

        if MENU_CODE_CANCEL_INPUT == ret {
            self.fModifying = false;
            return MENU_CODE_UNSELECT_ITEM;
        } else if MENU_CODE_INPUT_TYPE_CHANGED == ret {
            if 0 == self.iSelectedInputType {
                self.iSelectedInputType = 1;
                self.miInputTypeButton.set_name("Menu");
            } else {
                self.iSelectedInputType = 0;
                self.miInputTypeButton.set_name("Game");
            }

            self.set_visible_input_fields();

            return MENU_CODE_NONE;
        } else if MENU_CODE_INPUT_DEVICE_CHANGED == ret {
            //TODO: Need to handle writing out input configurations, modifying input configs here
            //Need to handle case where reading in and using a device that is no longer
            //an option will crash the system (joystick unplugged or something)

            let v = self.miDeviceSelectField.current_value();
            self.update_device_keys(v);
            playerInput.reset_keys();

            return MENU_CODE_NONE;
        }

        ret
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }
}
