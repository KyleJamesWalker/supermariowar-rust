//! Port of src/smw/ui/MI_ChatMessageBox.cpp

use crate::common::global::rm;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use sdl2::sys::SDL_Rect;

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

    fn draw(&mut self) {
        unsafe {
            let (x, y) = (self.m_pos.x as i32, self.m_pos.y as i32);
            let w = self.iWidth as i32;
            let n = self.iNumLines as i32;
            rm.menu_dialog.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: w - 16, h: n * 32 + 32 });
            rm.menu_dialog.draw_src(x + w - 16, y, &SDL_Rect { x: 496, y: 0, w: 16, h: n * 32 + 32 });
            rm.menu_dialog.draw_src(x, y + n * 32 + 32, &SDL_Rect { x: 0, y: 464, w: w - 16, h: 16 });
            rm.menu_dialog.draw_src(x + w - 16, y + n * 32 + 32, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });
        }
    }
}
