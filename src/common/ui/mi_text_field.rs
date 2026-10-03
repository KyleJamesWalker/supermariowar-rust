//! Port of src/common/ui/MI_TextField.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::{CPlayerInput, DEVICE_KEYBOARD};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::{SDL_GetKeyboardState, SDL_Keycode, SDL_Rect, SDL_Scancode};

const NUMBER_KEY_MAP: [i16; 10] = [41, 33, 64, 35, 36, 37, 94, 38, 42, 40];

pub struct MI_TextField {
    pub ui_control: UI_Control,

    pub iCursorIndex: i16,
    pub iNumChars: i16,
    pub iMaxChars: i16,

    pub spr: Ptr<gfxSprite>,
    pub szName: String,
    pub szTempValue: String,
    pub szOutValue: *mut String,

    pub iWidth: i16,
    pub iIndent: i16,

    pub miModifyCursor: Box<MI_Image>,

    pub mcItemChangedCode: MenuCodeEnum,
    pub mcControlSelectedCode: MenuCodeEnum,

    pub iAdjustmentY: i16,
    pub iStringWidth: i16,
    pub iAllowedWidth: i16,

    pub szDisallowedChars: String,
}
crate::impl_base!(MI_TextField => ui_control: UI_Control);

impl MI_TextField {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: &str, width: i16, indent: i16) -> Self {
        let ui_control = UI_Control::new(x, y);
        let mut miModifyCursor = Box::new(MI_Image::new(nspr, ui_control.m_pos.x + indent, ui_control.m_pos.y + 4, 136, 64, 15, 24, 4, 1, 8));
        miModifyCursor.set_blink(true, 20);
        miModifyCursor.set_visible(false);

        MI_TextField {
            ui_control,
            iCursorIndex: 0,
            iNumChars: 0,
            iMaxChars: 0,
            spr: nspr,
            szName: name.to_string(),
            szTempValue: String::new(),
            szOutValue: std::ptr::null_mut(),
            iWidth: width,
            iIndent: indent,
            miModifyCursor,
            mcItemChangedCode: MENU_CODE_NONE,
            mcControlSelectedCode: MENU_CODE_NONE,
            iAdjustmentY: if width > 256 { 0 } else { 128 },
            iStringWidth: 0,
            iAllowedWidth: width - indent - 24,
            szDisallowedChars: String::new(),
        }
    }

    pub fn set_title(&mut self, name: impl Into<String>) {
        self.szName = name.into();
    }

    /// Gets the values of the currently selected item
    pub fn get_value(&self) -> &String {
        unsafe { self.szOutValue.as_ref().expect("MI_TextField::GetValue without data") }
    }

    pub fn clear(&mut self) {
        unsafe { self.szOutValue.as_mut().expect("MI_TextField::Clear without data").clear() };
        self.iCursorIndex = 0;
        self.iNumChars = 0;
    }

    /// When the item is changed, this code will be returned from SendInput()
    pub fn set_item_changed_code(&mut self, code: MenuCodeEnum) {
        self.mcItemChangedCode = code;
    }
    pub fn set_control_selected_code(&mut self, code: MenuCodeEnum) {
        self.mcControlSelectedCode = code;
    }

    /// Set where the data of this control is written to (some member of game_values probably)
    pub fn set_data(&mut self, data: &mut String, maxchars: i16) {
        self.iMaxChars = maxchars;
        self.szOutValue = data as *mut String;
        self.iCursorIndex = data.len() as i16;
        self.iNumChars = self.iCursorIndex;

        self.szTempValue.clear();
        self.szTempValue.reserve(self.iMaxChars.max(0) as usize);

        self.update_cursor();
    }

    pub fn set_disallowed_chars(&mut self, chars: &str) {
        self.szDisallowedChars = chars.to_string();
    }

    fn update_cursor(&mut self) {
        if self.szOutValue.is_null() {
            return;
        }

        let mut bytes = unsafe { (*self.szOutValue).as_bytes().to_vec() };
        bytes.resize(self.iCursorIndex as usize, 0);
        // byte-level std::string::resize; the font only reads bytes
        self.szTempValue = unsafe { String::from_utf8_unchecked(bytes) };

        self.iStringWidth = unsafe { rm.menu_font_large.get_width(&self.szTempValue) } as i16;
        let (px, py) = (self.m_pos.x, self.m_pos.y);
        if self.iStringWidth <= self.iAllowedWidth {
            self.miModifyCursor.set_position(px + self.iIndent + 10 + self.iStringWidth, py + 4);
        } else {
            self.miModifyCursor.set_position(px + self.iIndent + 10 + self.iAllowedWidth, py + 4);
        }
    }
}

