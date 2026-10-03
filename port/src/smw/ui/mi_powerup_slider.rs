//! Port of src/smw/ui/MI_PowerupSlider.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_slider_field::MI_SliderField;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_PowerupSlider {
    pub mi_slider_field: MI_SliderField,

    pub m_sprPowerup: Ptr<gfxSprite>,
    pub m_powerupIndex: i16,
    pub m_halfWidth: i16,
}
crate::impl_base!(MI_PowerupSlider => mi_slider_field: MI_SliderField);

impl MI_PowerupSlider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, nsprSlider: Ptr<gfxSprite>, nsprPowerup: Ptr<gfxSprite>, x: i16, y: i16, width: i16, powerupIndex: i16) -> Self {
        let mut this = MI_PowerupSlider {
            mi_slider_field: MI_SliderField::new(nspr, nsprSlider, x, y, "", width, 0, 0),
            m_sprPowerup: nsprPowerup,
            m_powerupIndex: powerupIndex,
            m_halfWidth: ((width as i32 - 38) / 2) as i16,
        };

        let (px, py, w) = (this.m_pos.x, this.m_pos.y, this.m_width);
        this.miModifyImageLeft.set_position(px + 25, py + 4);
        this.miModifyImageRight.set_position(px + w - 12, py + 4);
        this
    }
}

impl UI_ControlTrait for MI_PowerupSlider {
    crate::impl_ctl!();

    fn update(&mut self) {
        self.mi_slider_field.update();
    }
    fn refresh(&mut self) {
        self.mi_slider_field.refresh();
    }
    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.mi_slider_field.modify(modify)
    }
    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        self.mi_slider_field.send_input(playerInput)
    }
    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        self.mi_slider_field.mouse_click(iMouseX, iMouseY)
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let width = self.m_width as i32;
        let halfWidth = self.m_halfWidth as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.m_adjustmentY as i32;

        self.m_spr.draw_src(x + 38, y, &SDL_Rect { x: 0, y: selY, w: halfWidth, h: 32 });
        self.m_spr.draw_src(x + 38 + halfWidth, y, &SDL_Rect { x: 550 - width + halfWidth, y: selY, w: width - halfWidth - 38, h: 32 });

        let iSpacing: i16 = ((width - 100) / ((self.m_items.len() as i16) as i32 - 1)) as i16;
        let mut iSpot: i16 = 0;

        for index in 0..self.m_items.len() as u32 {
            if index < (self.m_items.len() as u32).wrapping_sub(1) {
                self.m_sprSlider.draw_src(x + iSpot as i32 + 56, y + 10, &SDL_Rect { x: 0, y: 0, w: iSpacing as i32, h: 13 });
            } else {
                self.m_sprSlider.draw_src(x + iSpot as i32 + 56, y + 10, &SDL_Rect { x: 164, y: 0, w: 4, h: 13 });
            }

            iSpot = iSpot.wrapping_add(iSpacing);
        }

        // size_t arithmetic in C++, truncated to int at the call
        let knobX = x.wrapping_add((self.m_index as i32).wrapping_mul(iSpacing as i32)).wrapping_add(54);
        self.m_sprSlider.draw_src(knobX, y + 8, &SDL_Rect { x: 168, y: 0, w: 8, h: 16 });
        self.m_sprSlider.draw_src(x + width - 34, y + 8, &SDL_Rect { x: self.m_index as i32 * 16, y: 16, w: 16, h: 16 });
        self.m_sprPowerup.draw_src(x, y, &SDL_Rect { x: self.m_powerupIndex as i32 * 32, y: 0, w: 32, h: 32 });

        let drawLeft = self.m_index > 0;
        if self.m_wraps || drawLeft {
            self.miModifyImageLeft.draw();
        }

        let drawRight = (self.m_index + 1) < self.m_items.len();
        if self.m_wraps || drawRight {
            self.miModifyImageRight.draw();
        }
    }
}
