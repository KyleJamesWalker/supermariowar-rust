//! Port of src/smw/ui/MI_MapBrowser.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
use crate::common::input::CPlayerInput;
use crate::common::map::read_type_preview;
use crate::common::path::{convert_path, file_exists, get_name_from_file_name};
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::gs_gameplay::lookup_team_id;
use sdl2::sys::{SDL_Delay, SDL_Rect};

fn small_delay() {
    unsafe { SDL_Delay(10) };
}

pub struct MI_MapBrowser {
    pub ui_control: UI_Control,

    iPage: i16,
    iSelectedCol: i16,
    iSelectedRow: i16,
    iSelectedIndex: i16,

    mapSurfaces: [gfxSprite; 9],
    mapNames: [String; 9],
    /// Also stands in for `mapListNodes[i]`, which is `&(*mapListItr[i]).second`.
    mapListItr: [usize; 9],

    iFilterTagAnimationTimer: i16,
    iFilterTagAnimationFrame: i16,

    iType: i16,
    iMapCount: i16,

    srcRectBackground: SDL_Rect,
    dstRectBackground: SDL_Rect,
}
crate::impl_base!(MI_MapBrowser => ui_control: UI_Control);

impl MI_MapBrowser {
    pub fn new() -> Self {
        MI_MapBrowser {
            ui_control: UI_Control::new(0, 0),
            iPage: 0,
            iSelectedCol: 0,
            iSelectedRow: 0,
            iSelectedIndex: 0,
            mapSurfaces: Default::default(),
            mapNames: Default::default(),
            mapListItr: [0; 9],
            iFilterTagAnimationTimer: 0,
            iFilterTagAnimationFrame: 0,
            iType: 0,
            iMapCount: 0,
            srcRectBackground: SDL_Rect { x: 0, y: 0, w: App::screenWidth, h: App::screenHeight },
            dstRectBackground: SDL_Rect { x: 0, y: 0, w: 160, h: 120 },
        }
    }

    pub fn reset(&mut self, type_: i16) {
        self.iType = type_;

        unsafe {
            if self.iType == 0 {
                self.iSelectedIndex = maplist.node_at(maplist.get_current()).iIndex;

                self.iFilterTagAnimationTimer = 0;
                self.iFilterTagAnimationFrame = 72;

                self.iMapCount = maplist.count() as i16;
            } else {
                self.iSelectedIndex = maplist.node_at(maplist.get_current()).iFilteredIndex;
                self.iMapCount = maplist.filtered_count() as i16;
            }
        }

        self.iSelectedRow = (self.iSelectedIndex / 3) % 3;
        self.iSelectedCol = self.iSelectedIndex % 3;
        self.iPage = self.iSelectedIndex / 9;

        self.load_page(self.iPage, self.iType == 1);
    }

    fn load_page(&mut self, page: i16, fUseFilters: bool) {
        unsafe {
            for iMap in 0..9i16 {
                let iIndex: i16 = (iMap as i32 + page as i32 * 9) as i16;

                if iIndex >= self.iMapCount {
                    return;
                }

                let itr = maplist.get_iterator_at(iIndex as u16, fUseFilters);

                //See if we already have a thumbnail saved for this map

                let mut szThumbnail = String::from("maps/cache/");
                szThumbnail += &get_name_from_file_name(&maplist.node_at(itr).filename, false);
                szThumbnail += ".png";

                let sConvertedPath = convert_path(&szThumbnail);

                if !file_exists(&sConvertedPath) {
                    let filename = maplist.node_at(itr).filename.clone();
                    g_map.load_map(&filename, read_type_preview);
                    small_delay(); //Sleeps to help the music from skipping
                    g_map.save_thumbnail(&sConvertedPath, false);
                    small_delay();
                }

                let m = iMap as usize;
                self.mapSurfaces[m] = ImageLoader::new(&sConvertedPath).without_color_key().create();

                self.mapNames[m] = maplist.key_at(itr).to_string();
                self.mapListItr[m] = itr;
            }
        }
    }

    /// `mapListNodes[i]->pfFilters[game_values.selectedmapfilter]`
    fn node_filter(&self, i: usize) -> bool {
        unsafe { maplist.node_at(self.mapListItr[i]).pfFilters[game_values.selectedmapfilter as usize] }
    }
}

impl Default for MI_MapBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl UI_ControlTrait for MI_MapBrowser {
    crate::impl_ctl!();

