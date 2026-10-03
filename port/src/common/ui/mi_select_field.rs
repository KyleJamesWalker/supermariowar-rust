//! Port of src/common/ui/MI_SelectField.cpp
//!
//! The C++ class template becomes a generic struct. Its virtual bodies are inherent `*_impl` methods so
//! subclasses (MI_ImageSelectField, MI_SliderField, ...) can call `MI_SelectField<T>::Draw()` and friends.

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

#[derive(Clone, Debug)]
pub struct SF_ListItem<T: Copy> {
    pub name: String, // Display name
    pub value: T,
    pub hidden: bool,
    pub iconOverride: i16,
}

impl<T: Copy> SF_ListItem<T> {
    pub fn new(name: String, value: T) -> Self {
        SF_ListItem { name, value, hidden: false, iconOverride: -1 }
    }
}

pub struct MI_SelectField<T: Copy + PartialEq + 'static> {
    pub ui_control: UI_Control,

    pub m_spr: Ptr<gfxSprite>,
    pub m_name: String,

    pub miModifyImageLeft: Box<MI_Image>,
    pub miModifyImageRight: Box<MI_Image>,

    pub mcItemChangedCode: MenuCodeEnum,
    pub mcControlSelectedCode: MenuCodeEnum,

    pub m_autoAdvance: bool,
    pub m_wraps: bool,
    pub m_fastScroll: bool,
    pub m_adjustmentY: i16,
    pub m_width: i16,
    pub m_indent: i16,

    pub m_items: Vec<SF_ListItem<T>>,
    pub m_goodRandomIndices: Vec<usize>,
    pub m_index: usize,
    pub m_outputPtr: *mut T,
}

impl<T: Copy + PartialEq + 'static> std::ops::Deref for MI_SelectField<T> {
    type Target = UI_Control;
    fn deref(&self) -> &UI_Control {
        &self.ui_control
    }
}
impl<T: Copy + PartialEq + 'static> std::ops::DerefMut for MI_SelectField<T> {
    fn deref_mut(&mut self) -> &mut UI_Control {
        &mut self.ui_control
    }
}

fn modify_images(spr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, indent: i16) -> (Box<MI_Image>, Box<MI_Image>) {
    let mut left = Box::new(MI_Image::new(spr, x + indent - 26, y + 4, 32, 64, 26, 24, 4, 1, 8));
    left.set_visible(false);

    let mut right = Box::new(MI_Image::new(spr, x + width - 16, y + 4, 32, 88, 26, 24, 4, 1, 8));
    right.set_visible(false);
    (left, right)
}

