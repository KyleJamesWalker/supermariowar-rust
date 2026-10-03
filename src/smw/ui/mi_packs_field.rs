//! Port of src/smw/ui/MI_PacksField.cpp

use crate::common::file_list::SimpleFileList;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::UI_ControlTrait;
use crate::globals::*;
use crate::smw::ui::mi_announcer_field::MI_AnnouncerField;

pub struct MI_PacksField {
    pub mi_announcer_field: MI_AnnouncerField,

    pub itemChangedCode: MenuCodeEnum,
}
crate::impl_base!(MI_PacksField => mi_announcer_field: MI_AnnouncerField);

impl MI_PacksField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16, pList: Ptr<SimpleFileList>, code: MenuCodeEnum) -> Self {
        MI_PacksField { mi_announcer_field: MI_AnnouncerField::new(nspr, x, y, name, width, indent, pList), itemChangedCode: code }
    }
}

impl UI_ControlTrait for MI_PacksField {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.modify_impl(modify)
    }
    fn update(&mut self) {
        self.update_impl();
    }
    fn draw(&mut self) {
        self.draw_impl();
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let iLastIndex = self.list.current_index();
        let code = self.mi_announcer_field.send_input_impl(playerInput);

        if MENU_CODE_UNSELECT_ITEM == code {
            return code;
        }

        let mut returnCode = MENU_CODE_NONE;
        if iLastIndex != self.list.current_index() {
            returnCode = self.itemChangedCode;
        }

        for iPlayer in 0..4usize {
            let out = &playerInput.outputControls[iPlayer];
            if out.menu_right().fPressed || out.menu_down().fPressed {
                return returnCode;
            }

            if out.menu_left().fPressed || out.menu_up().fPressed {
                return returnCode;
            }

            if out.menu_random().fPressed {
                return returnCode;
            }
        }

        MENU_CODE_NONE
    }
}
