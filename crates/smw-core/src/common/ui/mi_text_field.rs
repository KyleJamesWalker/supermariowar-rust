//! Port of src/common/ui/MI_TextField.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::{CPlayerInput, DEVICE_KEYBOARD};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::{SDL_Keycode, SDL_Rect, SDL_Scancode};

const NUMBER_KEY_MAP: [i16; 10] = [41, 33, 64, 35, 36, 37, 94, 38, 42, 40];

/// Not in upstream: the on-screen keyboard's character rows, unshifted and shifted, and its bottom row
/// (first column, columns spanned, label).
const OSK_KEYS: [&[u8; 10]; 4] = [b"1234567890", b"qwertyuiop", b"asdfghjkl-", b"zxcvbnm,./"];
const OSK_SHIFTED: [&[u8; 10]; 4] = [b"!@#$%^&*()", b"QWERTYUIOP", b"ASDFGHJKL_", b"ZXCVBNM<>?"];
const OSK_BOTTOM: [(i16, i16, &str); 4] = [(0, 3, "Shift"), (3, 3, "Space"), (6, 2, "Del"), (8, 2, "Done")];
const OSK_ROWS: i16 = 5;
const OSK_COLS: i16 = 10;
const OSK_CELL: i32 = 56;
const OSK_ROW_H: i32 = 36;
const OSK_X: i32 = 24;
const OSK_W: i32 = 592;
const OSK_H: i32 = 16 + OSK_ROWS as i32 * OSK_ROW_H + 12;

/// Whether the menu press that opened the field came from a keyboard key; set by `UI_Menu`.
static mut OPENED_BY_KEY: bool = false;

/// Not in upstream. `UI_Menu` reports what opened a field: a pad button or a mouse click opens the on-screen keyboard.
pub fn note_opener(keyboard: bool) {
    unsafe { OPENED_BY_KEY = keyboard };
}

fn touch_session() -> bool {
    #[cfg(not(target_os = "emscripten"))]
    return crate::smw::touch::session();
    #[cfg(target_os = "emscripten")]
    false
}

