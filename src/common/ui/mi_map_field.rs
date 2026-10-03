//! Port of src/common/ui/MI_MapField.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_map_preview::MI_MapPreview;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::SDL_Rect;

/// `maplist->GetCurrent()->second.iIndex`
fn current_map_index() -> i16 {
    unsafe {
        maplist.node_at(maplist.get_current()).iIndex
    }
}

pub struct MI_MapField {
    pub mi_map_preview: MI_MapPreview,

    pub szName: String,

    pub miModifyImageLeft: Box<MI_Image>,
    pub miModifyImageRight: Box<MI_Image>,

    pub iSlideListOutGoal: i16,

    pub sSearchString: String,
    pub iSearchStringTimer: i16,

    pub fShowtags: bool,
}
crate::impl_base!(MI_MapField => mi_map_preview: MI_MapPreview);

impl MI_MapField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16, showtags: bool) -> Self {
        let mi_map_preview = MI_MapPreview::new(nspr, x, y, width, indent);
        let (px, py, iWidth) = (mi_map_preview.m_pos.x, mi_map_preview.m_pos.y, mi_map_preview.iWidth);

        let mut miModifyImageLeft = Box::new(MI_Image::new(nspr, px + indent - 26, py + 4, 32, 64, 26, 24, 4, 1, 8));
        miModifyImageLeft.set_visible(false);

        let mut miModifyImageRight = Box::new(MI_Image::new(nspr, px + iWidth - 16, py + 4, 32, 88, 26, 24, 4, 1, 8));
        miModifyImageRight.set_visible(false);

        let mut this = MI_MapField {
            mi_map_preview,
            szName: name.into(),
            miModifyImageLeft,
            miModifyImageRight,
            iSlideListOutGoal: 0,
            sSearchString: String::new(),
            iSearchStringTimer: 0,
            fShowtags: showtags,
        };

        if this.fShowtags {
            this.iSlideListOut = ((this.iWidth as i32 - 352) >> 1) as i16;
            this.iSlideListOutGoal = this.iSlideListOut;
        } else {
            this.iSlideListOut = 0;
            this.iSlideListOutGoal = this.iSlideListOut;
        }

        this
    }

    pub fn choose_random_map(&mut self) -> MenuCodeEnum {
        let iOldIndex = current_map_index();
        unsafe { maplist.random(true) };

        if iOldIndex != current_map_index() {
            self.load_current_map();
            return MENU_CODE_MAP_CHANGED;
        }

        MENU_CODE_NONE
    }

    pub fn move_prev(&mut self, fScrollFast: bool) -> bool {
        self.move_(false, fScrollFast)
    }

    pub fn move_next(&mut self, fScrollFast: bool) -> bool {
        self.move_(true, fScrollFast)
    }

    pub fn set_dimensions(&mut self, width: i16, indent: i16) {
        self.mi_map_preview.set_dimensions(width, indent);

        let (px, py, iWidth) = (self.m_pos.x, self.m_pos.y, self.iWidth);
        self.miModifyImageLeft.set_position(px + indent - 26, py + 4);
        self.miModifyImageRight.set_position(px + iWidth - 16, py + 4);

        if self.fShowtags {
            //iSlideListOut = (iWidth - 352) >> 1;
            self.iSlideListOut = (((self.iWidth as f64 - App::screenWidth as f64 * 0.55) as i32) >> 1) as i16;
            self.iSlideListOutGoal = self.iSlideListOut;
        }
    }

    fn move_(&mut self, fNext: bool, fScrollFast: bool) -> bool {
        let mut numadvance: i32 = 1;
        if fScrollFast {
            numadvance = 10;
        }

        let iOldIndex = current_map_index();
        for _ in 0..numadvance {
            unsafe {
                if fNext {
                    maplist.next(true);
                } else {
                    maplist.prev(true);
                }
            }
        }

        if iOldIndex != current_map_index() {
            self.load_current_map();
            return true;
        }

        false
    }
}

