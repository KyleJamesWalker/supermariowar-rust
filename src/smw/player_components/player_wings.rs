//! Port of src/smw/player_components/PlayerWings.cpp

use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::globals::*;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

#[derive(Default)]
pub struct PlayerWings {
    pub iWingsTimer: u8,
    pub iWingsFrame: u16,
    pub _alias: Aliased,
}

impl PlayerWings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.iWingsTimer = 0;
        self.iWingsFrame = 0;
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        if player.flying {
            self.iWingsTimer = self.iWingsTimer.wrapping_add(1);
            if self.iWingsTimer >= 8 {
                self.iWingsTimer = 0;
                self.iWingsFrame = self.iWingsFrame.wrapping_add(26);

                if self.iWingsFrame > 26 {
                    self.iWingsFrame = 0;
                }
            }
        } else {
            self.iWingsFrame = 26;
        }

        let fPlayerFacingRight = player.is_facing_right();
        let x = player.ix as i32 + if fPlayerFacingRight { -19 } else { 15 };
        let y = player.iy as i32 - 10;
        let src = SDL_Rect { x: self.iWingsFrame as i32, y: if fPlayerFacingRight { 0 } else { 32 }, w: 26, h: 32 };
        unsafe {
            if player.iswarping() {
                let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                rm.spr_wings.draw_clip(x, y, &src, edge, player.get_warp_plane() as i32);
            } else {
                rm.spr_wings.draw_src(x, y, &src);
            }
        }
    }
}
