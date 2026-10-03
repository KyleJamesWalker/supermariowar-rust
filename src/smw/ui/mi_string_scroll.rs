//! Port of src/smw/ui/MI_StringScroll.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::gs_gameplay::lookup_team_id;
use sdl2::sys::SDL_Rect;

pub struct StringScrollElement {
    str: String,
    pub _alias: Aliased,
}

impl StringScrollElement {
    pub fn new(text: &str) -> Self {
        StringScrollElement { _alias: Aliased::new(), str: text.to_string() }
    }

    pub fn draw(&self, x: i32, y: i32, max_w: u32) {
        unsafe { rm.menu_font_large.draw_chop_right(x + 28, y + 5, max_w as i32, &self.str) };
    }
}

pub struct MI_StringScroll {
    pub ui_control: UI_Control,

    pub items: Vec<StringScrollElement>,
    /// Index into `items` (the C++ iterator).
    pub current: usize,

    pub spr: Ptr<gfxSprite>,

    pub iNumLines: i16,
    pub iSelectedLine: i16,
    pub iIndex: u16,
    pub iWidth: i16,
    pub iOffset: i16,

    pub iTopStop: i16,
    pub iBottomStop: i16,

    pub sTitle: String,

    pub iAcceptCode: MenuCodeEnum,
    pub iCancelCode: MenuCodeEnum,
}
crate::impl_base!(MI_StringScroll => ui_control: UI_Control);

impl MI_StringScroll {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, numlines: i16) -> Self {
        MI_StringScroll {
            ui_control: UI_Control::new(x, y),
            items: Vec::new(),
            current: 0,
            spr: nspr,
            iNumLines: numlines,
            iSelectedLine: -1,
            iIndex: 0,
            iWidth: width,
            iOffset: 0,
            iTopStop: ((numlines as i32 - 1) >> 1) as i16,
            iBottomStop: 0,
            sTitle: String::new(),
            iAcceptCode: MENU_CODE_NONE,
            iCancelCode: MENU_CODE_NONE,
        }
    }

    pub fn add(&mut self, item: &str) {
        self.items.push(StringScrollElement::new(item));

        self.current = 0;
        self.iIndex = 0;

        self.iBottomStop = (self.items.len() as i64 - self.iNumLines as i64 + self.iTopStop as i64) as i16;
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.current = 0;
        self.iIndex = 0;
        self.iBottomStop = 0;
        self.iSelectedLine = -1;
        self.iOffset = 0;
    }

    pub fn set_accept_code(&mut self, code: MenuCodeEnum) {
        self.iAcceptCode = code;
    }

    pub fn set_cancel_code(&mut self, code: MenuCodeEnum) {
        self.iCancelCode = code;
    }

    pub fn activate(&mut self) {
        self.iSelectedLine = 0;
    }

    pub fn deactivate(&mut self) {
        self.iSelectedLine = -1;
    }

    pub fn current_index(&self) -> u16 {
        self.iIndex
    }

    pub fn move_next(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }

        if self.iIndex as usize == self.items.len() - 1 {
            return false;
        }

        self.iIndex += 1;
        self.current += 1;

        if self.iIndex as i32 > self.iTopStop as i32 && self.iIndex as i32 <= self.iBottomStop as i32 {
            self.iOffset += 1;
        } else {
            self.iSelectedLine += 1;
        }

        true
    }

    pub fn move_prev(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }

        if self.iIndex == 0 {
            return false;
        }

        self.iIndex -= 1;
        self.current -= 1;

        if self.iIndex as i32 >= self.iTopStop as i32 && (self.iIndex as i32) < self.iBottomStop as i32 {
            self.iOffset -= 1;
        } else {
            self.iSelectedLine -= 1;
        }

        true
    }
}

impl UI_ControlTrait for MI_StringScroll {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            for iPlayer in 0..4i16 {
                //Only allow the controlling team to control the menu (if there is one)
                if self.iControllingTeam != -1 && (self.iControllingTeam != lookup_team_id(iPlayer) || game_values.playercontrol[iPlayer as usize] != 1) {
                    continue;
                }

                let out = &playerInput.outputControls[iPlayer as usize];
                if out.menu_down().fPressed {
                    self.move_next();
                    return MENU_CODE_NONE;
                }

                if out.menu_up().fPressed {
                    self.move_prev();
                    return MENU_CODE_NONE;
                }

                if out.menu_left().fPressed {
                    if !self.neighbor(MenuNavDirection::Left).is_null() {
                        return MENU_CODE_UNSELECT_ITEM;
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_right().fPressed {
                    if !self.neighbor(MenuNavDirection::Right).is_null() {
                        return MENU_CODE_UNSELECT_ITEM;
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_select().fPressed {
                    return self.iAcceptCode;
                }

                if out.menu_cancel().fPressed {
                    return self.iCancelCode;
                }
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self) {}

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let iWidth = self.iWidth as i32;
        let iNumLines = self.iNumLines as i32;

        unsafe {
            rm.menu_dialog.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: iWidth - 16, h: iNumLines * 32 + 32 });
            rm.menu_dialog.draw_src(x + iWidth - 16, y, &SDL_Rect { x: 496, y: 0, w: 16, h: iNumLines * 32 + 32 });
            rm.menu_dialog.draw_src(x, y + iNumLines * 32 + 32, &SDL_Rect { x: 0, y: 464, w: iWidth - 16, h: 16 });
            rm.menu_dialog.draw_src(x + iWidth - 16, y + iNumLines * 32 + 32, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });

            rm.menu_font_large.draw_centered(x + (iWidth >> 1), y + 5, &self.sTitle);

            //Draw each filter field
            let mut iLine: i16 = 0;
            while iLine < self.iNumLines && (iLine as u16 as usize) < self.items.len() {
                let iLineWidth: i16 = (iWidth - 32) as i16;
                let iHalfLineWidth: i16 = iLineWidth >> 1;
                let (lw, hlw, l) = (iLineWidth as i32, iHalfLineWidth as i32, iLine as i32);
                let selY = if self.iSelectedLine == iLine { 32 } else { 0 };
                self.spr.draw_src(x + 16, y + 32 + l * 32, &SDL_Rect { x: 0, y: selY, w: hlw, h: 32 });
                self.spr.draw_src(x + 16 + hlw, y + 32 + l * 32, &SDL_Rect { x: 512 - lw + hlw, y: selY, w: lw - hlw, h: 32 });

                self.items[(self.iOffset as i32 + l) as usize].draw(x, 32 + y + l * 32, (iWidth - 104) as u32);
                iLine += 1;
            }
        }
    }
}
