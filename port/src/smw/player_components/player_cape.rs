//! Port of src/smw/player_components/PlayerCape.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::{PHOFFSET, PWOFFSET};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::objects::moving::mo_spin_attack::MO_SpinAttack;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

#[derive(Default)]
pub struct PlayerCape {
    pub iCapeTimer: u8,
    pub iCapeFrameX: u16,
    pub iCapeFrameY: u16,
    pub fCapeUp: bool,
    pub iCapeYOffset: u16,
    pub _alias: Aliased,
}

impl PlayerCape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.iCapeTimer = 0;
        self.iCapeFrameX = 0;
        self.iCapeFrameY = 0;
        self.fCapeUp = false;
        self.iCapeYOffset = 0;
    }

    pub fn restart_animation(&mut self) {
        self.iCapeTimer = 4;
    }

    pub fn spin(&mut self, player: &mut CPlayer) {
        unsafe {
            if_sound_on_play(&mut rm.sfx_tailspin);

            self.restart_animation(); //Add one extra frame to sync with spinning player sprite

            Ptr::from_mut(&mut player.spin).get().spin(player);

            objectcontainer[1].add(Ptr::new_box(MO_SpinAttack::new(
                player.get_global_id(),
                player.get_team_id(),
                KillStyle::Feather,
                player.is_facing_right(),
                24,
            )));
        }
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        /* See eyecandy/cape.png for reference */
        unsafe {
            self.iCapeTimer = self.iCapeTimer.wrapping_add(1);
            if self.iCapeTimer > 3 {
                if player.spin.is_spin_in_progress() {
                    self.iCapeFrameX = player.spin.to_cape_frame_x() as u16;
                    self.iCapeFrameY = 32;
                    self.iCapeYOffset = -8i32 as u16;
                } else if (!player.inair && player.velx != 0.0f32) || (player.inair && player.vely < 1.0f32) {
                    self.iCapeFrameX += 32;
                    if self.iCapeFrameX > 96 {
                        self.iCapeFrameX = 0;
                    }

                    self.iCapeFrameY = 0;
                    self.fCapeUp = true;
                    self.iCapeYOffset = 0;
                } else if !player.inair {
                    if self.fCapeUp {
                        self.fCapeUp = false;
                        self.iCapeFrameX = 0;
                    } else {
                        self.iCapeFrameX = 32;
                    }

                    self.iCapeFrameY = 96;
                    self.iCapeYOffset = 0;
                } else if player.inair {
                    self.iCapeFrameX += 32;
                    if self.iCapeFrameX > 64 {
                        self.iCapeFrameX = 0;
                    }

                    self.iCapeFrameY = 64;
                    self.fCapeUp = true;
                    self.iCapeYOffset = -18i32 as u16;
                }

                self.iCapeTimer = 0;
            }

            let mut fPlayerFacingRight = !game_values.reversewalk;
            if player.spin.is_spin_in_progress() {
                if player.spin.is_reverse_walking() {
                    fPlayerFacingRight = game_values.reversewalk;
                }
            } else {
                fPlayerFacingRight = player.is_facing_right();
            }

            // iCapeYOffset is unsigned short, so -8/-18 promote to 65528/65518 here, as in C++.
            let x = player.left_x() as i32 - PWOFFSET + (if fPlayerFacingRight { -18 } else { 18 });
            let y = player.top_y() as i32 - PHOFFSET + 4 + self.iCapeYOffset as i32;
            let src = SDL_Rect {
                x: (if fPlayerFacingRight { 128 } else { 0 }) + self.iCapeFrameX as i32,
                y: self.iCapeFrameY as i32,
                w: 32,
                h: 32,
            };
            if player.iswarping() {
                let edge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                rm.spr_cape.draw_clip(x, y, &src, edge, player.get_warp_plane() as i32);
            } else {
                rm.spr_cape.draw_src(x, y, &src);
            }
        }
    }
}
