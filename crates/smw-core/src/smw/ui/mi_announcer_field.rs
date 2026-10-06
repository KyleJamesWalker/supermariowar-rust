//! Port of src/smw/ui/MI_AnnouncerField.cpp

use crate::common::file_list::SimpleFileList;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::path::get_name_from_file_name;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_AnnouncerField {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,

    pub szName: String,
    pub iWidth: i16,
    pub iIndent: i16,

    pub szFieldName: String,

    pub miModifyImageLeft: Box<MI_Image>,
    pub miModifyImageRight: Box<MI_Image>,

    pub list: Ptr<SimpleFileList>,
}
crate::impl_base!(MI_AnnouncerField => ui_control: UI_Control);

impl MI_AnnouncerField {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16, pList: Ptr<SimpleFileList>) -> Self {
        let ui_control = UI_Control::new(x, y);
        let (px, py) = (ui_control.m_pos.x, ui_control.m_pos.y);
        let mut this = MI_AnnouncerField {
            ui_control,
            spr: nspr,
            szName: name.into(),
            iWidth: width,
            iIndent: indent,
            szFieldName: String::new(),
            miModifyImageLeft: Box::new(MI_Image::new(nspr, px + indent - 26, py + 4, 32, 64, 26, 24, 4, 1, 8)),
            miModifyImageRight: Box::new(MI_Image::new(nspr, px + width - 16, py + 4, 32, 88, 26, 24, 4, 1, 8)),
            list: pList,
        };
        this.update_name();

        this.miModifyImageLeft.set_visible(false);
        this.miModifyImageRight.set_visible(false);
        this
    }

    pub fn update_name(&mut self) {
        self.szFieldName = get_name_from_file_name(&self.list.current_path().to_string_lossy(), false);
    }

    pub fn modify_impl(&mut self, modify: bool) -> MenuCodeEnum {
        self.miModifyImageLeft.set_visible(modify);
        self.miModifyImageRight.set_visible(modify);

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    pub fn send_input_impl(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        for iPlayer in 0..4usize {
            let out = &playerInput.outputControls[iPlayer];
            if out.menu_right().fPressed || out.menu_down().fPressed {
                self.list.next();
                self.update_name();
                return MENU_CODE_NONE;
            }

            if out.menu_left().fPressed || out.menu_up().fPressed {
                self.list.prev();
                self.update_name();
                return MENU_CODE_NONE;
            }

            if out.menu_random().fPressed {
                self.list.random();
                self.update_name();
                return MENU_CODE_NONE;
            }

            if out.menu_select().fPressed || out.menu_cancel().fPressed {
                self.miModifyImageLeft.set_visible(false);
                self.miModifyImageRight.set_visible(false);

                self.fModifying = false;

                return MENU_CODE_UNSELECT_ITEM;
            }
        }

        MENU_CODE_NONE
    }

    pub fn update_impl(&mut self) {
        self.miModifyImageRight.update();
        self.miModifyImageLeft.update();
    }

    pub fn draw_impl(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let iIndent = self.iIndent as i32;
        let iWidth = self.iWidth as i32;

        unsafe {
            //Draw the select field background
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: if self.fSelected { 32 } else { 0 }, w: iIndent - 16, h: 32 });
            self.spr.draw_src(x + iIndent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 96 } else { 64 }, w: 32, h: 32 });
            self.spr.draw_src(x + iIndent + 16, y, &SDL_Rect { x: 528 - iWidth + iIndent, y: if self.fSelected { 32 } else { 0 }, w: iWidth - iIndent - 16, h: 32 });

            rm.menu_font_large.draw_chop_right(x + 16, y + 5, iIndent - 8, &self.szName);
            rm.menu_font_large.draw_chop_right(x + iIndent + 8, y + 5, iWidth - iIndent - 24, &self.szFieldName);
        }

        self.miModifyImageLeft.draw();
        self.miModifyImageRight.draw();
    }
}

impl UI_ControlTrait for MI_AnnouncerField {
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
        self.send_input_impl(playerInput)
    }
}
