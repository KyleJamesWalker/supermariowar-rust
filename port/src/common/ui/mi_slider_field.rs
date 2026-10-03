//! Port of src/common/ui/MI_SliderField.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_SliderField {
    pub mi_select_field: MI_SelectField<i16>,

    pub m_sprSlider: Ptr<gfxSprite>,
    pub m_indent2: i16,
}
crate::impl_base!(MI_SliderField => mi_select_field: MI_SelectField<i16>);

impl MI_SliderField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, nsprSlider: Ptr<gfxSprite>, x: i16, y: i16, name: &str, width: i16, indent1: i16, indent2: i16) -> Self {
        let mut this = MI_SliderField {
            mi_select_field: MI_SelectField::new(nspr, x, y, name, width, indent1),
            m_sprSlider: nsprSlider,
            m_indent2: indent2,
        };

        this.set_position(x, y);
        this
    }

    pub fn set_position(&mut self, x: i16, y: i16) {
        self.mi_select_field.set_position(x, y);
        let (px, py) = (self.m_pos.x, self.m_pos.y);
        let (indent, width) = (self.m_indent, self.m_width);
        self.miModifyImageLeft.set_position(px + indent - 26, py + 4);
        self.miModifyImageRight.set_position(px + width - 16, py + 4);
    }
}

impl UI_ControlTrait for MI_SliderField {
    crate::impl_ctl!();

    fn update(&mut self) {
        self.mi_select_field.update_impl();
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.m_width as i32;
        let indent = self.m_indent as i32;
        let indent2 = self.m_indent2 as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.m_adjustmentY as i32;

        unsafe {
            self.m_spr.draw_src(x, y, &SDL_Rect { x: 0, y: selY, w: indent - 16, h: 32 });
            self.m_spr.draw_src(x + indent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            self.m_spr.draw_src(x + indent + 16, y, &SDL_Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

            rm.menu_font_large.draw_chop_right(x + 16, y + 5, indent - 8, &self.m_name);

            if !self.m_items.is_empty() {
                rm.menu_font_large.draw_chop_right(x + indent2 + 16, y + 5, width - indent2 - 24, &self.current_item().name);
            }
        }

        let iSpacing: i16 = ((indent2 - indent - 20) / ((self.m_items.len() as i16) as i32 - 1)) as i16;
        let mut iSpot: i16 = 0;

        for index in 0..self.m_items.len() {
            if index < self.m_items.len().wrapping_sub(1) {
                self.m_sprSlider.draw_src(x + indent + iSpot as i32 + 16, y + 10, &SDL_Rect { x: 0, y: 0, w: iSpacing as i32, h: 13 });
            } else {
                self.m_sprSlider.draw_src(x + indent + iSpot as i32 + 16, y + 10, &SDL_Rect { x: 164, y: 0, w: 4, h: 13 });
            }

            iSpot = iSpot.wrapping_add(iSpacing);
        }

        // size_t arithmetic in C++, truncated to int at the call
        let knobX = (x + indent).wrapping_add((self.m_index as i32).wrapping_mul(iSpacing as i32)).wrapping_add(14);
        self.m_sprSlider.draw_src(knobX, y + 8, &SDL_Rect { x: 168, y: 0, w: 8, h: 16 });

        let drawLeft = self.m_index > 0;
        if self.m_wraps || drawLeft {
            self.miModifyImageLeft.draw();
        }

        let drawRight = (self.m_index + 1) < self.m_items.len();
        if self.m_wraps || drawRight {
            self.miModifyImageRight.draw();
        }
    }

    fn refresh(&mut self) {
        self.mi_select_field.refresh_impl();
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.mi_select_field.modify_impl(modify)
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        for iPlayer in 0..4usize {
            if playerInput.outputControls[iPlayer].menu_scrollfast().fPressed {
                if self.m_index == 0 {
                    while self.move_next() {}
                } else {
                    while self.move_prev() {}
                }

                return self.mcItemChangedCode;
            }
        }

        self.mi_select_field.send_input_impl(playerInput)
    }

    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        self.mi_select_field.mouse_click_impl(iMouseX, iMouseY)
    }
}
