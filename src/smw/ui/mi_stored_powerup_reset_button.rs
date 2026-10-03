//! Port of src/smw/ui/MI_StoredPowerupResetButton.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::uicontrol::{TextAlign, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_StoredPowerupResetButton {
    pub mi_button: MI_Button,
}
crate::impl_base!(MI_StoredPowerupResetButton => mi_button: MI_Button);

impl MI_StoredPowerupResetButton {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, name: impl Into<String>, width: i16, align: TextAlign) -> Self {
        MI_StoredPowerupResetButton { mi_button: MI_Button::new(nspr, x, y, name, width, align) }
    }
}

impl UI_ControlTrait for MI_StoredPowerupResetButton {
    crate::impl_ctl!();

    fn modify(&mut self, fModify: bool) -> MenuCodeEnum {
        self.mi_button.modify(fModify)
    }
    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        self.mi_button.send_input(playerInput)
    }
    fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        self.mi_button.mouse_click(iMouseX, iMouseY)
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        self.mi_button.draw();

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;
        let iWidth = self.iWidth as i32;

        unsafe {
            for iPowerup in 0..4i32 {
                rm.spr_selectfield.draw_src(x + iWidth - 142 + iPowerup * 30, y + 4, &SDL_Rect { x: 188, y: 88, w: 24, h: 24 });
                rm.spr_storedpowerupsmall.draw_src(
                    x + iWidth - 138 + iPowerup * 30,
                    y + 8,
                    &SDL_Rect { x: game_values.storedpowerups[iPowerup as usize] as i32 * 16, y: 0, w: 16, h: 16 },
                );
            }
        }
    }
}