impl UI_ControlTrait for MI_TextField {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_UNSELECT_ITEM;
        }

        if MENU_CODE_NONE != self.mcControlSelectedCode {
            return self.mcControlSelectedCode;
        }

        self.miModifyCursor.set_visible(modify);
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let keystate = unsafe { SDL_GetKeyboardState(std::ptr::null_mut()) };

        for iPlayer in 0..4usize {
            // NOTE: copied from UI_Menu::SendInput
            // Only let player 1 on the keyboard control the menu
            unsafe {
                if iPlayer != 0
                    && !game_values.playerInput.inputControls[iPlayer].is_null()
                    && game_values.playerInput.inputControls[iPlayer].iDevice == DEVICE_KEYBOARD
                {
                    continue;
                }
            }

            if playerInput.outputControls[iPlayer].menu_select().fPressed || playerInput.outputControls[iPlayer].menu_cancel().fPressed {
                self.miModifyCursor.set_visible(false);

                self.fModifying = false;

                return MENU_CODE_UNSELECT_ITEM;
            }
        }

        if self.szOutValue.is_null() || self.iNumChars >= self.iMaxChars {
            return MENU_CODE_NONE;
        }

        let k = |c: sdl2::sys::SDL_KeyCode| c as SDL_Keycode;

        // TODO: check string conversion
        //Watch for characters typed in including delete and backspace
        let mut key: SDL_Keycode = playerInput.iPressedKey;
        if (key >= k(SDLK_a) && key <= k(SDLK_z))
            || key == k(SDLK_SPACE)
            || (key >= k(SDLK_0) && key <= k(SDLK_9))
            || key == k(SDLK_EQUALS)
            || key == k(SDLK_MINUS)
            || key == k(SDLK_BACKQUOTE)
            || (key >= k(SDLK_LEFTBRACKET) && key <= k(SDLK_RIGHTBRACKET))
            || key == k(SDLK_SEMICOLON)
            || key == k(SDLK_QUOTE)
            || key == k(SDLK_COMMA)
            || key == k(SDLK_PERIOD)
            || key == k(SDLK_SLASH)
        {
            if self.iNumChars < self.iMaxChars - 1 {
                //Take care of holding shift to shift the pressed key to another character
                let shift = unsafe {
                    *keystate.add(SDL_Scancode::SDL_SCANCODE_LSHIFT as usize) != 0 || *keystate.add(SDL_Scancode::SDL_SCANCODE_RSHIFT as usize) != 0
                };
                if shift {
                    if key >= k(SDLK_a) && key <= k(SDLK_z) {
                        key -= 32;
                    } else if key >= k(SDLK_0) && key <= k(SDLK_9) {
                        key = NUMBER_KEY_MAP[(key - 48) as usize] as SDL_Keycode;
                    } else if key == k(SDLK_MINUS) {
                        key = k(SDLK_UNDERSCORE);
                    } else if key == k(SDLK_EQUALS) {
                        key = k(SDLK_PLUS);
                    } else if key == k(SDLK_BACKQUOTE) {
                        key = 126;
                    } else if key >= k(SDLK_LEFTBRACKET) && key <= k(SDLK_RIGHTBRACKET) {
                        key += 32;
                    } else if key == k(SDLK_SEMICOLON) {
                        key = k(SDLK_COLON);
                    } else if key == k(SDLK_QUOTE) {
                        key = k(SDLK_QUOTEDBL);
                    } else if key == k(SDLK_COMMA) {
                        key = k(SDLK_LESS);
                    } else if key == k(SDLK_PERIOD) {
                        key = k(SDLK_GREATER);
                    } else if key == k(SDLK_SLASH) {
                        key = k(SDLK_QUESTION);
                    }
                }

                //Check to see if this is an allowed character for this field
                let mut fAllowed = true;
                if self.szDisallowedChars.as_bytes().contains(&(key as u8)) {
                    fAllowed = false;
                }

                //If it is an allowed character, then add it to the field
                if fAllowed {
                    unsafe { (*self.szOutValue).as_mut_vec().insert(self.iCursorIndex as usize, key as u8) };
                    self.iCursorIndex += 1;
                    self.iNumChars += 1;

                    self.update_cursor();
                    return self.mcItemChangedCode;
                }
            }
        } else if key == k(SDLK_BACKSPACE) {
            if self.iCursorIndex > 0 {
                self.iCursorIndex -= 1;
                self.iNumChars -= 1;
                unsafe { (*self.szOutValue).as_mut_vec().remove(self.iCursorIndex as usize) };

                self.update_cursor();
                return self.mcItemChangedCode;
            }
        } else if key == k(SDLK_DELETE) {
            if self.iCursorIndex < self.iNumChars {
                unsafe { (*self.szOutValue).as_mut_vec().remove(self.iCursorIndex as usize) };
                self.iNumChars -= 1;

                self.update_cursor();
                return self.mcItemChangedCode;
            }
        } else if key == k(SDLK_LEFT) {
            if self.iCursorIndex > 0 {
                self.iCursorIndex -= 1;
                self.update_cursor();
            }
        } else if key == k(SDLK_RIGHT) {
            if self.iCursorIndex < self.iNumChars {
                self.iCursorIndex += 1;
                self.update_cursor();
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self) {
        self.miModifyCursor.update();
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.iWidth as i32;
        let indent = self.iIndent as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.iAdjustmentY as i32;

        unsafe {
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: selY, w: indent - 16, h: 32 });
            self.spr.draw_src(x + indent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            self.spr.draw_src(x + indent + 16, y, &SDL_Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

            rm.menu_font_large.draw_chop_right(x + 16, y + 5, indent - 8, &self.szName);

            if !self.szOutValue.is_null() {
                if self.iStringWidth <= self.iAllowedWidth || !self.fModifying {
                    rm.menu_font_large.draw_chop_right(x + indent + 8, y + 5, self.iAllowedWidth as i32, &*self.szOutValue);
                } else {
                    rm.menu_font_large.draw_chop_left(x + width - 16, y + 5, self.iAllowedWidth as i32, &self.szTempValue);
                }
            }
        }

        self.miModifyCursor.draw();
    }

    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.szOutValue.is_null() || self.fDisable {
            return MENU_CODE_NONE;
        }

        //If we are modifying this control, see if we clicked on a next/prev button
        if self.fModifying {
            //Move cursor to index in string where clicked
            let mut iPixelCount: i16 = 0;
            for iChar in 0..self.iNumChars {
                let szChar = [unsafe { (*self.szOutValue).as_bytes()[iChar as usize] }];
                let szChar = unsafe { std::str::from_utf8_unchecked(&szChar) };
                iPixelCount = (iPixelCount as i32 + unsafe { rm.menu_font_large.get_width(szChar) }) as i16;

                if iPixelCount as i32 >= iMouseX as i32 - (self.m_pos.x as i32 + self.iIndent as i32 + 8) {
                    self.iCursorIndex = iChar;
                    self.update_cursor();
                    return MENU_CODE_NONE;
                }
            }
        }

        //Otherwise just check to see if we clicked on the whole control
        let (mx, my) = (iMouseX as i32, iMouseY as i32);
        if mx >= self.m_pos.x as i32 && mx < self.m_pos.x as i32 + self.iWidth as i32 && my >= self.m_pos.y as i32 && my < self.m_pos.y as i32 + 32 {
            self.iCursorIndex = unsafe { (&*self.szOutValue).len() } as i16;
            self.update_cursor();
            return MENU_CODE_CLICKED;
        }

        //Otherwise this control wasn't clicked at all
        MENU_CODE_NONE
    }

    fn refresh(&mut self) {
        //Look at destination string and update control based on that value
        if self.szOutValue.is_null() {
            return;
        }

        let p = self.szOutValue;
        self.set_data(unsafe { &mut *p }, self.iMaxChars);
    }
}
