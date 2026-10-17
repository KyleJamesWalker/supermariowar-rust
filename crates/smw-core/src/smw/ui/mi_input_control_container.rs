//! Port of src/smw/ui/MI_InputControlContainer.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::resource_manager::SpriteId;
use crate::common::keys::*;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use smw_platform::{InputEvent, MouseEvent};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::*;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::harness;
use sdl2::sys::{SDL_Event, SDL_GetKeyName, SDL_JoystickName, SDL_JoystickNameForIndex, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP};
use std::ffi::CStr;

const GameInputNames: [&str; NUM_KEYS] = ["Left", "Right", "Jump", "Down", "Turbo", "Use Item", "Pause", "Exit"];
const MenuInputNames: [&str; NUM_KEYS] = ["Up", "Down", "Left", "Right", "Select", "Cancel", "Random", "Fast Map"];

/**************************************
 * MI_InputControlField Class
 **************************************/

pub struct MI_InputControlField {
    pub ui_control: UI_Control,

    pub spr: SpriteId,
    pub szName: String,

    pub iWidth: i16,
    pub iIndent: i16,

    pub iDevice: i16,
    pub iKey: *mut Keycode,
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

    pub fn new(nspr: SpriteId, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16) -> Self {
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
    pub fn set_key(&mut self, key: *mut Keycode) {
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

    /// `SetKey(Keycode* iSetKey, Keycode key, short device)`
    pub fn set_key_device(&mut self, g: &mut Game, iSetKey: *mut Keycode, key: Keycode, device: i16) {
        unsafe {
            let mut fNeedSwap = false;
            let mut iSwapPlayer: i16 = 0;
            let mut iSwapKey: i16 = 0;

            for iPlayer in 0..4i32 {
                if g.input_control(g.game_values.playerInput.inputControls[iPlayer as usize]).iDevice != device {
                    continue;
                }

                for selKey in 0..NUM_KEYS as i32 {
                    if selKey == self.iKeyIndex as i32 && iPlayer == self.iPlayerIndex as i32 {
                        continue;
                    }

                    if g.input_control(g.game_values.playerInput.inputControls[iPlayer as usize]).inputGameControls[self.iType as usize].keys[selKey as usize] == key {
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
                g.input_control(g.game_values.playerInput.inputControls[iSwapPlayer as usize]).inputGameControls[self.iType as usize].keys[iSwapKey as usize] = *iSetKey;
            }

            *iSetKey = key;
        }
    }
}

impl UI_ControlTrait for MI_InputControlField {
    crate::impl_ctl!();

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, g: &mut Game, _playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let mut gp = g.ptr();
        let mut done = false;
        #[cfg(target_os = "emscripten")]
        let mut events = std::mem::take(&mut g.frame_events).into_iter();

        while !done {
            #[cfg(not(target_os = "emscripten"))]
            let input = harness::wait_event(g);
            #[cfg(target_os = "emscripten")]
            let Some(input) = events.next() else {
                return MENU_CODE_NONE;
            };

            gp.game_values.playerInput.update_input(g, &input, 1);

            use InputEvent as E;
            let ev = &input.event;
            let key_sym = if let E::Key { key, .. } = *ev { key.0 } else { 0 };
            let (motion_xrel, motion_yrel) = if let E::Mouse { event: MouseEvent::Motion { xrel, yrel, .. }, .. } = *ev { (xrel, yrel) } else { (0, 0) };
            let mouse_button = if let E::Mouse { event: MouseEvent::Button { button, .. }, .. } = *ev { button } else { 0 };
            let jhat_value = if let E::PadHat { value, .. } = *ev { value } else { 0 };
            let (jaxis_axis, jaxis_value) = if let E::PadAxis { axis, value, .. } = *ev { (axis, value) } else { (0, 0) };
            let jbutton_button = if let E::PadButton { button, .. } = *ev { button } else { 0 };

            if self.iDevice == DEVICE_KEYBOARD {
                if matches!(ev, E::Key { down: true, .. }) {
                    let key: Keycode = key_sym;

                    self.set_key_device(g, self.iKey, key, self.iDevice);
                    done = true;
                } else if matches!(ev, E::Mouse { event: MouseEvent::Motion { .. }, .. }) {
                    let xmag = motion_xrel.abs() as i16;
                    let ymag = motion_yrel.abs() as i16;

                    if (xmag as i32) < MOUSE_X_DEAD_ZONE && (ymag as i32) < MOUSE_Y_DEAD_ZONE {
                        continue;
                    }

                    let key: Keycode = if xmag > ymag {
                        if motion_xrel < 0 {
                            MOUSE_LEFT
                        } else {
                            MOUSE_RIGHT
                        }
                    } else if motion_yrel < 0 {
                        MOUSE_UP
                    } else {
                        MOUSE_DOWN
                    };

                    if key != KEY_NONE {
                        self.set_key_device(g, self.iKey, key, self.iDevice);
                        done = true;
                    }
                } else if matches!(ev, E::Mouse { event: MouseEvent::Button { down: true, .. }, .. }) {
                    let key: Keycode = mouse_button as i32 + MOUSE_BUTTON_START;
                    self.set_key_device(g, self.iKey, key, self.iDevice);
                    done = true;
                }
            } else if matches!(ev, E::Key { down: true, .. }) {
                if key_sym == SDLK_ESCAPE {
                    done = true;
                }
            } else if matches!(ev, E::PadHat { .. }) {
                let mut key: Keycode = KEY_NONE;
                let value = jhat_value as u32;

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
                    self.set_key_device(g, self.iKey, key, self.iDevice);
                    done = true;
                }
            } else if matches!(ev, E::PadAxis { .. }) {
                let mut key: Keycode = KEY_NONE;
                let value = jaxis_value as i32;

                if jaxis_axis == 0 {
                    if value < -JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_1_LEFT;
                    } else if value > JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_1_RIGHT;
                    }
                } else if jaxis_axis == 1 {
                    if value < -JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_1_UP;
                    } else if value > JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_1_DOWN;
                    }
                } else if jaxis_axis == 2 {
                    if value < -JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_2_LEFT;
                    } else if value > JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_2_RIGHT;
                    }
                } else if jaxis_axis == 3 {
                    if value < -JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_2_UP;
                    } else if value > JOYSTICK_DEAD_ZONE {
                        key = JOY_STICK_2_DOWN;
                    }
                }

                if key != KEY_NONE {
                    self.set_key_device(g, self.iKey, key, self.iDevice);
                    done = true;
                }
            } else if matches!(ev, E::PadButton { down: true, .. }) {
                let key: Keycode = jbutton_button as i32 + JOY_BUTTON_START;
                self.set_key_device(g, self.iKey, key, self.iDevice);
                done = true;
            }
        }

        //Need to clear down keys when coming into this (we'll ignore key up events in here)
        g.game_values.playerInput.reset_keys();

        self.fModifying = false;
        MENU_CODE_UNSELECT_ITEM
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let iIndent = self.iIndent as i32;
        let iWidth = self.iWidth as i32;

        unsafe {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: 0, y: if self.fSelected { 32 } else { 0 }, w: iIndent - 16, h: 32 });
            g.rm.sprite(self.spr).draw_src(g, x + iIndent - 16, y, &Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            g.rm.sprite(self.spr).draw_src(g, x + iIndent + 16, y, &Rect { x: 528 - iWidth + iIndent, y: if self.fSelected { 32 } else { 0 }, w: iWidth - iIndent - 16, h: 32 });

            gp.rm.menu_font_large.draw_chop_right(g, x + 16, y + 5, iIndent - 8, &self.szName);

            if self.iKey.is_null() {
                gp.rm.menu_font_large.draw_chop_right(g, x + iIndent + 8, y + 5, iWidth - iIndent - 16, "Unassigned");
            } else if self.fModifying {
                gp.rm.menu_font_large.draw_chop_right(g, x + iIndent + 8, y + 5, iWidth - iIndent - 16, "(Press Button)");
            } else if self.iDevice == DEVICE_KEYBOARD {
                let name = crate::services::services(gp.get()).keyboard.key_name(*self.iKey);
                gp.rm.menu_font_large.draw_chop_right(g, x + iIndent + 8, y + 5, iWidth - iIndent - 16, &name);
            } else {
                gp.rm.menu_font_large.draw_chop_right(g, x + iIndent + 8, y + 5, iWidth - iIndent - 16, Self::Joynames[*self.iKey as usize]);
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

    pub miImage: [Ctl<MI_Image>; 2],
    pub miText: Ctl<MI_Text>,
    pub miDeviceSelectField: Ctl<MI_SelectField<i16>>,
    pub miInputTypeButton: Ctl<MI_Button>,
    pub miGameInputControlFields: [Ctl<MI_InputControlField>; NUM_KEYS],
    pub miMenuInputControlFields: [Ctl<MI_InputControlField>; NUM_KEYS],

    pub miBackButton: Ctl<MI_Button>,
    /// `pad::generation()` when the device list was filled.
    devices: u32,
}
crate::impl_base!(MI_InputControlContainer => ui_control: UI_Control);

unsafe fn add_joysticks(g: &mut Game, field: &mut MI_SelectField<i16>) {
    for iJoystick in 0..g.devices.len() as i16 {
        let js = g.joystick(iJoystick as usize);
        let p = if js.is_null() { SDL_JoystickNameForIndex(iJoystick as i32) } else { SDL_JoystickName(js) };
        let name = if harness::replay_joysticks(g).is_some() {
            "Virtual Controller".to_string()
        } else if p.is_null() {
            String::new()
        } else if harness::replaying(g) {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        } else {
            let name = CStr::from_ptr(p).to_string_lossy().into_owned();
            let label = crate::smw::pad_type::controller_type(iJoystick as i32, &name).label();
            // The field is about 30 characters wide; browser ids are longer, so those show the family alone.
            if label.len() + name.len() < 30 { format!("{}: {}", label, name) } else { label.to_string() }
        };
        field.add_random(name, iJoystick, false);
    }
}

fn key_ptr(g: &mut Game, iPlayerID: i16, iType: usize, iKey: usize) -> *mut Keycode {
    &mut g.input_control(g.game_values.playerInput.inputControls[iPlayerID as usize]).inputGameControls[iType].keys[iKey] as *mut Keycode
}

impl MI_InputControlContainer {
    //call with x = 94, y = 19
    pub fn new(g: &mut Game, spr_button: SpriteId, x: i16, y: i16, playerID: i16) -> Self {
        let mut gp = g.ptr();
        unsafe {
            let iPlayerID = playerID;
            let mut iDevice = g.input_control(g.game_values.playerInput.inputControls[iPlayerID as usize]).iDevice;
            let iSelectedInputType: i16 = 0;

            let szTitle = format!("Player {} Controls", iPlayerID as i32 + 1);
            let miText = Ctl::new(MI_HeaderText::new(g, szTitle, 320, 5), g);

            let miImage = [
                Ctl::new(MI_Image::new(spr_button, 0, 0, 0, 0, 320, 32, 1, 1, 0), g),
                Ctl::new(MI_Image::new(spr_button, 320, 0, 192, 0, 320, 32, 1, 1, 0), g),
            ];

            let mut miDeviceSelectField = Ctl::new(MI_SelectField::<i16>::new(spr_button, x + 16, y + 38, "Device", 420, 150), g);
            g.ctl(miDeviceSelectField).set_item_changed_code(MENU_CODE_INPUT_DEVICE_CHANGED);
            g.ctl(miDeviceSelectField).add("Keyboard", -1);

            add_joysticks(g, g.ctl(miDeviceSelectField));

            //If the device is not found, default to the keyboard
            if !g.ctl(miDeviceSelectField).set_current_value(iDevice) {
                iDevice = DEVICE_KEYBOARD;
                g.ctl(miDeviceSelectField).set_current_value(iDevice);
            }

            let mut miInputTypeButton = Ctl::new(MI_Button::new(g, spr_button, x + 336, y + 84, "Game", 100, TextAlign::CENTER), g);
            g.ctl(miInputTypeButton).set_code(MENU_CODE_INPUT_TYPE_CHANGED);

            let mut miGameInputControlFields: [Ctl<MI_InputControlField>; NUM_KEYS] = [Default::default(); NUM_KEYS];
            for iKey in 0..NUM_KEYS {
                let mut f = Ctl::new(MI_InputControlField::new(spr_button, x + 16, y + 118 + iKey as i16 * 34, GameInputNames[iKey], 420, 150), g);
                g.ctl(f).set_device(iDevice);
                g.ctl(f).set_type(0);
                g.ctl(f).set_key_index(iKey as i16);
                g.ctl(f).set_player_index(iPlayerID);
                g.ctl(f).set_key(key_ptr(g, iPlayerID, 0, iKey));
                miGameInputControlFields[iKey] = f;
            }

            let mut miMenuInputControlFields: [Ctl<MI_InputControlField>; NUM_KEYS] = [Default::default(); NUM_KEYS];
            for iKey in 0..NUM_KEYS {
                let mut f = Ctl::new(MI_InputControlField::new(spr_button, x + 16, y + 118 + iKey as i16 * 34, MenuInputNames[iKey], 420, 150), g);
                g.ctl(f).set_device(iDevice);
                g.ctl(f).set_type(1);
                g.ctl(f).set_key_index(iKey as i16);
                g.ctl(f).set_player_index(iPlayerID);
                g.ctl(f).set_key(key_ptr(g, iPlayerID, 1, iKey));
                miMenuInputControlFields[iKey] = f;
            }

            let mut miBackButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(miBackButton).set_code(MENU_CODE_BACK_TO_CONTROLS_MENU);

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
                devices: 0,
            };

            let gc = |i: usize| miGameInputControlFields[i].id();
            let m = |i: usize| miMenuInputControlFields[i].id();
            let dev = miDeviceSelectField.id();
            let typ = miInputTypeButton.id();
            let back = miBackButton.id();
            let null = ControlId::NONE;

            let menu = &mut this.mInputMenu;
            menu.set_cancel_code(MENU_CODE_BACK_TO_CONTROLS_MENU);
            menu.add_non_control(miImage[0].id());
            menu.add_non_control(miImage[1].id());
            menu.add_non_control(miText.id());

            menu.add_control(g, dev, back, typ, null, back);
            menu.add_control(g, typ, dev, gc(0), null, back);
            menu.add_control(g, gc(0), typ, gc(1), null, back);
            menu.add_control(g, gc(1), gc(0), gc(2), null, back);
            menu.add_control(g, gc(2), gc(1), gc(3), null, back);
            menu.add_control(g, gc(3), gc(2), gc(4), null, back);
            menu.add_control(g, gc(4), gc(3), gc(5), null, back);
            menu.add_control(g, gc(5), gc(4), gc(6), null, back);
            menu.add_control(g, gc(6), gc(5), gc(7), null, back);
            menu.add_control(g, gc(7), gc(6), m(0), null, back);
            menu.add_control(g, m(0), gc(7), m(1), null, back);
            menu.add_control(g, m(1), m(0), m(2), null, back);
            menu.add_control(g, m(2), m(1), m(3), null, back);
            menu.add_control(g, m(3), m(2), m(4), null, back);
            menu.add_control(g, m(4), m(3), m(5), null, back);
            menu.add_control(g, m(5), m(4), m(6), null, back);
            menu.add_control(g, m(6), m(5), m(7), null, back);
            menu.add_control(g, m(7), m(6), back, null, back);

            menu.add_control(g, back, m(7), dev, dev, null);

            menu.set_initial_focus(gp.get(), dev);

            this.set_visible_input_fields(g);
            this
        }
    }

    fn set_visible_input_fields(&mut self, g: &mut Game) {
        let selDevice = g.ctl(self.miDeviceSelectField).current_value() as i32;

        for iKey in 0..NUM_KEYS {
            let visible = 0 == self.iSelectedInputType && (iKey < 6 || DEVICE_KEYBOARD as i32 != selDevice || self.iPlayerID == 0);
            g.ctl(self.miGameInputControlFields[iKey]).set_visible(visible);
        }

        for iKey in 0..NUM_KEYS {
            let visible = 1 == self.iSelectedInputType && (iKey < 6 || DEVICE_KEYBOARD as i32 != selDevice || self.iPlayerID == 0);
            g.ctl(self.miMenuInputControlFields[iKey]).set_visible(visible);
        }
    }

    pub fn set_player(&mut self, g: &mut Game, playerID: i16) {
        // Not in upstream: list the pads connected since (pad::hotplug).
        #[cfg(not(target_os = "emscripten"))]
        if self.devices != crate::smw::pad::generation() {
            self.devices = crate::smw::pad::generation();
            g.ctl(self.miDeviceSelectField).clear();
            g.ctl(self.miDeviceSelectField).add("Keyboard", -1);
            unsafe { add_joysticks(g, g.ctl(self.miDeviceSelectField)) };
        }

        //Hide input options that other players are using
        g.ctl(self.miDeviceSelectField).hide_all_items(false);

        for iPlayer in 0..4i16 {
            if iPlayer == playerID {
                continue;
            }

            let device = g.input_control(g.game_values.playerInput.inputControls[iPlayer as usize]).iDevice;
            if device > -1 {
                g.ctl(self.miDeviceSelectField).hide_item(device, true);
            }
        }

        self.iPlayerID = playerID;
        self.iDevice = g.input_control(g.game_values.playerInput.inputControls[self.iPlayerID as usize]).iDevice;
        let iDevice = self.iDevice;
        g.ctl(self.miDeviceSelectField).set_current_value(iDevice);

        for iKey in 0..NUM_KEYS {
            let mut f = self.miGameInputControlFields[iKey];
            g.ctl(f).set_device(iDevice);
            g.ctl(f).set_type(0);
            g.ctl(f).set_key_index(iKey as i16);
            g.ctl(f).set_player_index(self.iPlayerID);
            g.ctl(f).set_key(key_ptr(g, self.iPlayerID, 0, iKey));
        }

        for iKey in 0..NUM_KEYS {
            let mut f = self.miMenuInputControlFields[iKey];
            g.ctl(f).set_device(iDevice);
            g.ctl(f).set_type(1);
            g.ctl(f).set_key_index(iKey as i16);
            g.ctl(f).set_player_index(self.iPlayerID);
            g.ctl(f).set_key(key_ptr(g, self.iPlayerID, 1, iKey));
        }

        self.set_visible_input_fields(g);

        let szNewTitle = format!("Player {} Controls", self.iPlayerID as i32 + 1);
        g.ctl(self.miText).set_text(szNewTitle);
        self.mInputMenu.reset_menu(g);
    }

    pub fn update_device_keys(&mut self, g: &mut Game, lDevice: i16) {
        let p = self.iPlayerID as usize;
        // Not in upstream: a joystick keeps its own bindings, and a keyboard player takes the first
        // keyboard set no other player has (player p's own unless joysticks moved them, see assign_inputs).
        let control = if lDevice == DEVICE_KEYBOARD {
            let taken = |q: usize| (0..4).any(|o| o != p && g.game_values.playerInput.inputControls[o] == Some((q as u8, 0)));
            let q = (0..4).find(|&q| !taken(q)).unwrap_or(p);
            (q as u8, 0)
        } else {
            (lDevice as u8, 1)
        };
        g.game_values.playerInput.inputControls[p] = Some(control);
        g.input_control(g.game_values.playerInput.inputControls[p]).iDevice = lDevice;

        for iKey in 0..NUM_KEYS {
            let mut f = self.miGameInputControlFields[iKey];
            g.ctl(f).set_device(lDevice);
            g.ctl(f).set_key(key_ptr(g, self.iPlayerID, 0, iKey));
        }

        for iKey in 0..NUM_KEYS {
            let mut f = self.miMenuInputControlFields[iKey];
            g.ctl(f).set_device(lDevice);
            g.ctl(f).set_key(key_ptr(g, self.iPlayerID, 1, iKey));
        }

        self.set_visible_input_fields(g);
    }
}

