//! A text entry menu item, with an on-screen keyboard for pads, mice and touch.

use crate::common::resource_manager::SpriteId;
use crate::common::keys::*;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::{CPlayerInput, DEVICE_KEYBOARD};
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::common::global_constants::ticks;
use crate::common::keys::{SDL_SCANCODE_LSHIFT, SDL_SCANCODE_RSHIFT};

/// The shifted characters of the number keys 0 to 9 on a US layout.
const NUMBER_KEY_MAP: [i16; 10] = [41, 33, 64, 35, 36, 37, 94, 38, 42, 40];

/// The on-screen keyboard's character rows, unshifted and shifted, and its bottom row
/// (first column, columns spanned, label).
const OSK_KEYS: [&[u8; 10]; 4] = [b"1234567890", b"qwertyuiop", b"asdfghjkl-", b"zxcvbnm,./"];
const OSK_SHIFTED: [&[u8; 10]; 4] = [b"!@#$%^&*()", b"QWERTYUIOP", b"ASDFGHJKL_", b"ZXCVBNM<>?"];
const OSK_BOTTOM: [(i16, i16, &str); 4] = [(0, 3, "Shift"), (3, 3, "Space"), (6, 2, "Del"), (8, 2, "Done")];
const OSK_ROWS: i16 = 5;
const OSK_COLS: i16 = 10;
/// On-screen keyboard geometry in pixels: key cell width, row height, panel left edge, width and height.
const OSK_CELL: i32 = 56;
const OSK_ROW_H: i32 = 36;
const OSK_X: i32 = 24;
const OSK_W: i32 = 592;
const OSK_H: i32 = 16 + OSK_ROWS as i32 * OSK_ROW_H + 12;

/// `UI_Menu` reports what opened a field: a pad button or a mouse click opens the on-screen keyboard.
pub fn note_opener(g: &mut Game, keyboard: bool) {
    g.text_field_opened_by_key = keyboard;
}

fn touch_session(g: &mut Game) -> bool {
    #[cfg(not(target_os = "emscripten"))]
    return crate::smw::touch::session(g);
    #[cfg(target_os = "emscripten")]
    false
}

fn prev_char(s: &str, i: usize) -> usize {
    s.floor_char_boundary(i.saturating_sub(1))
}

fn next_char(s: &str, i: usize) -> usize {
    s.ceil_char_boundary(i + 1)
}

fn is_text_key(key: Keycode) -> bool {
    (key >= SDLK_a && key <= SDLK_z)
        || key == SDLK_SPACE
        || (key >= SDLK_0 && key <= SDLK_9)
        || key == SDLK_EQUALS
        || key == SDLK_MINUS
        || key == SDLK_BACKQUOTE
        || (key >= SDLK_LEFTBRACKET && key <= SDLK_RIGHTBRACKET)
        || key == SDLK_SEMICOLON
        || key == SDLK_QUOTE
        || key == SDLK_COMMA
        || key == SDLK_PERIOD
        || key == SDLK_SLASH
}

/// A labeled text field that edits a `String` in place.
pub struct MI_TextField {
    pub ui_control: UI_Control,

    /// Byte index of the cursor, and the length in bytes.
    pub iCursorIndex: i16,
    pub iNumChars: i16,
    /// Buffer size: the field holds at most `iMaxChars - 1` characters.
    pub iMaxChars: i16,

    pub spr: SpriteId,
    pub szName: String,
    /// The text before the cursor, for measuring.
    pub szTempValue: String,
    /// The string being edited; null until `set_data`.
    pub szOutValue: *mut String,

    pub iWidth: i16,
    pub iIndent: i16,

    pub miModifyCursor: Box<MI_Image>,

    pub mcItemChangedCode: MenuCodeEnum,
    pub mcControlSelectedCode: MenuCodeEnum,

    pub iAdjustmentY: i16,
    /// Width in pixels of the text before the cursor, and how much the field shows before it scrolls.
    pub iStringWidth: i16,
    pub iAllowedWidth: i16,

    pub szDisallowedChars: String,

    /// The on-screen keyboard is open, its cursor, and its one-shot Shift.
    pub fOsk: bool,
    pub iOskRow: i16,
    pub iOskCol: i16,
    pub fOskShift: bool,
}
crate::impl_base!(MI_TextField => ui_control: UI_Control);

