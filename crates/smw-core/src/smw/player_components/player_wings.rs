//! P-wings sprite drawn on a flying player.

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::globals::*;
use crate::smw::player::CPlayer;

#[derive(Clone, Default)]
pub struct PlayerWings {
    /// Ticks in the current flap frame; frames change every 8.
    pub iWingsTimer: u8,
    /// Source x of the wing frame: 0 or 26; 26 when not flying.
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

    /// Animates and draws the wings, clipped while the player warps.
    pub fn draw(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
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

        let fPlayerFacingRight = player.is_facing_right(g);
        let x = player.ix as i32 + if fPlayerFacingRight { -19 } else { 15 };
        let y = player.iy as i32 - 10;
        let src = Rect { x: self.iWingsFrame as i32, y: if fPlayerFacingRight { 0 } else { 32 }, w: 26, h: 32 };
        unsafe {
            if player.iswarping() {
                let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                gp.rm.sprite(gp.rm.spr_wings).draw_clip(g, x, y, &src, edge, player.get_warp_plane() as i32);
            } else {
                gp.rm.sprite(gp.rm.spr_wings).draw_src(g, x, y, &src);
            }
        }
    }
}
