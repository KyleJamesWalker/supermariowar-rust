//! Port of src/smw/ui/MI_MapFilterScroll.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::NUM_AUTO_FILTERS;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::gs_gameplay::lookup_team_id;
use sdl2::sys::SDL_Rect;

#[derive(Clone, Debug, Default)]
pub struct MFS_ListItem {
    pub sName: String, //Display name
    pub iIcon: i16,     //Icon to display with name
    pub fSelected: bool, //Filter selected
}

impl MFS_ListItem {
    pub fn new(sname: String, icon: i16, fselected: bool) -> Self {
        MFS_ListItem { sName: sname, iIcon: icon, fSelected: fselected }
    }
}

pub struct MI_MapFilterScroll {
    pub ui_control: UI_Control,

    pub items: Vec<Box<MFS_ListItem>>,
    /// Index into `items` (the C++ iterator).
    pub current: usize,

    pub spr: Ptr<gfxSprite>,

    pub iSelectedColumn: i16,
    pub iNumLines: i16,
    pub iSelectedLine: i16,
    pub iIndex: u16,
    pub iWidth: i16,
    pub iOffset: i16,

    pub iTopStop: i16,
    pub iBottomStop: i16,
}
crate::impl_base!(MI_MapFilterScroll => ui_control: UI_Control);

impl MI_MapFilterScroll {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, numlines: i16) -> Self {
        MI_MapFilterScroll {
            ui_control: UI_Control::new(x, y),
            items: Vec::new(),
            current: 0,
            spr: nspr,
            iSelectedColumn: 0,
            iNumLines: numlines,
            iSelectedLine: 0,
            iIndex: 0,
            iWidth: width,
            iOffset: 0,
            iTopStop: ((numlines as i32 - 1) / 2) as i16,
            iBottomStop: 0,
        }
    }

    pub fn add(&mut self, name: impl Into<String>, icon: i16) {
        let item = Box::new(MFS_ListItem::new(name.into(), icon, false));
        self.items.push(item);

        if !self.items.is_empty() {
            self.current = 0;
            self.iIndex = 0;
        }

        self.iBottomStop = (self.items.len() as i64 - self.iNumLines as i64 + self.iTopStop as i64) as i16;
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

        if (self.iIndex as i32) < NUM_AUTO_FILTERS {
            self.iSelectedColumn = 0;
        }

        true
    }
}

impl UI_ControlTrait for MI_MapFilterScroll {
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
                    self.iSelectedColumn = 0;
                    return MENU_CODE_NONE;
                }

                if out.menu_right().fPressed {
                    if self.iIndex as i32 >= NUM_AUTO_FILTERS {
                        self.iSelectedColumn = 1;
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_select().fPressed {
                    let idx = self.iIndex as usize;
                    //If the left column is selected, then turn that filter on/off
                    if self.iSelectedColumn == 0 {
                        self.items[idx].fSelected = !self.items[idx].fSelected;
                        game_values.pfFilters[idx] = !game_values.pfFilters[idx];
                    } else {
                        //otherwise if the right is selected, go into the details of that filter
                        game_values.selectedmapfilter = self.iIndex as i16;
                        return MENU_CODE_TO_MAP_FILTER_EDIT;
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_cancel().fPressed {
                    return MENU_CODE_MAP_FILTER_EXIT;
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
            //Draw the background for the map preview
            rm.menu_dialog.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: iWidth - 16, h: iNumLines * 32 + 32 });
            rm.menu_dialog.draw_src(x + iWidth - 16, y, &SDL_Rect { x: 496, y: 0, w: 16, h: iNumLines * 32 + 32 });
            rm.menu_dialog.draw_src(x, y + iNumLines * 32 + 32, &SDL_Rect { x: 0, y: 464, w: iWidth - 16, h: 16 });
            rm.menu_dialog.draw_src(x + iWidth - 16, y + iNumLines * 32 + 32, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });

            rm.menu_font_large.draw_centered(x + (iWidth >> 1), y + 5, "Map Filters");

            //Draw each filter field
            let mut iLine: i16 = 0;
            while iLine < self.iNumLines && (iLine as u16 as usize) < self.items.len() {
                let l = iLine as i32;
                let lineSel = self.iSelectedLine == iLine && self.iSelectedColumn == 0;
                if self.iOffset as i32 + l >= NUM_AUTO_FILTERS {
                    let iHalfLineWidth = ((iWidth - 64) as i16 >> 1) as i32;
                    let iLineWidth = (iWidth - 64) as i16 as i32;

                    self.spr.draw_src(x + 16, y + 32 + l * 32, &SDL_Rect { x: 0, y: if lineSel { 32 } else { 0 }, w: iHalfLineWidth, h: 32 });
                    self.spr.draw_src(
                        x + 16 + iHalfLineWidth,
                        y + 32 + l * 32,
                        &SDL_Rect { x: 512 - iLineWidth + iHalfLineWidth, y: if lineSel { 32 } else { 0 }, w: iLineWidth - iHalfLineWidth, h: 32 },
                    );

                    let colSel = self.iSelectedLine == iLine && self.iSelectedColumn == 1;
                    rm.menu_map_filter.draw_src(x + iWidth - 48, y + 32 + l * 32, &SDL_Rect { x: 48, y: if colSel { 32 } else { 0 }, w: 32, h: 32 });
                } else {
                    let iHalfLineWidth = ((iWidth - 32) as i16 >> 1) as i32;
                    let iLineWidth = (iWidth - 32) as i16 as i32;
                    self.spr.draw_src(x + 16, y + 32 + l * 32, &SDL_Rect { x: 0, y: if lineSel { 32 } else { 0 }, w: iHalfLineWidth, h: 32 });
                    self.spr.draw_src(
                        x + 16 + iHalfLineWidth,
                        y + 32 + l * 32,
                        &SDL_Rect { x: 512 - iLineWidth + iHalfLineWidth, y: if lineSel { 32 } else { 0 }, w: iLineWidth - iHalfLineWidth, h: 32 },
                    );
                }

                let item = &self.items[(self.iOffset as i32 + l) as usize];
                if item.fSelected {
                    rm.menu_map_filter.draw_src(x + 24, y + 32 + l * 32 + 4, &SDL_Rect { x: 24, y: 0, w: 24, h: 24 });
                }

                rm.menu_font_large.draw_chop_right(x + 52, y + 5 + l * 32 + 32, iWidth - 104, &item.sName);
                let icon = item.iIcon as i32;
                rm.spr_map_filter_icons.draw_src(x + 28, y + 32 + l * 32 + 8, &SDL_Rect { x: icon % 10 * 16, y: icon / 10 * 16, w: 16, h: 16 });
                iLine += 1;
            }
        }
    }
}