    fn update(&mut self) {
        self.iFilterTagAnimationTimer += 1;
        if self.iFilterTagAnimationTimer > 8 {
            self.iFilterTagAnimationTimer = 0;

            self.iFilterTagAnimationFrame += 24;
            if self.iFilterTagAnimationFrame > 24 {
                self.iFilterTagAnimationFrame = 0;
            }
        }
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let rSrc = SDL_Rect { x: 0, y: 0, w: 160, h: 120 };
        let mut rDst = SDL_Rect { x: 0, y: 0, w: 160, h: 120 };
        let filterSrc = SDL_Rect { x: self.iFilterTagAnimationFrame as i32, y: 24, w: 24, h: 24 };

        unsafe {
            for iRow in 0..3i16 {
                rDst.y = iRow as i32 * 150 + 30;

                for iCol in 0..3i16 {
                    if self.iSelectedCol != iCol || self.iSelectedRow != iRow {
                        if iRow as i32 * 3 + iCol as i32 + self.iPage as i32 * 9 >= self.iMapCount as i32 {
                            break;
                        }

                        rDst.x = iCol as i32 * 200 + 40;

                        let i = (iRow * 3 + iCol) as usize;
                        self.mapSurfaces[i].draw_src_to(&rSrc, blitdest, &rDst);

                        if self.iType == 0 && self.node_filter(i) {
                            rm.menu_map_filter.draw_src(rDst.x, rDst.y, &filterSrc);
                        }

                        rm.menu_font_large.draw_chop_right(rDst.x, rDst.y + 120, 165, &self.mapNames[i]);
                    }
                }
            }

            //Draw the selected map
            rDst.y = self.iSelectedRow as i32 * 150 + 30;
            rDst.x = self.iSelectedCol as i32 * 200 + 40;

            rm.menu_dialog.draw_src(rDst.x - 16, rDst.y - 16, &SDL_Rect { x: 0, y: 0, w: 176, h: 148 });
            rm.menu_dialog.draw_src(rDst.x + 160, rDst.y - 16, &SDL_Rect { x: 496, y: 0, w: 16, h: 148 });
            rm.menu_dialog.draw_src(rDst.x - 16, rDst.y + 132, &SDL_Rect { x: 0, y: 464, w: 176, h: 16 });
            rm.menu_dialog.draw_src(rDst.x + 160, rDst.y + 132, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });

            let sel = (self.iSelectedRow * 3 + self.iSelectedCol) as usize;
            self.mapSurfaces[sel].draw_src_to(&rSrc, blitdest, &rDst);

            if self.iType == 0 && self.node_filter(sel) {
                rm.menu_map_filter.draw_src(rDst.x, rDst.y, &filterSrc);
            }

            rm.menu_font_large.draw_chop_right(rDst.x, rDst.y + 120, 165, &self.mapNames[sel]);
        }
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            for iPlayer in 0..4i16 {
                //Only allow the controlling team to control the menu (if there is one)
                if self.iControllingTeam != -1
                    && (self.iControllingTeam != lookup_team_id(iPlayer) || game_values.playercontrol[iPlayer as usize] != 1)
                {
                    continue;
                }

                let out = playerInput.outputControls[iPlayer as usize];

                if out.menu_down().fPressed {
                    let mut iSkipRows: i16 = 1;
                    if out.menu_scrollfast().fDown {
                        iSkipRows = 3;
                    }

                    self.iSelectedIndex = ((self.iSelectedRow as i32 + iSkipRows as i32) * 3 + self.iSelectedCol as i32 + self.iPage as i32 * 9) as i16;

                    if self.iSelectedIndex >= self.iMapCount {
                        self.iSelectedIndex = self.iMapCount - 1;
                    }

                    self.iSelectedRow = (self.iSelectedIndex / 3) % 3;
                    self.iSelectedCol = self.iSelectedIndex % 3;

                    let iOldPage = self.iPage;
                    self.iPage = self.iSelectedIndex / 9;

                    if iOldPage != self.iPage {
                        self.load_page(self.iPage, self.iType == 1);
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_up().fPressed {
                    let mut iSkipRows: i16 = 1;
                    if out.menu_scrollfast().fDown {
                        iSkipRows = 3;
                    }

                    self.iSelectedIndex = ((self.iSelectedRow as i32 - iSkipRows as i32) * 3 + self.iSelectedCol as i32 + self.iPage as i32 * 9) as i16;

                    if self.iSelectedIndex < 0 {
                        self.iSelectedIndex = 0;
                    }

                    self.iSelectedRow = (self.iSelectedIndex / 3) % 3;
                    self.iSelectedCol = self.iSelectedIndex % 3;

                    let iOldPage = self.iPage;
                    self.iPage = self.iSelectedIndex / 9;

                    if iOldPage != self.iPage {
                        self.load_page(self.iPage, self.iType == 1);
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_left().fPressed {
                    let iNextIndex: i16 = (self.iSelectedRow as i32 * 3 + self.iSelectedCol as i32 - 1 + self.iPage as i32 * 9) as i16;

                    if iNextIndex < 0 {
                        return MENU_CODE_NONE;
                    }

                    self.iSelectedIndex = iNextIndex;

                    self.iSelectedCol -= 1;
                    if self.iSelectedCol < 0 {
                        self.iSelectedCol = 2;

                        self.iSelectedRow -= 1;
                        if self.iSelectedRow < 0 {
                            self.iSelectedRow = 2;
                            self.iPage -= 1;
                            self.load_page(self.iPage, self.iType == 1);
                        }
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_right().fPressed {
                    let iNextIndex: i16 = (self.iSelectedRow as i32 * 3 + self.iSelectedCol as i32 + 1 + self.iPage as i32 * 9) as i16;

                    if iNextIndex >= self.iMapCount {
                        return MENU_CODE_NONE;
                    }

                    self.iSelectedIndex = iNextIndex;

                    self.iSelectedCol += 1;
                    if self.iSelectedCol > 2 {
                        self.iSelectedCol = 0;

                        self.iSelectedRow += 1;
                        if self.iSelectedRow > 2 {
                            self.iSelectedRow = 0;
                            self.iPage += 1;
                            self.load_page(self.iPage, self.iType == 1);
                        }
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_select().fPressed {
                    let itr = self.mapListItr[(self.iSelectedRow * 3 + self.iSelectedCol) as usize];
                    if self.iType == 0 {
                        let f = &mut maplist.maps.node_mut(itr).pfFilters[game_values.selectedmapfilter as usize];
                        *f = !*f;
                        game_values.fNeedWriteFilters = true;
                    } else {
                        maplist.set_current(itr);
                        return MENU_CODE_MAP_BROWSER_EXIT;
                    }

                    return MENU_CODE_NONE;
                }

                if out.menu_cancel().fPressed {
                    return MENU_CODE_MAP_BROWSER_EXIT;
                }
            }
        }

        MENU_CODE_NONE
    }
}
