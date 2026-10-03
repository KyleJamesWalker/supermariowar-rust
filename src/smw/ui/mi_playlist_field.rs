//! Port of src/smw/ui/MI_PlaylistField.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::sfx::sfxMusic;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_PlaylistField {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,

    pub szName: String,
    pub iWidth: i16,
    pub iIndent: i16,

    pub miModifyImageLeft: Box<MI_Image>,
    pub miModifyImageRight: Box<MI_Image>,
}
crate::impl_base!(MI_PlaylistField => ui_control: UI_Control);

/// The music switch shared by every scroll direction.
unsafe fn restart_menu_music() {
    rm.backgroundmusic[2].stop();
    rm.backgroundmusic[2] = sfxMusic::from_file(musiclist.music(1)).unwrap_or_else(|e| std::panic::panic_any(e));

    if game_values.music {
        rm.backgroundmusic[2].play(false, false);
    }
}

impl MI_PlaylistField {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16) -> Self {
        let ui_control = UI_Control::new(x, y);
        let (px, py) = (ui_control.m_pos.x, ui_control.m_pos.y);
        let mut miModifyImageLeft = Box::new(MI_Image::new(nspr, px + indent - 26, py + 4, 32, 64, 26, 24, 4, 1, 8));
        miModifyImageLeft.set_visible(false);

        let mut miModifyImageRight = Box::new(MI_Image::new(nspr, px + width - 16, py + 4, 32, 88, 26, 24, 4, 1, 8));
        miModifyImageRight.set_visible(false);

        MI_PlaylistField { ui_control, spr: nspr, szName: name.into(), iWidth: width, iIndent: indent, miModifyImageLeft, miModifyImageRight }
    }
}

impl UI_ControlTrait for MI_PlaylistField {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.miModifyImageLeft.set_visible(modify);
        self.miModifyImageRight.set_visible(modify);

        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            for iPlayer in 0..4usize {
                let out = &playerInput.outputControls[iPlayer];
                if out.menu_right().fPressed || out.menu_down().fPressed {
                    musiclist.next();
                    restart_menu_music();
                    return MENU_CODE_NONE;
                }

                if out.menu_left().fPressed || out.menu_up().fPressed {
                    musiclist.prev();
                    restart_menu_music();
                    return MENU_CODE_NONE;
                }

                if out.menu_random().fPressed {
                    musiclist.random();
                    restart_menu_music();
                    return MENU_CODE_NONE;
                }

                if out.menu_select().fPressed || out.menu_cancel().fPressed {
                    self.miModifyImageLeft.set_visible(false);
                    self.miModifyImageRight.set_visible(false);

                    self.fModifying = false;

                    return MENU_CODE_UNSELECT_ITEM;
                }
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self) {
        self.miModifyImageRight.update();
        self.miModifyImageLeft.update();
    }

    fn draw(&mut self) {
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
            rm.menu_font_large.draw_chop_right(x + iIndent + 8, y + 5, iWidth - iIndent - 24, musiclist.current_name());
        }

        self.miModifyImageLeft.draw();
        self.miModifyImageRight.draw();
    }
}
