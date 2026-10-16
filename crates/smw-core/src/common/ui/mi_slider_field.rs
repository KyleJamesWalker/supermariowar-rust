//! Port of src/common/ui/MI_SliderField.cpp

use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;

pub struct MI_SliderField {
    pub mi_select_field: MI_SelectField<i16>,

    pub m_sprSlider: SpriteId,
    pub m_indent2: i16,
}
crate::impl_base!(MI_SliderField => mi_select_field: MI_SelectField<i16>);

impl MI_SliderField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: SpriteId, nsprSlider: SpriteId, x: i16, y: i16, name: &str, width: i16, indent1: i16, indent2: i16) -> Self {
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

    fn update(&mut self, g: &mut Game) {
        self.mi_select_field.update_impl(g);
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.m_width as i32;
        let indent = self.m_indent as i32;
        let indent2 = self.m_indent2 as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.m_adjustmentY as i32;

        g.rm.sprite(self.m_spr).draw_src(g, x, y, &Rect { x: 0, y: selY, w: indent - 16, h: 32 });
        g.rm.sprite(self.m_spr).draw_src(g, x + indent - 16, y, &Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
        g.rm.sprite(self.m_spr).draw_src(g, x + indent + 16, y, &Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

        gp.rm.menu_font_large.draw_chop_right(g, x + 16, y + 5, indent - 8, &self.m_name);

        if !self.m_items.is_empty() {
            gp.rm.menu_font_large.draw_chop_right(g, x + indent2 + 16, y + 5, width - indent2 - 24, &self.current_item().name);
        }

        let iSpacing: i16 = ((indent2 - indent - 20) / ((self.m_items.len() as i16) as i32 - 1)) as i16;
        let mut iSpot: i16 = 0;

        for index in 0..self.m_items.len() {
            if index < self.m_items.len().wrapping_sub(1) {
                g.rm.sprite(self.m_sprSlider).draw_src(g, x + indent + iSpot as i32 + 16, y + 10, &Rect { x: 0, y: 0, w: iSpacing as i32, h: 13 });
            } else {
                g.rm.sprite(self.m_sprSlider).draw_src(g, x + indent + iSpot as i32 + 16, y + 10, &Rect { x: 164, y: 0, w: 4, h: 13 });
            }

            iSpot = iSpot.wrapping_add(iSpacing);
        }

        // size_t arithmetic in C++, truncated to int at the call
        let knobX = (x + indent).wrapping_add((self.m_index as i32).wrapping_mul(iSpacing as i32)).wrapping_add(14);
        g.rm.sprite(self.m_sprSlider).draw_src(g, knobX, y + 8, &Rect { x: 168, y: 0, w: 8, h: 16 });

        let drawLeft = self.m_index > 0;
        if self.m_wraps || drawLeft {
            self.miModifyImageLeft.draw(g);
        }

        let drawRight = (self.m_index + 1) < self.m_items.len();
        if self.m_wraps || drawRight {
            self.miModifyImageRight.draw(g);
        }
    }

    fn refresh(&mut self, g: &mut Game) {
        self.mi_select_field.refresh_impl();
    }

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.mi_select_field.modify_impl(modify)
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
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

        self.mi_select_field.send_input_impl(g, playerInput)
    }

    fn mouse_click(&mut self, g: &mut Game, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        self.mi_select_field.mouse_click_impl(iMouseX, iMouseY)
    }
}
