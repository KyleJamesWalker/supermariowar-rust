//! Port of src/common/ui/MI_Button.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_Button {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,
    pub szName: String,

    pub iWidth: i16,
    pub iIndent: i16,
    pub m_text_align: TextAlign,

    pub menuCode: MenuCodeEnum,

    pub sprImage: Ptr<gfxSprite>,
    pub iImageSrcX: i16,
    pub iImageSrcY: i16,
    pub iImageW: i16,
    pub iImageH: i16,

    pub iTextW: i16,

    pub iAdjustmentY: i16,
    pub iHalfWidth: i16,

    pub onPressFn: Box<dyn FnMut()>,
}
crate::impl_base!(MI_Button => ui_control: UI_Control);

impl MI_Button {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, align: TextAlign) -> Self {
        let szName: String = name.into();
        let iTextW = unsafe { rm.menu_font_large.get_width(&szName) } as i16;
        MI_Button {
            ui_control: UI_Control::new(x, y),
            spr: nspr,
            szName,
            iWidth: width,
            iIndent: 0,
            m_text_align: align,
            menuCode: MENU_CODE_NONE,
            sprImage: Ptr::null(),
            iImageSrcX: 0,
            iImageSrcY: 0,
            iImageW: 0,
            iImageH: 0,
            iTextW,
            iAdjustmentY: if width > 256 { 0 } else { 128 },
            iHalfWidth: width / 2,
            onPressFn: Box::new(|| {}), // do nothing
        }
    }

    /// `new MI_Button(spr, x, y, name, width)` with the default `TextAlign::LEFT`.
    pub fn new_left(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16) -> Self {
        Self::new(nspr, x, y, name, width, TextAlign::LEFT)
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.szName = name.into();
        self.iTextW = unsafe { rm.menu_font_large.get_width(&self.szName) } as i16;
    }

    pub fn set_code(&mut self, code: MenuCodeEnum) {
        self.menuCode = code;
    }

    pub fn set_image(&mut self, nsprImage: Ptr<gfxSprite>, x: i16, y: i16, w: i16, h: i16) {
        self.sprImage = nsprImage;
        self.iImageSrcX = x;
        self.iImageSrcY = y;
        self.iImageW = w;
        self.iImageH = h;
    }

    pub fn set_on_press(&mut self, func: Box<dyn FnMut()>) {
        self.onPressFn = func;
    }
}

impl UI_ControlTrait for MI_Button {
    crate::impl_ctl!();

    fn modify(&mut self, _fModify: bool) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_UNSELECT_ITEM;
        }

        (self.onPressFn)();
        self.menuCode
    }

    fn send_input(&mut self, _playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        //If input is being sent, that means the button is selected i.e. clicked
        (self.onPressFn)();
        self.menuCode
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let iWidth = self.iWidth as i32;
        let iHalfWidth = self.iHalfWidth as i32;
        let iImageW = self.iImageW as i32;
        let iImageH = self.iImageH as i32;
        let selY = (if self.fSelected { 32 } else { 0 }) + self.iAdjustmentY as i32;

        unsafe {
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: selY, w: iHalfWidth, h: 32 });
            self.spr.draw_src(x + iHalfWidth, y, &SDL_Rect { x: 512 - iWidth + iHalfWidth, y: selY, w: iWidth - iHalfWidth, h: 32 });

            let srcRect = SDL_Rect { x: self.iImageSrcX as i32, y: self.iImageSrcY as i32, w: iImageW, h: iImageH };

            match self.m_text_align {
                TextAlign::LEFT => {
                    rm.menu_font_large.draw_chop_right(x + 16 + (if iImageW > 0 { iImageW + 2 } else { 0 }), y + 5, iWidth - 32, &self.szName);
                    if !self.sprImage.is_null() {
                        self.sprImage.draw_src(x + 16, y + 16 - (iImageH >> 1), &srcRect);
                    }
                }
                TextAlign::CENTER => {
                    rm.menu_font_large.draw_centered(x + ((iWidth + (if iImageW > 0 { iImageW + 2 } else { 0 })) >> 1), y + 5, &self.szName);
                    if !self.sprImage.is_null() {
                        self.sprImage.draw_src(x + (iWidth >> 1) - ((self.iTextW as i32 + iImageW) >> 1) - 1, y + 16 - (iImageH >> 1), &srcRect);
                    }
                }
                TextAlign::RIGHT => {
                    rm.menu_font_large.draw_right_justified(x + iWidth - 16, y + 5, &self.szName);
                    if !self.sprImage.is_null() {
                        self.sprImage.draw_src(x + iWidth - 18 - self.iTextW as i32 - iImageW, y + 16 - (iImageH >> 1), &srcRect);
                    }
                }
            }
        }
    }

    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if self.fDisable {
            return MENU_CODE_NONE;
        }

        let (mx, my) = (iMouseX as i32, iMouseY as i32);
        let (x, y) = (self.m_pos.x as i32, self.m_pos.y as i32);
        if mx >= x && mx < x + self.iWidth as i32 && my >= y && my < y + 32 {
            return self.menuCode;
        }

        MENU_CODE_NONE
    }
}