impl UI_ControlTrait for MI_InputControlContainer {
    crate::impl_ctl!();

    fn update(&mut self, g: &mut Game) {
        self.mInputMenu.update(g);
    }

    fn draw(&mut self, g: &mut Game) {
        if !self.m_visible {
            return;
        }

        self.mInputMenu.draw(g);
    }

    fn send_input(&mut self, g: &mut Game, mut playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let ret = self.mInputMenu.send_input(g, playerInput);

        if MENU_CODE_CANCEL_INPUT == ret {
            self.fModifying = false;
            return MENU_CODE_UNSELECT_ITEM;
        } else if MENU_CODE_INPUT_TYPE_CHANGED == ret {
            if 0 == self.iSelectedInputType {
                self.iSelectedInputType = 1;
                g.ctl(self.miInputTypeButton).set_name(g, "Menu");
            } else {
                self.iSelectedInputType = 0;
                g.ctl(self.miInputTypeButton).set_name(g, "Game");
            }

            self.set_visible_input_fields(g);

            return MENU_CODE_NONE;
        } else if MENU_CODE_INPUT_DEVICE_CHANGED == ret {
            //TODO: Need to handle writing out input configurations, modifying input configs here
            //Need to handle case where reading in and using a device that is no longer
            //an option will crash the system (joystick unplugged or something)

            let v = g.ctl(self.miDeviceSelectField).current_value();
            self.update_device_keys(g, v);
            playerInput.reset_keys();

            return MENU_CODE_NONE;
        }

        ret
    }

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }
}
