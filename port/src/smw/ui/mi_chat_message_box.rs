//! Port of src/smw/ui/MI_ChatMessageBox.cpp

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

    fn draw(&mut self) {
        // TODO(mi_chat_message_box): port MI_ChatMessageBox::Draw (menu_dialog frame + messages).
    }
}
