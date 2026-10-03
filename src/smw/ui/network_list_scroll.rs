//! Port of src/smw/ui/NetworkListScroll.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::{game_values, rm};
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::gs_gameplay::lookup_team_id;
use sdl2::sys::SDL_Rect;

#[derive(Clone, Debug, Default)]
pub struct NLS_ListItem {
    pub sLeft: String,  // Left-aligned text (eg. name)
    pub sRight: String, // Right-aligned text (eg. player count)
}

impl NLS_ListItem {
    pub fn new(left: &str, right: &str) -> Self {
        NLS_ListItem { sLeft: left.to_string(), sRight: right.to_string() }
    }
}

pub struct MI_NetworkListScroll {
    pub ui_control: UI_Control,

    items: Vec<NLS_ListItem>,

    spr: Ptr<gfxSprite>,
    iAcceptCode: MenuCodeEnum,
    iCancelCode: MenuCodeEnum,

    sTitle: String,
    iNumLines: i16,
    iSelectedLine: i16,
    iSelectedLineBackup: i16,
    iIndex: u16,
    /// Null means `&iIndex` (the C++ default).
    iRemoteIndex: *mut u16,
    iWidth: i16,
    iOffset: i16,

    iTopStop: i16,
    iBottomStop: i16,
}
crate::impl_base!(MI_NetworkListScroll => ui_control: UI_Control);

impl MI_NetworkListScroll {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, numlines: i16, title: &str, acceptCode: MenuCodeEnum, cancelCode: MenuCodeEnum) -> Self {
        MI_NetworkListScroll {
            ui_control: UI_Control::new(x, y),
            items: Vec::new(),
            spr: nspr,
            iAcceptCode: acceptCode,
            iCancelCode: cancelCode,
            sTitle: title.to_string(),
            iNumLines: numlines,
            iSelectedLine: -1,
            iSelectedLineBackup: 0,
            iIndex: 0,
            iRemoteIndex: std::ptr::null_mut(),
            iWidth: width,
            iOffset: 0,
            iTopStop: (numlines - 1) >> 1,
            iBottomStop: 0,
        }
    }

    fn remote_index_mut(&mut self) -> &mut u16 {
        if self.iRemoteIndex.is_null() {
            &mut self.iIndex
        } else {
            unsafe { &mut *self.iRemoteIndex }
        }
    }

    pub fn add(&mut self, left: &str, right: &str) {
        self.items.push(NLS_ListItem::new(left, right));

        if !self.items.is_empty() {
            self.iIndex = 0;
        }

        self.iBottomStop = (self.items.len() as i64 - self.iNumLines as i64 + self.iTopStop as i64) as i16;
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.iIndex = 0;
        self.iSelectedLine = -1;
        self.iSelectedLineBackup = 0;
        self.iOffset = 0;
        *self.remote_index_mut() = 0;
    }

    pub fn move_next(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }

        if self.iIndex as usize == self.items.len() - 1 {
            return false;
        }

        self.iIndex += 1;
        let i = self.iIndex;
        *self.remote_index_mut() = i;

        if (self.iIndex as i32) > self.iTopStop as i32 && (self.iIndex as i32) <= self.iBottomStop as i32 {
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
        let i = self.iIndex;
        *self.remote_index_mut() = i;

        if (self.iIndex as i32) >= self.iTopStop as i32 && (self.iIndex as i32) < self.iBottomStop as i32 {
            self.iOffset -= 1;
        } else {
            self.iSelectedLine -= 1;
        }

        true
    }

    pub fn remote_index(&mut self, index: *mut u16) {
        self.iRemoteIndex = index;
    }
}

impl UI_ControlTrait for MI_NetworkListScroll {
    crate::impl_ctl!();

    fn update(&mut self) {}

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        unsafe {
            let (x, y) = (self.m_pos.x as i32, self.m_pos.y as i32);
            let w = self.iWidth as i32;
            let n = self.iNumLines as i32;
            rm.menu_dialog.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: w - 16, h: n * 32 + 32 });
            rm.menu_dialog.draw_src(x + w - 16, y, &SDL_Rect { x: 496, y: 0, w: 16, h: n * 32 + 32 });
            rm.menu_dialog.draw_src(x, y + n * 32 + 32, &SDL_Rect { x: 0, y: 464, w: w - 16, h: 16 });
            rm.menu_dialog.draw_src(x + w - 16, y + n * 32 + 32, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });

            rm.menu_font_large.draw_centered(x + (w >> 1), y + 5, &self.sTitle);

            let mut iLine: i16 = 0;
            while iLine < self.iNumLines && (iLine as u16 as usize) < self.items.len() {
                let iHalfLineWidth: i16 = (self.iWidth - 32) >> 1;
                let iLineWidth: i16 = self.iWidth - 32;
                let sel = if self.iSelectedLine == iLine { 32 } else { 0 };
                let ly = y + 32 + iLine as i32 * 32;
                self.spr.draw_src(x + 16, ly, &SDL_Rect { x: 0, y: sel, w: iHalfLineWidth as i32, h: 32 });
                self.spr.draw_src(
                    x + 16 + iHalfLineWidth as i32,
                    ly,
                    &SDL_Rect { x: 512 - iLineWidth as i32 + iHalfLineWidth as i32, y: sel, w: (iLineWidth - iHalfLineWidth) as i32, h: 32 },
                );

                let item = &self.items[(self.iOffset + iLine) as usize];
                rm.menu_font_large.draw_chop_right(x + 28, y + 5 + iLine as i32 * 32 + 32, w - 104, &item.sLeft);
                rm.menu_font_large.draw_right_justified(x + 28 + w - 64, y + 5 + iLine as i32 * 32 + 32, &item.sRight);
                iLine += 1;
            }
        }
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            for iPlayer in 0..4usize {
                if self.iControllingTeam != -1
                    && (self.iControllingTeam != lookup_team_id(iPlayer as i16) || game_values.playercontrol[iPlayer] != 1)
                {
                    continue;
                }

                let out = &playerInput.outputControls[iPlayer];

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
                        self.iSelectedLineBackup = self.iSelectedLine;
                        self.iSelectedLine = -1;
                        return MENU_CODE_UNSELECT_ITEM;
                    }
                    return MENU_CODE_NONE;
                }

                if out.menu_right().fPressed {
                    if !self.neighbor(MenuNavDirection::Right).is_null() {
                        self.iSelectedLineBackup = self.iSelectedLine;
                        self.iSelectedLine = -1;
                        return MENU_CODE_UNSELECT_ITEM;
                    }
                    return MENU_CODE_NONE;
                }

                if out.menu_select().fPressed {
                    if self.items.is_empty() {
                        return MENU_CODE_NONE;
                    }

                    println!("Selected index: {}", *self.remote_index_mut());
                    return self.iAcceptCode;
                }

                if out.menu_cancel().fPressed {
                    return self.iCancelCode;
                }
            }
        }

        MENU_CODE_NONE
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        if self.iSelectedLine == -1 {
            self.iSelectedLine = self.iSelectedLineBackup;
        }

        MENU_CODE_MODIFY_ACCEPTED
    }
}
