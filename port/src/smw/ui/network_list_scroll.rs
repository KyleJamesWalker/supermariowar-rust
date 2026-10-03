//! Port of src/smw/ui/NetworkListScroll.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{ui_control_modify, UI_Control, UI_ControlTrait};
use crate::globals::*;

#[derive(Clone, Debug, Default)]
pub struct NLS_ListItem {
    pub sLeft: String,  // Left-aligned text (eg. name)
    pub sRight: String, // Right-aligned text (eg. player count)
}

impl NLS_ListItem {
    pub fn new(left: &str, right: &str) -> Self {
        NLS_ListItem { sLeft: left.to_string(), sRight: right.to_string() }
    }
}

pub struct MI_NetworkListScroll {
    pub ui_control: UI_Control,

    items: Vec<NLS_ListItem>,

    spr: Ptr<gfxSprite>,
    iAcceptCode: MenuCodeEnum,
    iCancelCode: MenuCodeEnum,

    sTitle: String,
    iNumLines: i16,
    iSelectedLine: i16,
    iSelectedLineBackup: i16,
    iIndex: u16,
    iRemoteIndex: *mut u16,
    iWidth: i16,
    iOffset: i16,

    iTopStop: i16,
    iBottomStop: i16,
}
crate::impl_base!(MI_NetworkListScroll => ui_control: UI_Control);

impl MI_NetworkListScroll {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, numlines: i16, title: &str, acceptCode: MenuCodeEnum, cancelCode: MenuCodeEnum) -> Self {
        MI_NetworkListScroll {
            ui_control: UI_Control::new(x, y),
            items: Vec::new(),
            spr: nspr,
            iAcceptCode: acceptCode,
            iCancelCode: cancelCode,
            sTitle: title.to_string(),
            iNumLines: numlines,
            iSelectedLine: -1,
            iSelectedLineBackup: 0,
            iIndex: 0,
            // TODO(network_list_scroll): C++ defaults iRemoteIndex to &iIndex; null here until the struct is pinned.
            iRemoteIndex: std::ptr::null_mut(),
            iWidth: width,
            iOffset: 0,
            iTopStop: (numlines - 1) >> 1,
            iBottomStop: 0,
        }
    }

    pub fn add(&mut self, left: &str, right: &str) {
        self.items.push(NLS_ListItem::new(left, right));
        // TODO(network_list_scroll): port the iBottomStop / selection updates of MI_NetworkListScroll::Add.
    }

    pub fn clear(&mut self) {
        self.items.clear();
        // TODO(network_list_scroll): port the index resets of MI_NetworkListScroll::Clear.
    }

    pub fn move_next(&mut self) -> bool {
        // TODO(network_list_scroll): port MI_NetworkListScroll::MoveNext.
        false
    }

    pub fn move_prev(&mut self) -> bool {
        // TODO(network_list_scroll): port MI_NetworkListScroll::MovePrev.
        false
    }

    pub fn remote_index(&mut self, index: *mut u16) {
        self.iRemoteIndex = index;
        // TODO(network_list_scroll): port the rest of MI_NetworkListScroll::RemoteIndex.
    }
}

impl UI_ControlTrait for MI_NetworkListScroll {
    crate::impl_ctl!();

    fn update(&mut self) {
        // TODO(network_list_scroll): port MI_NetworkListScroll::Update.
    }

    fn draw(&mut self) {
        // TODO(network_list_scroll): port MI_NetworkListScroll::Draw.
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let _ = playerInput;
        // TODO(network_list_scroll): port MI_NetworkListScroll::SendInput.
        MENU_CODE_NONE
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        // TODO(network_list_scroll): port MI_NetworkListScroll::Modify.
        ui_control_modify(self, modify)
    }
}
