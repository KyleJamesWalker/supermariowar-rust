//! Port of src/common/ui/MI_ImageSelectField.cpp

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;

pub struct MI_ImageSelectField {
    pub mi_select_field: MI_SelectField<i16>,

    spr_image: Ptr<gfxSprite>,
    iImageWidth: i16,
    iImageHeight: i16,
}
crate::impl_base!(MI_ImageSelectField => mi_select_field: MI_SelectField<i16>);

impl MI_ImageSelectField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nspr: Ptr<gfxSprite>,
        nspr_image: Ptr<gfxSprite>,
        x: i16,
        y: i16,
        name: impl Into<String>,
        width: i16,
        indent: i16,
        imageHeight: i16,
        imageWidth: i16,
    ) -> Self {
        MI_ImageSelectField {
            mi_select_field: MI_SelectField::new(nspr, x, y, name, width, indent),
            spr_image: nspr_image,
            iImageWidth: imageWidth,
            iImageHeight: imageHeight,
        }
    }

    /// Implicit copy constructor.
    pub fn copy_from(other: &MI_ImageSelectField) -> Self {
        MI_ImageSelectField {
            mi_select_field: MI_SelectField::copy_from(&other.mi_select_field),
            spr_image: other.spr_image,
            iImageWidth: other.iImageWidth,
            iImageHeight: other.iImageHeight,
        }
    }
}

impl UI_ControlTrait for MI_ImageSelectField {
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
        let selY = if self.fSelected { 32 } else { 0 };

        self.m_spr.draw_src(g, x, y, &Rect { x: 0, y: selY, w: indent - 16, h: 32 });
        self.m_spr.draw_src(g, x + indent - 16, y, &Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
        self.m_spr.draw_src(g, x + indent + 16, y, &Rect { x: 528 - width + indent, y: selY, w: width - indent - 16, h: 32 });

        gp.rm.menu_font_large.draw_chop_right(g, x + 16, y + 5, indent - 8, &self.m_name);

        if !self.m_items.is_empty() {
            gp.rm.menu_font_large.draw_chop_right(g, x + indent + self.iImageWidth as i32 + 10, y + 5, width - indent - 24, &self.current_item().name);
        }

        let item = self.current_item();
        let srcIndex = (if item.iconOverride >= 0 { item.iconOverride } else { item.value }) as i32;
        self.spr_image.draw_src(
            g,
            x + indent + 8,
            y + 16 - (self.iImageHeight as i32 >> 1),
            &Rect { x: srcIndex * self.iImageWidth as i32, y: 0, w: self.iImageWidth as i32, h: self.iImageHeight as i32 },
        );

        self.miModifyImageRight.draw(g);
        self.miModifyImageLeft.draw(g);
    }

    fn refresh(&mut self, g: &mut Game) {
        self.mi_select_field.refresh_impl();
    }

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.mi_select_field.modify_impl(modify)
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        self.mi_select_field.send_input_impl(g, playerInput)
    }

    fn mouse_click(&mut self, g: &mut Game, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        self.mi_select_field.mouse_click_impl(iMouseX, iMouseY)
    }
}