fn is_text_key(key: SDL_Keycode) -> bool {
    let k = |c: sdl2::sys::SDL_KeyCode| c as SDL_Keycode;
    (key >= k(SDLK_a) && key <= k(SDLK_z))
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
}

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

    /// Not in upstream: the on-screen keyboard is open, its cursor, and its one-shot Shift.
    pub fOsk: bool,
    pub iOskRow: i16,
    pub iOskCol: i16,
    pub fOskShift: bool,
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
            fOsk: false,
            iOskRow: 0,
            iOskCol: 0,
            fOskShift: false,
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
        if self.fModifying {
            self.notify_page();
        }
    }

    fn insert_char(&mut self, c: u8) -> MenuCodeEnum {
        if self.iNumChars >= self.iMaxChars - 1 || self.szDisallowedChars.as_bytes().contains(&c) {
            return MENU_CODE_NONE;
        }
        unsafe { (*self.szOutValue).as_mut_vec().insert(self.iCursorIndex as usize, c) };
        self.iCursorIndex += 1;
        self.iNumChars += 1;

        self.update_cursor();
        self.mcItemChangedCode
    }

    fn backspace(&mut self) -> MenuCodeEnum {
        if self.iCursorIndex > 0 {
            self.iCursorIndex -= 1;
            self.iNumChars -= 1;
            unsafe { (*self.szOutValue).as_mut_vec().remove(self.iCursorIndex as usize) };

            self.update_cursor();
            return self.mcItemChangedCode;
        }
        MENU_CODE_NONE
    }

    fn finish(&mut self) -> MenuCodeEnum {
        self.miModifyCursor.set_visible(false);
        self.fModifying = false;
        self.fOsk = false;
        self.notify_page();
        MENU_CODE_UNSELECT_ITEM
    }

    /// Not in upstream. The on-screen keyboard: the menu directions move its cursor, select presses the key,
    /// cancel erases (or leaves an empty field). A typed character means a keyboard is here, so it closes.
    fn osk_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let key = playerInput.iPressedKey;
        if is_text_key(key) || key == SDLK_BACKSPACE as SDL_Keycode || key == SDLK_DELETE as SDL_Keycode {
            self.fOsk = false;
            return self.send_input(playerInput);
        }

        for iPlayer in 0..4usize {
            unsafe {
                if iPlayer != 0
                    && !game_values.playerInput.inputControls[iPlayer].is_null()
                    && game_values.playerInput.inputControls[iPlayer].iDevice == DEVICE_KEYBOARD
                {
                    continue;
                }
            }
            let out = &playerInput.outputControls[iPlayer];
            if out.menu_up().fPressed {
                self.iOskRow = (self.iOskRow + OSK_ROWS - 1) % OSK_ROWS;
                return MENU_CODE_NONE;
            }
            if out.menu_down().fPressed {
                self.iOskRow = (self.iOskRow + 1) % OSK_ROWS;
                return MENU_CODE_NONE;
            }
            if out.menu_left().fPressed || out.menu_right().fPressed {
                let step = if out.menu_left().fPressed { -1 } else { 1 };
                if self.iOskRow == OSK_ROWS - 1 {
                    let n = OSK_BOTTOM.len() as i16;
                    self.iOskCol = OSK_BOTTOM[((self.osk_bottom_key() as i16 + step + n) % n) as usize].0;
                } else {
                    self.iOskCol = (self.iOskCol + step + OSK_COLS) % OSK_COLS;
                }
                return MENU_CODE_NONE;
            }
            if out.menu_select().fPressed {
                return self.osk_press();
            }
            if out.menu_cancel().fPressed {
                return if self.iNumChars > 0 { self.backspace() } else { self.finish() };
            }
        }
        MENU_CODE_NONE
    }

    fn osk_bottom_key(&self) -> usize {
        OSK_BOTTOM.iter().rposition(|&(start, _, _)| self.iOskCol >= start).unwrap_or(0)
    }

    fn osk_press(&mut self) -> MenuCodeEnum {
        if self.iOskRow < OSK_ROWS - 1 {
            let rows = if self.fOskShift { OSK_SHIFTED } else { OSK_KEYS };
            self.fOskShift = false;
            return self.insert_char(rows[self.iOskRow as usize][self.iOskCol as usize]);
        }
        match self.osk_bottom_key() {
            0 => {
                self.fOskShift = !self.fOskShift;
                MENU_CODE_NONE
            }
            1 => self.insert_char(b' '),
            2 => self.backspace(),
            _ => self.finish(),
        }
    }

    /// Drawn over the menu, away from the field: below it for a field in the top half, else above.
    fn draw_osk(&mut self) {
        let top = if (self.m_pos.y as i32) < 240 { 480 - OSK_H - 8 } else { 8 };
        // The dialog image is 512 wide, so the panel is its left and right halves.
        let half = OSK_W / 2;
        unsafe {
            rm.menu_dialog.draw_src(OSK_X, top, &SDL_Rect { x: 0, y: 0, w: half, h: OSK_H - 16 });
            rm.menu_dialog.draw_src(OSK_X + half, top, &SDL_Rect { x: 512 - half, y: 0, w: half, h: OSK_H - 16 });
            rm.menu_dialog.draw_src(OSK_X, top + OSK_H - 16, &SDL_Rect { x: 0, y: 464, w: half, h: 16 });
            rm.menu_dialog.draw_src(OSK_X + half, top + OSK_H - 16, &SDL_Rect { x: 512 - half, y: 464, w: half, h: 16 });
        }
        let key = |spr: &Ptr<gfxSprite>, col: i16, row: i16, span: i16, selected: bool, label: &str| {
            let x = OSK_X + 16 + col as i32 * OSK_CELL;
            let y = top + 12 + row as i32 * OSK_ROW_H;
            let w = span as i32 * OSK_CELL - 4;
            let srcY = if selected { 32 } else { 0 };
            spr.draw_src(x, y, &SDL_Rect { x: 0, y: srcY, w: w - 8, h: 32 });
            spr.draw_src(x + w - 8, y, &SDL_Rect { x: 504, y: srcY, w: 8, h: 32 });
            unsafe { rm.menu_font_large.draw_centered(x + w / 2, y + 5, label) };
        };
        let rows = if self.fOskShift { OSK_SHIFTED } else { OSK_KEYS };
        for row in 0..OSK_ROWS - 1 {
            for col in 0..OSK_COLS {
                let c = rows[row as usize][col as usize];
                let label = if self.szDisallowedChars.as_bytes().contains(&c) { String::new() } else { (c as char).to_string() };
                key(&self.spr, col, row, 1, row == self.iOskRow && col == self.iOskCol, &label);
            }
        }
        let selected = if self.iOskRow == OSK_ROWS - 1 { Some(self.osk_bottom_key()) } else { None };
        for (i, &(start, span, label)) in OSK_BOTTOM.iter().enumerate() {
            let label = if i == 0 && self.fOskShift { "SHIFT" } else { label };
            key(&self.spr, start, OSK_ROWS - 1, span, selected == Some(i), label);
        }
    }

    /// Not in upstream. Tells the web page (web/textinput.js) where the field being edited is and what it
    /// holds, so a touch screen can open its on-screen keyboard; the typing still arrives as key events.
    #[cfg(target_os = "emscripten")]
    fn notify_page(&self) {
        extern "C" {
            fn emscripten_run_script(script: *const std::ffi::c_char);
        }
        let js_string = |bytes: &[u8]| {
            let mut out = String::from("\"");
            for &b in bytes {
                if b.is_ascii_alphanumeric() || b == b' ' {
                    out.push(b as char);
                } else {
                    out += &format!("\\u{:04x}", b);
                }
            }
            out + "\""
        };
        let value = if self.szOutValue.is_null() { Vec::new() } else { unsafe { (*self.szOutValue).as_bytes().to_vec() } };
        let script = format!(
            "Module.onTextField && Module.onTextField({{active: {}, x: {}, y: {}, width: {}, indent: {}, value: {}, cursor: {}, max: {}, disallowed: {}}})",
            self.fModifying,
            self.m_pos.x,
            self.m_pos.y,
            self.iWidth,
            self.iIndent,
            js_string(&value),
            self.iCursorIndex,
            self.iMaxChars - 1,
            js_string(self.szDisallowedChars.as_bytes()),
        );
        let script = std::ffi::CString::new(script).expect("no NUL in the script");
        unsafe { emscripten_run_script(script.as_ptr()) };
    }

    #[cfg(not(target_os = "emscripten"))]
    fn notify_page(&self) {}
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
        self.fOsk = modify && (touch_session() || unsafe { !OPENED_BY_KEY });
        self.iOskRow = 0;
        self.iOskCol = 0;
        self.fOskShift = false;
        self.notify_page();
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        if self.fOsk && !self.szOutValue.is_null() {
            return self.osk_input(playerInput);
        }

        let keystate = crate::smw::harness::keyboard_state();

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
                self.fOsk = false;
                self.notify_page();

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
        if is_text_key(key) {
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

                return self.insert_char(key as u8);
            }
        } else if key == k(SDLK_BACKSPACE) {
            return self.backspace();
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

    fn draw_overlay(&mut self) {
        if self.fModifying && self.fOsk {
            self.draw_osk();
        }
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
