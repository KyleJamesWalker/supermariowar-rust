//! Port of src/smw/ui/MI_ChatMessageBox.cpp
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};

#[derive(Clone, Debug, Default)]
pub struct CMB_ChatMessage {
    pub playerName: String,
    pub message: String,
    // time?
}

impl CMB_ChatMessage {
    pub fn new(name: String, text: String) -> Self {
        CMB_ChatMessage { playerName: name, message: text }
    }
}

pub struct MI_ChatMessageBox {
    pub ui_control: UI_Control,

    pub messages: Vec<CMB_ChatMessage>,
    pub iNumLines: i16,

    pub iWidth: i16,
    pub iHeight: i16,
}
crate::impl_base!(MI_ChatMessageBox => ui_control: UI_Control);

impl MI_ChatMessageBox {
    pub fn new(x: i16, y: i16, width: i16, numlines: i16) -> Self {
        MI_ChatMessageBox { ui_control: UI_Control::new(x, y), messages: Vec::new(), iNumLines: numlines, iWidth: width, iHeight: 0 }
    }
}

impl UI_ControlTrait for MI_ChatMessageBox {
    crate::impl_ctl!();

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let (x, y) = (self.m_pos.x as i32, self.m_pos.y as i32);
        let w = self.iWidth as i32;
        let n = self.iNumLines as i32;
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, x, y, &Rect { x: 0, y: 0, w: w - 16, h: n * 32 + 32 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, x + w - 16, y, &Rect { x: 496, y: 0, w: 16, h: n * 32 + 32 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, x, y + n * 32 + 32, &Rect { x: 0, y: 464, w: w - 16, h: 16 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, x + w - 16, y + n * 32 + 32, &Rect { x: 496, y: 464, w: 16, h: 16 });

        let font = &gp.rm.menu_font_small;
        let line = font.get_height().max(1);
        let lines = ((n * 32 + 16) / line).max(1) as usize;
        let shown = g.netplay.chat.len().saturating_sub(lines);
        let messages: Vec<String> = g.netplay.chat[shown..].iter().map(|(name, text)| format!("{name}: {text}")).collect();
        for (i, message) in messages.iter().enumerate() {
            font.draw_chop_right(g, x + 16, y + 16 + i as i32 * line, w - 32, message);
        }
    }
}