impl<T: Copy + PartialEq + 'static> MI_SelectField<T> {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16) -> Self {
        let (left, right) = modify_images(nspr, x, y, width, indent);
        MI_SelectField {
            ui_control: UI_Control::new(x, y),
            m_spr: nspr,
            m_name: name.into(),
            miModifyImageLeft: left,
            miModifyImageRight: right,
            mcItemChangedCode: MENU_CODE_NONE,
            mcControlSelectedCode: MENU_CODE_NONE,
            m_autoAdvance: false,
            m_wraps: true,
            m_fastScroll: false,
            m_adjustmentY: if width > 256 { 0 } else { 128 },
            m_width: width,
            m_indent: indent,
            m_items: Vec::new(),
            m_goodRandomIndices: Vec::new(),
            m_index: 0,
            m_outputPtr: std::ptr::null_mut(),
        }
    }

    /// Copy constructor.
    pub fn copy_from(other: &MI_SelectField<T>) -> Self {
        let ui_control = UI_Control::copy_from(&other.ui_control);
        let (left, right) = modify_images(other.m_spr, ui_control.m_pos.x, ui_control.m_pos.y, other.m_width, other.m_indent);
        let mut this = MI_SelectField {
            ui_control,
            m_spr: other.m_spr,
            m_name: other.m_name.clone(),
            miModifyImageLeft: left,
            miModifyImageRight: right,
            mcItemChangedCode: other.mcItemChangedCode,
            mcControlSelectedCode: other.mcControlSelectedCode,
            m_autoAdvance: other.m_autoAdvance,
            m_wraps: other.m_wraps,
            m_fastScroll: other.m_fastScroll,
            m_adjustmentY: other.m_adjustmentY,
            m_width: other.m_width,
            m_indent: other.m_indent,
            m_items: other.m_items.clone(),
            m_goodRandomIndices: other.m_goodRandomIndices.clone(),
            m_index: 0,
            m_outputPtr: other.m_outputPtr,
        };
        this.set_current_index(other.m_index);
        this
    }

    pub fn set_title(&mut self, name: impl Into<String>) {
        self.m_name = name.into();
    }
    pub fn set_auto_advance(&mut self, advance: bool) {
        self.m_autoAdvance = advance;
    }
    pub fn allow_wrap(&mut self, wraps: bool) {
        self.m_wraps = wraps;
    }
    pub fn set_item_changed_code(&mut self, code: MenuCodeEnum) {
        self.mcItemChangedCode = code;
    }
    pub fn set_control_selected_code(&mut self, code: MenuCodeEnum) {
        self.mcControlSelectedCode = code;
    }
    pub fn allow_fast_scroll(&mut self, fastscroll: bool) {
        self.m_fastScroll = fastscroll;
    }
    pub fn set_output_ptr(&mut self, ptr: *mut T) {
        self.m_outputPtr = ptr;
    }

    pub fn current_item(&self) -> &SF_ListItem<T> {
        &self.m_items[self.m_index]
    }
    pub fn current_value(&self) -> T {
        self.current_item().value
    }
    /// Does not change the current item.
    pub fn random_value(&self) -> T {
        let idx = self.m_goodRandomIndices[RANDOM_INT(self.m_goodRandomIndices.len() as i32) as usize];
        self.m_items[idx].value
    }

    pub fn set_current_value(&mut self, value: T) -> bool {
        let prev_idx = self.m_index;

        self.m_index = 0;
        while self.m_index < self.m_items.len() {
            if self.m_items[self.m_index].value == value {
                self.update_output();
                return true;
            }
            self.m_index += 1;
        }

        self.m_index = prev_idx;
        false
    }

    pub fn set_current_index(&mut self, index: usize) -> bool {
        if index < self.m_items.len() {
            self.m_index = index;
            self.update_output();
            return true;
        }
        false
    }

    pub fn add(&mut self, name: impl Into<String>, value: T) -> &mut SF_ListItem<T> {
        self.add_random(name, value, true)
    }

    pub fn add_random(&mut self, name: impl Into<String>, value: T, goodRandom: bool) -> &mut SF_ListItem<T> {
        let new_idx = self.m_items.len();
        self.m_items.push(SF_ListItem::new(name.into(), value));
        self.m_index = 0;

        if goodRandom {
            self.m_goodRandomIndices.push(new_idx);
        }

        self.m_items.last_mut().unwrap()
    }

    pub fn clear(&mut self) {
        self.m_items.clear();
        self.m_goodRandomIndices.clear();
    }

    pub fn hide_item(&mut self, value: T, hide: bool) {
        for item in self.m_items.iter_mut() {
            if item.value == value {
                item.hidden = hide;
            }
        }
        if self.current_item().hidden {
            let _ = self.move_next() || self.move_prev();
        }
    }

    pub fn hide_all_items(&mut self, hide: bool) {
        for item in self.m_items.iter_mut() {
            item.hidden = hide;
        }
    }

    pub fn update_output(&self) {
        if !self.m_outputPtr.is_null() {
            unsafe { *self.m_outputPtr = self.current_value() };
        }
    }

    pub fn move_next(&mut self) -> bool {
        if self.m_items.is_empty() {
            return false;
        }

        // stop when detecting a restart or a loop
        let stop_idx = if self.m_wraps { self.m_index } else { 0 };

        loop {
            self.m_index = (self.m_index + 1) % self.m_items.len();
            if self.m_index == stop_idx {
                break;
            }
            if !self.current_item().hidden {
                self.update_output();
                return true;
            }
        }

        false
    }

    pub fn move_prev(&mut self) -> bool {
        if self.m_items.is_empty() {
            return false;
        }

        // stop when detecting a restart or a loop
        let stop_idx = if self.m_wraps { self.m_index } else { self.m_items.len() - 1 };

        loop {
            self.m_index = (self.m_index + self.m_items.len() - 1) % self.m_items.len();
            if self.m_index == stop_idx {
                break;
            }
            if !self.current_item().hidden {
                self.update_output();
                return true;
            }
        }

        false
    }

    pub fn move_random(&mut self) -> bool {
        let mut valid_indices: Vec<usize> = Vec::with_capacity(self.m_items.len());

        for idx in 0..self.m_items.len() {
            if !self.m_items[idx].hidden && idx != self.m_index {
                valid_indices.push(idx);
            }
        }
        if valid_indices.len() <= 1 {
            return false;
        }

        self.m_index = valid_indices[RANDOM_INT(valid_indices.len() as i32) as usize];
        self.update_output();
        true
    }

    pub fn update_impl(&mut self) {
        self.miModifyImageRight.update();
        self.miModifyImageLeft.update();
    }

    pub fn draw_impl(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.m_width as i32;
        let indent = self.m_indent as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.m_adjustmentY as i32;

        unsafe {
            if self.m_indent == 0 {
                let iHalfWidth = (self.m_width / 2) as i32;
                self.m_spr.draw_src(x, y, &SDL_Rect { x: 0, y: selY, w: iHalfWidth, h: 32 });
                self.m_spr.draw_src(x + iHalfWidth, y, &SDL_Rect { x: 512 - iHalfWidth, y: selY, w: width - iHalfWidth, h: 32 });
            } else {
                self.m_spr.draw_src(x, y, &SDL_Rect { x: 0, y: selY, w: indent - 16, h: 32 });
                self.m_spr.draw_src(x + indent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
                self.m_spr.draw_src(x + indent + 16, y, &SDL_Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });
            }

            if self.m_indent > 0 {
                rm.menu_font_large.draw_chop_right(x + 16, y + 5, indent - 8, &self.m_name);
            }

            if !self.m_items.is_empty() {
                let indent = if self.m_indent > 0 { indent } else { 8 };
                rm.menu_font_large.draw_chop_right(x + indent + 8, y + 5, width - indent - 24, &self.current_item().name);
            }
        }

        let drawLeft = self.m_index > 0;
        if self.m_wraps || drawLeft {
            self.miModifyImageLeft.draw();
        }

        let drawRight = (self.m_index + 1) < self.m_items.len();
        if self.m_wraps || drawRight {
            self.miModifyImageRight.draw();
        }
    }

    pub fn modify_impl(&mut self, modify: bool) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_UNSELECT_ITEM;
        }

        if self.m_autoAdvance && modify {
            self.move_next();
            return self.mcItemChangedCode;
        }

        if MENU_CODE_NONE != self.mcControlSelectedCode {
            return self.mcControlSelectedCode;
        }

        self.miModifyImageLeft.set_visible(modify);
        self.miModifyImageRight.set_visible(modify);
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    pub fn send_input_impl(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        for iPlayer in 0..4usize {
            let out = playerInput.outputControls[iPlayer];
            let mut iNumMoves: i16 = 1;
            if self.m_fastScroll && out.menu_scrollfast().fDown {
                iNumMoves = 10;
            }

            if out.menu_right().fPressed || out.menu_down().fPressed {
                let mut fMoved = false;

                for _ in 0..iNumMoves {
                    fMoved |= self.move_next();
                }

                if fMoved {
                    return self.mcItemChangedCode;
                }
            }

            if out.menu_left().fPressed || out.menu_up().fPressed {
                let mut fMoved = false;

                for _ in 0..iNumMoves {
                    fMoved |= self.move_prev();
                }

                if fMoved {
                    return self.mcItemChangedCode;
                }
            }

            if out.menu_random().fPressed && self.move_random() {
                return self.mcItemChangedCode;
            }

            if out.menu_select().fPressed || out.menu_cancel().fPressed {
                self.miModifyImageLeft.set_visible(false);
                self.miModifyImageRight.set_visible(false);

                self.fModifying = false;

                self.update_output();

                return MENU_CODE_UNSELECT_ITEM;
            }
        }

        MENU_CODE_NONE
    }

    pub fn mouse_click_impl(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_NONE;
        }

        let (mx, my) = (iMouseX as i32, iMouseY as i32);
        let inside = |x: i16, y: i16, w: i16, h: i16| x as i32 <= mx && mx < x as i32 + w as i32 && y as i32 <= my && my < y as i32 + h as i32;

        //If we are modifying this control, see if we clicked on a next/prev button
        if self.fModifying {
            let (mut x, mut y, mut w, mut h) = (0, 0, 0, 0);

            self.miModifyImageLeft.get_position_and_size(&mut x, &mut y, &mut w, &mut h);
            if inside(x, y, w, h) && self.move_prev() {
                return if self.mcItemChangedCode == MENU_CODE_NONE { MENU_CODE_CLICKED } else { self.mcItemChangedCode };
            }

            self.miModifyImageRight.get_position_and_size(&mut x, &mut y, &mut w, &mut h);
            if inside(x, y, w, h) && self.move_next() {
                return if self.mcItemChangedCode == MENU_CODE_NONE { MENU_CODE_CLICKED } else { self.mcItemChangedCode };
            }
        }

        //Otherwise just check to see if we clicked on the whole control
        if inside(self.m_pos.x, self.m_pos.y, self.m_width, 32) {
            return MENU_CODE_CLICKED;
        }

        //Otherwise this control wasn't clicked at all
        MENU_CODE_NONE
    }

    pub fn refresh_impl(&mut self) {
        if !self.m_outputPtr.is_null() {
            let v = unsafe { *self.m_outputPtr };
            self.set_current_value(v);
        }
    }
}

impl<T: Copy + PartialEq + 'static> UI_ControlTrait for MI_SelectField<T> {
    crate::impl_ctl!();

    fn update(&mut self) {
        self.update_impl();
    }
    fn draw(&mut self) {
        self.draw_impl();
    }
    fn refresh(&mut self) {
        self.refresh_impl();
    }
    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.modify_impl(modify)
    }
    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        self.send_input_impl(playerInput)
    }
    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        self.mouse_click_impl(iMouseX, iMouseY)
    }
}
