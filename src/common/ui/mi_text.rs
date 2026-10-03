//! Port of src/common/ui/MI_Text.cpp

use crate::common::gfx::gfx_font::gfxFont;
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::globals::*;

pub struct MI_Text {
    pub ui_control: UI_Control,

    szText: String,
    iw: i16,
    m_align: TextAlign,
    font: Ptr<gfxFont>,
}
crate::impl_base!(MI_Text => ui_control: UI_Control);

impl MI_Text {
    pub fn new(text: impl Into<String>, x: i16, y: i16, w: i16, use_large_font: bool, align: TextAlign) -> Self {
        let font = unsafe { if use_large_font { Ptr::from_mut(&mut rm.menu_font_large) } else { Ptr::from_mut(&mut rm.menu_font_small) } };
        MI_Text { ui_control: UI_Control::new(x, y), szText: text.into(), iw: w, m_align: align, font }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.szText = text.into();
    }
}

impl UI_ControlTrait for MI_Text {
    crate::impl_ctl!();

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let (x, y) = (self.m_pos.x as i32, self.m_pos.y as i32);
        if self.m_align == TextAlign::LEFT && self.iw == 0 {
            self.font.draw(x, y, &self.szText);
        } else if self.m_align == TextAlign::LEFT {
            self.font.draw_chop_right(x, y, self.iw as i32, &self.szText);
        } else if self.m_align == TextAlign::CENTER {
            self.font.draw_centered(x, y, &self.szText);
        } else if self.m_align == TextAlign::RIGHT {
            self.font.draw_right_justified(x, y, &self.szText);
        }
    }
}

/// `MI_HeaderText`: a centered large-font `MI_Text`.
pub struct MI_HeaderText;

impl MI_HeaderText {
    pub fn new(text: impl Into<String>, x: i16, y: i16) -> MI_Text {
        MI_Text::new(text, x, y, 0, true, TextAlign::CENTER)
    }
}
