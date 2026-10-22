//! The map picker menu item: a map preview with arrows, random pick and type-to-search over the filtered maps.

use crate::common::global_constants::{ticks, vel};
use crate::common::resource_manager::SpriteId;
use crate::common::keys::*;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_map_preview::MI_MapPreview;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;

/// The selected map's index in the whole map list.
fn current_map_index(g: &mut Game) -> i16 {
    g.maplist.node_at(g.maplist.get_current()).iIndex
}

/// Picks the map from `Game::maplist`'s filtered set and shows its preview.
pub struct MI_MapField {
    pub mi_map_preview: MI_MapPreview,

    pub szName: String,

    pub miModifyImageLeft: Box<MI_Image>,
    pub miModifyImageRight: Box<MI_Image>,

    /// The offset `iSlideListOut` slides toward, 4 pixels per 62.5 Hz tick.
    pub iSlideListOutGoal: i16,
    pub dSlideListRemainder: f32,

    /// Letters typed to jump to a map by name; cleared `iSearchStringTimer` ticks after the last one.
    pub sSearchString: String,
    pub iSearchStringTimer: i16,

    /// Leaves room beside the preview for the map's filter tags.
    pub fShowtags: bool,
    /// Ticks a direction has been held; past `HOLD_DELAY` it repeats every `HOLD_STEP`.
    pub iHoldTicks: i16,
}

const HOLD_DELAY: i16 = ticks(25) as i16;
const HOLD_STEP: i16 = ticks(6) as i16;
crate::impl_base!(MI_MapField => mi_map_preview: MI_MapPreview);

impl MI_MapField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16, showtags: bool) -> Self {
        let mut gp = g.ptr();
        let mi_map_preview = MI_MapPreview::new(gp.get(), nspr, x, y, width, indent);
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
            dSlideListRemainder: 0.0,
            sSearchString: String::new(),
            iSearchStringTimer: 0,
            fShowtags: showtags,
            iHoldTicks: 0,
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

    /// Moves to a random filtered map; `MENU_CODE_MAP_CHANGED` if it changed.
    pub fn choose_random_map(&mut self, g: &mut Game) -> MenuCodeEnum {
        let mut gp = g.ptr();
        let iOldIndex = current_map_index(g);
        gp.maplist.random(g, true);

        if iOldIndex != current_map_index(g) {
            self.load_current_map(g);
            return MENU_CODE_MAP_CHANGED;
        }

        MENU_CODE_NONE
    }

    /// Moves to the previous filtered map, or ten back with `fScrollFast`; true if the map changed.
    pub fn move_prev(&mut self, g: &mut Game, fScrollFast: bool) -> bool {
        self.move_(g, false, fScrollFast)
    }

    /// Moves to the next filtered map, or ten on with `fScrollFast`; true if the map changed.
    pub fn move_next(&mut self, g: &mut Game, fScrollFast: bool) -> bool {
        self.move_(g, true, fScrollFast)
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

    fn move_(&mut self, g: &mut Game, fNext: bool, fScrollFast: bool) -> bool {
        let mut numadvance: i32 = 1;
        if fScrollFast {
            numadvance = 10;
        }

        let iOldIndex = current_map_index(g);
        for _ in 0..numadvance {
            if fNext {
                g.maplist.next(true);
            } else {
                g.maplist.prev(true);
            }
        }

        if iOldIndex != current_map_index(g) {
            self.load_current_map(g);
            return true;
        }

        false
    }
}

impl UI_ControlTrait for MI_MapField {
    crate::impl_ctl!();

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_UNSELECT_ITEM;
        }

        self.miModifyImageLeft.set_visible(modify);
        self.miModifyImageRight.set_visible(modify);

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let iPressedKey: i16 = playerInput.iPressedKey as i16;