impl MI_TextField {
    pub fn new(nspr: SpriteId, x: i16, y: i16, name: &str, width: i16, indent: i16) -> Self {
        let ui_control = UI_Control::new(x, y);
        let mut miModifyCursor = Box::new(MI_Image::new(nspr, ui_control.m_pos.x + indent, ui_control.m_pos.y + 4, 136, 64, 15, 24, 4, 1, 8));
        miModifyCursor.set_blink(true, ticks(20) as i16);
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

    /// The string being edited; panics before `set_data`.
    pub fn get_value(&self) -> &String {
        unsafe { self.szOutValue.as_ref().expect("MI_TextField::GetValue without data") }
    }

    /// Empties the string being edited; panics before `set_data`.
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

    /// Edits `data` in place, holding at most `maxchars - 1` characters, with the cursor at its end.
    pub fn set_data(&mut self, g: &mut Game, data: &mut String, maxchars: i16) {
        self.iMaxChars = maxchars;
        self.szOutValue = data as *mut String;
        self.iCursorIndex = data.len() as i16;
        self.iNumChars = self.iCursorIndex;

        self.szTempValue.clear();
        self.szTempValue.reserve(self.iMaxChars.max(0) as usize);

        self.update_cursor(g);
    }

    pub fn set_disallowed_chars(&mut self, chars: &str) {
        self.szDisallowedChars = chars.to_string();
    }

    fn update_cursor(&mut self, g: &mut Game) {
        if self.szOutValue.is_null() {
            return;
        }

        let value = unsafe { &*self.szOutValue };
        self.iCursorIndex = value.floor_char_boundary(self.iCursorIndex.max(0) as usize) as i16;
        self.szTempValue.clear();
        self.szTempValue.push_str(&value[..self.iCursorIndex as usize]);

        self.iStringWidth = g.rm.menu_font_large.get_width(&self.szTempValue) as i16;
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

    fn insert_char(&mut self, g: &mut Game, c: u8) -> MenuCodeEnum {
        if !c.is_ascii() || self.iNumChars >= self.iMaxChars - 1 || self.szDisallowedChars.as_bytes().contains(&c) {
            return MENU_CODE_NONE;
        }
        unsafe { (*self.szOutValue).insert(self.iCursorIndex as usize, c as char) };
        self.iCursorIndex += 1;
        self.iNumChars += 1;

        self.update_cursor(g);
        self.mcItemChangedCode
    }

    fn backspace(&mut self, g: &mut Game) -> MenuCodeEnum {
        if self.iCursorIndex > 0 {
            let value = unsafe { &mut *self.szOutValue };
            self.iCursorIndex = prev_char(value, self.iCursorIndex as usize) as i16;
            self.iNumChars -= value.remove(self.iCursorIndex as usize).len_utf8() as i16;

            self.update_cursor(g);
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

    /// The on-screen keyboard: the menu directions move its cursor, select presses the key,
    /// cancel erases (or leaves an empty field). A typed character means a keyboard is here, so it closes.
    fn osk_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let key = playerInput.iPressedKey;
        if is_text_key(key) || key == SDLK_BACKSPACE as Keycode || key == SDLK_DELETE as Keycode {
            self.fOsk = false;
            return self.send_input(g, playerInput);
        }

        for iPlayer in 0..4usize {
            if iPlayer != 0
                && !g.input_control(g.game_values.playerInput.inputControls[iPlayer]).is_null()
                && g.input_control(g.game_values.playerInput.inputControls[iPlayer]).iDevice == DEVICE_KEYBOARD
            {
                continue;
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
                return self.osk_press(g);
            }
            if out.menu_cancel().fPressed {
                return if self.iNumChars > 0 { self.backspace(g) } else { self.finish() };
            }
        }
        MENU_CODE_NONE
    }

    fn osk_bottom_key(&self) -> usize {
        OSK_BOTTOM.iter().rposition(|&(start, _, _)| self.iOskCol >= start).unwrap_or(0)
    }

    fn osk_press(&mut self, g: &mut Game) -> MenuCodeEnum {
        if self.iOskRow < OSK_ROWS - 1 {
            let rows = if self.fOskShift { OSK_SHIFTED } else { OSK_KEYS };
            self.fOskShift = false;
            return self.insert_char(g, rows[self.iOskRow as usize][self.iOskCol as usize]);
        }
        match self.osk_bottom_key() {
            0 => {
                self.fOskShift = !self.fOskShift;
                MENU_CODE_NONE
            }
            1 => self.insert_char(g, b' '),
            2 => self.backspace(g),
            _ => self.finish(),
        }
    }

    /// Drawn over the menu, away from the field: below it for a field in the top half, else above.
    fn draw_osk(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let top = if (self.m_pos.y as i32) < 240 { 480 - OSK_H - 8 } else { 8 };
        // The dialog image is 512 wide, so the panel is its left and right halves.
        let half = OSK_W / 2;
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, OSK_X, top, &Rect { x: 0, y: 0, w: half, h: OSK_H - 16 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, OSK_X + half, top, &Rect { x: 512 - half, y: 0, w: half, h: OSK_H - 16 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, OSK_X, top + OSK_H - 16, &Rect { x: 0, y: 464, w: half, h: 16 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, OSK_X + half, top + OSK_H - 16, &Rect { x: 512 - half, y: 464, w: half, h: 16 });
        let mut key = |spr: SpriteId, col: i16, row: i16, span: i16, selected: bool, label: &str| {
            let x = OSK_X + 16 + col as i32 * OSK_CELL;
            let y = top + 12 + row as i32 * OSK_ROW_H;
            let w = span as i32 * OSK_CELL - 4;
            let srcY = if selected { 32 } else { 0 };
            g.rm.sprite(spr).draw_src(g, x, y, &Rect { x: 0, y: srcY, w: w - 8, h: 32 });
            g.rm.sprite(spr).draw_src(g, x + w - 8, y, &Rect { x: 504, y: srcY, w: 8, h: 32 });
            gp.rm.menu_font_large.draw_centered(g, x + w / 2, y + 5, label);
        };
        let rows = if self.fOskShift { OSK_SHIFTED } else { OSK_KEYS };
        for row in 0..OSK_ROWS - 1 {
            for col in 0..OSK_COLS {
                let c = rows[row as usize][col as usize];
                let label = if self.szDisallowedChars.as_bytes().contains(&c) { String::new() } else { (c as char).to_string() };
                key(self.spr, col, row, 1, row == self.iOskRow && col == self.iOskCol, &label);
            }
        }
        let selected = if self.iOskRow == OSK_ROWS - 1 { Some(self.osk_bottom_key()) } else { None };
        for (i, &(start, span, label)) in OSK_BOTTOM.iter().enumerate() {
            let label = if i == 0 && self.fOskShift { "SHIFT" } else { label };
            key(self.spr, start, OSK_ROWS - 1, span, selected == Some(i), label);
        }
    }

    /// Tells the web page (web/textinput.js) where the field being edited is and what it
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

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_UNSELECT_ITEM;
        }

        if MENU_CODE_NONE != self.mcControlSelectedCode {
            return self.mcControlSelectedCode;
        }

        self.miModifyCursor.set_visible(modify);
        self.fModifying = modify;
        self.fOsk = modify && (touch_session(g) || !g.text_field_opened_by_key);
        self.iOskRow = 0;
        self.iOskCol = 0;
        self.fOskShift = false;
        self.notify_page();
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        if self.fOsk && !self.szOutValue.is_null() {
            return self.osk_input(g, playerInput);
        }

        let keystate = crate::smw::harness::keyboard_state(g);

        for iPlayer in 0..4usize {
            // Only let player 1 on the keyboard control the menu
            if iPlayer != 0
                && !g.input_control(g.game_values.playerInput.inputControls[iPlayer]).is_null()
                && g.input_control(g.game_values.playerInput.inputControls[iPlayer]).iDevice == DEVICE_KEYBOARD
            {
                continue;
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


        //Watch for characters typed in including delete and backspace
        let mut key: Keycode = playerInput.iPressedKey;
        if is_text_key(key) {
            if self.iNumChars < self.iMaxChars - 1 {
                //Take care of holding shift to shift the pressed key to another character
                let shift = unsafe {
                    *keystate.add(SDL_SCANCODE_LSHIFT) != 0 || *keystate.add(SDL_SCANCODE_RSHIFT) != 0
                };
                if shift {
                    if key >= SDLK_a && key <= SDLK_z {
                        key -= 32;
                    } else if key >= SDLK_0 && key <= SDLK_9 {
                        key = NUMBER_KEY_MAP[(key - 48) as usize] as Keycode;
                    } else if key == SDLK_MINUS {
                        key = SDLK_UNDERSCORE;
                    } else if key == SDLK_EQUALS {
                        key = SDLK_PLUS;
                    } else if key == SDLK_BACKQUOTE {
                        key = 126;
                    } else if key >= SDLK_LEFTBRACKET && key <= SDLK_RIGHTBRACKET {
                        key += 32;
                    } else if key == SDLK_SEMICOLON {
                        key = SDLK_COLON;
                    } else if key == SDLK_QUOTE {
                        key = SDLK_QUOTEDBL;
                    } else if key == SDLK_COMMA {
                        key = SDLK_LESS;
                    } else if key == SDLK_PERIOD {
                        key = SDLK_GREATER;
                    } else if key == SDLK_SLASH {
                        key = SDLK_QUESTION;
                    }
                }

                return self.insert_char(g, key as u8);
            }
        } else if key == SDLK_BACKSPACE {
            return self.backspace(g);
        } else if key == SDLK_DELETE {
            if self.iCursorIndex < self.iNumChars {
                self.iNumChars -= unsafe { (*self.szOutValue).remove(self.iCursorIndex as usize) }.len_utf8() as i16;

                self.update_cursor(g);
                return self.mcItemChangedCode;
            }
        } else if key == SDLK_LEFT {
            if self.iCursorIndex > 0 {
                self.iCursorIndex = prev_char(unsafe { &*self.szOutValue }, self.iCursorIndex as usize) as i16;
                self.update_cursor(g);
            }
        } else if key == SDLK_RIGHT {
            if self.iCursorIndex < self.iNumChars {
                self.iCursorIndex = next_char(unsafe { &*self.szOutValue }, self.iCursorIndex as usize) as i16;
                self.update_cursor(g);
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self, g: &mut Game) {
        self.miModifyCursor.update(g);
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.iWidth as i32;
        let indent = self.iIndent as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.iAdjustmentY as i32;

        unsafe {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: 0, y: selY, w: indent - 16, h: 32 });
            g.rm.sprite(self.spr).draw_src(g, x + indent - 16, y, &Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            g.rm.sprite(self.spr).draw_src(g, x + indent + 16, y, &Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

            gp.rm.menu_font_large.draw_chop_right(g, x + 16, y + 5, indent - 8, &self.szName);

            if !self.szOutValue.is_null() {
                if self.iStringWidth <= self.iAllowedWidth || !self.fModifying {
                    gp.rm.menu_font_large.draw_chop_right(g, x + indent + 8, y + 5, self.iAllowedWidth as i32, &*self.szOutValue);
                } else {
                    gp.rm.menu_font_large.draw_chop_left(g, x + width - 16, y + 5, self.iAllowedWidth as i32, &self.szTempValue);
                }
            }
        }

        self.miModifyCursor.draw(g);
    }

    fn draw_overlay(&mut self, g: &mut Game) {
        if self.fModifying && self.fOsk {
            self.draw_osk(g);
        }
    }

    fn mouse_click(&mut self, g: &mut Game, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.szOutValue.is_null() || self.fDisable {
            return MENU_CODE_NONE;
        }

        //If we are modifying this control, see if we clicked on a next/prev button
        if self.fModifying {
            //Move cursor to index in string where clicked
            let mut iPixelCount: i16 = 0;
            let value = unsafe { &*self.szOutValue };
            for (iChar, c) in value.char_indices() {
                iPixelCount = (iPixelCount as i32 + g.rm.menu_font_large.get_width(c.encode_utf8(&mut [0; 4]))) as i16;

                if iPixelCount as i32 >= iMouseX as i32 - (self.m_pos.x as i32 + self.iIndent as i32 + 8) {
                    self.iCursorIndex = iChar as i16;
                    self.update_cursor(g);
                    return MENU_CODE_NONE;
                }
            }
        }

        //Otherwise just check to see if we clicked on the whole control
        let (mx, my) = (iMouseX as i32, iMouseY as i32);
        if mx >= self.m_pos.x as i32 && mx < self.m_pos.x as i32 + self.iWidth as i32 && my >= self.m_pos.y as i32 && my < self.m_pos.y as i32 + 32 {
            self.iCursorIndex = unsafe { (&*self.szOutValue).len() } as i16;
            self.update_cursor(g);
            return MENU_CODE_CLICKED;
        }

        //Otherwise this control wasn't clicked at all
        MENU_CODE_NONE
    }

    fn refresh(&mut self, g: &mut Game) {
        //Look at destination string and update control based on that value
        if self.szOutValue.is_null() {
            return;
        }

        let p = self.szOutValue;
        self.set_data(g, unsafe { &mut *p }, self.iMaxChars);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_steps_over_whole_characters() {
        let s = "aé€b";
        assert_eq!(next_char(s, 0), 1);
        assert_eq!(next_char(s, 1), 3);
        assert_eq!(next_char(s, 3), 6);
        assert_eq!(next_char(s, 7), 7);
        assert_eq!(prev_char(s, 7), 6);
        assert_eq!(prev_char(s, 6), 3);
        assert_eq!(prev_char(s, 3), 1);
        assert_eq!(prev_char(s, 1), 0);
    }
}
