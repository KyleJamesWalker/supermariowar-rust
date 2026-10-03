//! Port of src/smw/ui/MI_PlayerSelect.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_PlayerSelect {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,

    pub szName: String,
    pub iWidth: i16,
    pub iIndent: i16,

    pub iSelectedPlayer: i16,
    pub iPlayerPosition: [i16; 4],

    pub miModifyImage: Box<MI_Image>,
}
crate::impl_base!(MI_PlayerSelect => ui_control: UI_Control);

impl MI_PlayerSelect {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, indent: i16) -> Self {
        let ui_control = UI_Control::new(x, y);
        let mut miModifyImage = Box::new(MI_Image::new(nspr, ui_control.m_pos.x, ui_control.m_pos.y - 6, 32, 128, 78, 78, 4, 1, 8));
        miModifyImage.set_visible(false);

        let mut this = MI_PlayerSelect {
            ui_control,
            spr: nspr,
            szName: name.into(),
            iWidth: width,
            iIndent: indent,
            iSelectedPlayer: 0,
            iPlayerPosition: [0; 4],
            miModifyImage,
        };

        let iSpacing: i16 = ((width as i32 - indent as i32 - 136) / 5) as i16;
        for i in 0..this.iPlayerPosition.len() {
            this.iPlayerPosition[i] = (iSpacing as i32 + indent as i32 + i as i32 * (iSpacing as i32 + 34)) as i16;
        }

        this.set_image_position();
        this
    }

    pub fn set_image_position(&mut self) {
        let x = self.m_pos.x + self.iPlayerPosition[self.iSelectedPlayer as usize] - 22;
        let y = self.m_pos.y - 7;
        self.miModifyImage.set_position(x, y);
    }
}

impl UI_ControlTrait for MI_PlayerSelect {
    crate::impl_ctl!();

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.miModifyImage.set_visible(modify);
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            for iPlayer in 0..4usize {
                let out = &playerInput.outputControls[iPlayer];
                if out.menu_right().fPressed {
                    self.iSelectedPlayer += 1;
                    if self.iSelectedPlayer > 3 {
                        self.iSelectedPlayer = 0;
                    }

                    self.set_image_position();
                }

                if out.menu_left().fPressed {
                    self.iSelectedPlayer -= 1;
                    if self.iSelectedPlayer < 0 {
                        self.iSelectedPlayer = 3;
                    }

                    self.set_image_position();
                }
                let sel = self.iSelectedPlayer as usize;

                if out.menu_up().fPressed {
                    game_values.playercontrol[sel] -= 1;
                    if game_values.playercontrol[sel] < 0 {
                        game_values.playercontrol[sel] = 2;
                    }

                    if game_values.playercontrol[sel] == 0 {
                        let mut iCountPlayers = 0;
                        for iPlayer in 0..4usize {
                            if game_values.playercontrol[iPlayer] > 0 {
                                iCountPlayers += 1;
                            }
                        }

                        if iCountPlayers < 2 {
                            game_values.playercontrol[sel] = 2;
                        }
                    }
                }

                if out.menu_down().fPressed {
                    game_values.playercontrol[sel] += 1;
                    if game_values.playercontrol[sel] > 2 {
                        game_values.playercontrol[sel] = 0;
                    }

                    if game_values.playercontrol[sel] == 0 {
                        let mut iCountPlayers = 0;
                        for iPlayer in 0..4usize {
                            if game_values.playercontrol[iPlayer] > 0 {
                                iCountPlayers += 1;
                            }
                        }

                        if iCountPlayers < 2 {
                            game_values.playercontrol[sel] = 1;
                        }
                    }
                }

                if out.menu_select().fPressed || out.menu_cancel().fPressed {
                    self.miModifyImage.set_visible(false);
                    self.fModifying = false;
                    return MENU_CODE_UNSELECT_ITEM;
                }
            }
        }

        MENU_CODE_NONE
    }

    fn update(&mut self) {
        self.miModifyImage.update();
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
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: if self.fSelected { 64 } else { 0 }, w: iIndent - 16, h: 64 });
            self.spr.draw_src(x + iIndent - 16, y, &SDL_Rect { x: 0, y: if self.fSelected { 192 } else { 128 }, w: 32, h: 64 });
            self.spr.draw_src(x + iIndent + 16, y, &SDL_Rect { x: 528 - iWidth + iIndent, y: if self.fSelected { 64 } else { 0 }, w: iWidth - iIndent - 16, h: 64 });

            rm.menu_font_large.draw_chop_right(x + 16, y + 20, iIndent - 8, &self.szName);

            self.miModifyImage.draw();

            for iPlayer in 0..4usize {
                self.spr.draw_src(
                    x + self.iPlayerPosition[iPlayer] as i32,
                    y + 16,
                    &SDL_Rect { x: game_values.playercontrol[iPlayer] as i32 * 34 + 32, y: 206, w: 34, h: 32 },
                );
            }
        }
    }
}