        for iPlayer in 0..4usize {
            let out = playerInput.outputControls[iPlayer];
            let next = out.menu_right().fPressed || out.menu_down().fPressed;
            let prev = out.menu_left().fPressed || out.menu_up().fPressed;
            let next_held = out.menu_right().fDown || out.menu_down().fDown;
            let prev_held = out.menu_left().fDown || out.menu_up().fDown;
            if next || prev || next_held || prev_held {
                if next || prev {
                    self.iHoldTicks = 0;
                } else {
                    self.iHoldTicks += 1;
                    if self.iHoldTicks < HOLD_DELAY {
                        return MENU_CODE_NONE;
                    }
                    self.iHoldTicks = HOLD_DELAY - HOLD_STEP;
                }

                let fast = out.menu_scrollfast().fDown;
                let moved = if next || (!prev && next_held) { self.move_next(g, fast) } else { self.move_prev(g, fast) };
                return if moved { MENU_CODE_MAP_CHANGED } else { MENU_CODE_NONE };
            }

            if out.menu_random().fPressed {
                return self.choose_random_map(g);
            }

            if out.menu_select().fPressed || out.menu_cancel().fPressed {
                self.miModifyImageLeft.set_visible(false);
                self.miModifyImageRight.set_visible(false);

                self.fModifying = false;
                return MENU_CODE_UNSELECT_ITEM;
            }

            if iPlayer == 0 && iPressedKey > 0 {
                let key = iPressedKey as i32;
                if (key >= SDLK_a && key <= SDLK_z)
                    || (key >= SDLK_0 && key <= SDLK_9)
                    || key == SDLK_MINUS
                    || key == SDLK_EQUALS
                {
                    let iOldIndex = current_map_index(g);

                    //maplist->startswith((char)playerInput->iPressedKey);

                    self.sSearchString.push(iPressedKey as u8 as char);
                    self.iSearchStringTimer = ticks(10) as i16;

                    if !g.maplist.startswith(&self.sSearchString) {
                        self.sSearchString = String::new();
                        self.iSearchStringTimer = 0;
                    }

                    if iOldIndex != current_map_index(g) {
                        self.load_current_map(g);
                        return MENU_CODE_MAP_CHANGED;
                    }

                    return MENU_CODE_NONE;
                }
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self, g: &mut Game) {
        //Empty out the search string after a certain time
        if self.iSearchStringTimer > 0 {
            self.iSearchStringTimer -= 1;
            if self.iSearchStringTimer == 0 {
                self.sSearchString = String::new();
            }
        }

        if self.iSlideListOut != self.iSlideListOutGoal {
            self.dSlideListRemainder += vel(4.0);
            let iStep = self.dSlideListRemainder as i16;
            self.dSlideListRemainder -= iStep as f32;

            if self.iSlideListOutGoal > self.iSlideListOut {
                self.iSlideListOut += iStep;

                if self.iSlideListOut > self.iSlideListOutGoal {
                    self.iSlideListOut = self.iSlideListOutGoal;
                }
            } else if self.iSlideListOutGoal < self.iSlideListOut {
                self.iSlideListOut -= iStep;

                if self.iSlideListOut < self.iSlideListOutGoal {
                    self.iSlideListOut = self.iSlideListOutGoal;
                }
            }
        }

        //Update hazards
        self.mi_map_preview.update_impl(g);

        self.miModifyImageRight.update(g);
        self.miModifyImageLeft.update(g);
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
        let selY = if self.fSelected { 32 } else { 0 };

        //Draw the select field background
        g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: 0, y: selY, w: indent - 16, h: 32 });
        g.rm.sprite(self.spr).draw_src(g, x + indent - 16, y, &Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
        g.rm.sprite(self.spr).draw_src(g, x + indent + 16, y, &Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

        gp.rm.menu_font_large.draw_chop_right(g, x + 16, y + 5, indent - 8, &self.szName);
        gp.rm.menu_font_large.draw_chop_right(g, x + indent + 8, y + 5, width - indent - 24, &self.szMapName);

        self.mi_map_preview.draw_impl(g);

        self.miModifyImageLeft.draw(g);
        self.miModifyImageRight.draw(g);

        //rm->menu_font_large.draw(rectDst.x, rectDst.y, sSearchString.c_str());
    }

    fn mouse_click(&mut self, g: &mut Game, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_NONE;
        }

        let (mx, my) = (iMouseX as i32, iMouseY as i32);

        //If we are modifying this control, see if we clicked on a next/prev button
        if self.fModifying {
            let (mut x, mut y, mut w, mut h) = (0i16, 0i16, 0i16, 0i16);
            self.miModifyImageLeft.get_position_and_size(&mut x, &mut y, &mut w, &mut h);

            if mx >= x as i32 && mx < x as i32 + w as i32 && my >= y as i32 && my < y as i32 + h as i32 && self.move_prev(g, false) {
                return MENU_CODE_MAP_CHANGED;
            }

            self.miModifyImageRight.get_position_and_size(&mut x, &mut y, &mut w, &mut h);

            if mx >= x as i32 && mx < x as i32 + w as i32 && my >= y as i32 && my < y as i32 + h as i32 && self.move_next(g, false) {
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