impl UI_ControlTrait for MI_MapField {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_UNSELECT_ITEM;
        }

        self.miModifyImageLeft.set_visible(modify);
        self.miModifyImageRight.set_visible(modify);

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let iPressedKey: i16 = playerInput.iPressedKey as i16;

        for iPlayer in 0..4usize {
            let out = playerInput.outputControls[iPlayer];
            if out.menu_right().fPressed || out.menu_down().fPressed {
                if self.move_next(out.menu_scrollfast().fDown) {
                    return MENU_CODE_MAP_CHANGED;
                }

                return MENU_CODE_NONE;
            }

            if out.menu_left().fPressed || out.menu_up().fPressed {
                if self.move_prev(out.menu_scrollfast().fDown) {
                    return MENU_CODE_MAP_CHANGED;
                }

                return MENU_CODE_NONE;
            }

            if out.menu_random().fPressed {
                return self.choose_random_map();
            }

            if out.menu_select().fPressed || out.menu_cancel().fPressed {
                self.miModifyImageLeft.set_visible(false);
                self.miModifyImageRight.set_visible(false);

                self.fModifying = false;
                return MENU_CODE_UNSELECT_ITEM;
            }

            if iPlayer == 0 && iPressedKey > 0 {
                let key = iPressedKey as i32;
                if (key >= SDLK_a as i32 && key <= SDLK_z as i32)
                    || (key >= SDLK_0 as i32 && key <= SDLK_9 as i32)
                    || key == SDLK_MINUS as i32
                    || key == SDLK_EQUALS as i32
                {
                    let iOldIndex = current_map_index();

                    //maplist->startswith((char)playerInput->iPressedKey);

                    self.sSearchString.push(iPressedKey as u8 as char);
                    self.iSearchStringTimer = 10;

                    if unsafe { !maplist.startswith(&self.sSearchString) } {
                        self.sSearchString = String::new();
                        self.iSearchStringTimer = 0;
                    }

                    if iOldIndex != current_map_index() {
                        self.load_current_map();
                        return MENU_CODE_MAP_CHANGED;
                    }

                    return MENU_CODE_NONE;
                }
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self) {
        //Empty out the search string after a certain time
        if self.iSearchStringTimer > 0 {
            self.iSearchStringTimer -= 1;
            if self.iSearchStringTimer == 0 {
                self.sSearchString = String::new();
            }
        }

        if self.iSlideListOut != self.iSlideListOutGoal {
            if self.iSlideListOutGoal > self.iSlideListOut {
                self.iSlideListOut += 4;

                if self.iSlideListOut > self.iSlideListOutGoal {
                    self.iSlideListOut = self.iSlideListOutGoal;
                }
            } else if self.iSlideListOutGoal < self.iSlideListOut {
                self.iSlideListOut -= 4;

                if self.iSlideListOut < self.iSlideListOutGoal {
                    self.iSlideListOut = self.iSlideListOutGoal;
                }
            }
        }

        //Update hazards
        self.mi_map_preview.update_impl();

        self.miModifyImageRight.update();
        self.miModifyImageLeft.update();
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.iWidth as i32;
        let indent = self.iIndent as i32;
        let selY = if self.fSelected { 32 } else { 0 };

        unsafe {
            //Draw the select field background
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: selY, w: indent - 16, h: 32 });
            self.spr.draw_src(x + indent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            self.spr.draw_src(x + indent + 16, y, &SDL_Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

            rm.menu_font_large.draw_chop_right(x + 16, y + 5, indent - 8, &self.szName);
            rm.menu_font_large.draw_chop_right(x + indent + 8, y + 5, width - indent - 24, &self.szMapName);
        }

        self.mi_map_preview.draw_impl();

        self.miModifyImageLeft.draw();
        self.miModifyImageRight.draw();

        //rm->menu_font_large.draw(rectDst.x, rectDst.y, sSearchString.c_str());
    }

    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_NONE;
        }

        let (mx, my) = (iMouseX as i32, iMouseY as i32);

        //If we are modifying this control, see if we clicked on a next/prev button
        if self.fModifying {
            let (mut x, mut y, mut w, mut h) = (0i16, 0i16, 0i16, 0i16);
            self.miModifyImageLeft.get_position_and_size(&mut x, &mut y, &mut w, &mut h);

            if mx >= x as i32 && mx < x as i32 + w as i32 && my >= y as i32 && my < y as i32 + h as i32 && self.move_prev(false) {
                return MENU_CODE_MAP_CHANGED;
            }

            self.miModifyImageRight.get_position_and_size(&mut x, &mut y, &mut w, &mut h);

            if mx >= x as i32 && mx < x as i32 + w as i32 && my >= y as i32 && my < y as i32 + h as i32 && self.move_next(false) {
                return MENU_CODE_MAP_CHANGED;
            }
        }

        //Otherwise just check to see if we clicked on the whole control
        if mx >= self.m_pos.x as i32 && mx < self.m_pos.x as i32 + self.iWidth as i32 && my >= self.m_pos.y as i32 && my < self.m_pos.y as i32 + 32 {
            return MENU_CODE_CLICKED;
        }

        //Otherwise this control wasn't clicked at all
        MENU_CODE_NONE
    }
}
