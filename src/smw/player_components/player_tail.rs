//! Port of src/smw/player_components/PlayerTail.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::mo_spin_attack::MO_SpinAttack;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

pub type PlayerTailState = i32;
pub const TAIL_NOT_IN_USE: PlayerTailState = 0;
pub const TAIL_SHAKE: PlayerTailState = 1;
pub const TAIL_SPIN_AND_SHAKE: PlayerTailState = 2;

#[derive(Default)]
pub struct PlayerTail {
    pub iTailTimer: i8,
    pub iTailFrame: u16,
    pub iTailState: PlayerTailState,
    pub _alias: Aliased,
}

impl PlayerTail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.iTailTimer = 0;
        self.iTailState = TAIL_NOT_IN_USE;
        self.iTailFrame = 0;
    }

    pub fn is_in_use(&self) -> bool {
        self.iTailState > TAIL_NOT_IN_USE
    }

    pub fn shake(&mut self, player: &mut CPlayer) {
        unsafe {
            rm.sfx_tailspin.stop();
            if_sound_on_play(&mut rm.sfx_tailspin);
        }
        player.lockjump = true;

        self.iTailState = TAIL_SHAKE; //cause tail to shake
        self.iTailTimer = 0;
        self.iTailFrame = 110;
    }

    pub fn spin(&mut self, player: &mut CPlayer) {
        unsafe {
            if_sound_on_play(&mut rm.sfx_tailspin);
        }

        self.iTailState = TAIL_SPIN_AND_SHAKE; //cause tail to shake
        self.iTailTimer = -1; //Add one extra frame to sync with spinning player sprite
        self.iTailFrame = 22;

        let pp = Ptr::from_mut(player);
        player.spin.spin(pp.get());

        unsafe {
            objectcontainer[1].add(Ptr::new_box(MO_SpinAttack::new(
                player.get_global_id(),
                player.get_team_id(),
                KillStyle::Leaf,
                player.is_facing_right(),
                13,
            )));
        }
    }

    //If player is shaking tail, slow decent
    pub fn slow_descent(&mut self, player: &mut CPlayer) {
        if self.iTailState == TAIL_SHAKE {
            if player.vely > 1.5f32 {
                player.vely = 1.5f32;
            }
        }
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        unsafe {
            let mut iOffsetY: i16 = 0;
            if self.iTailState == TAIL_SHAKE {
                self.iTailTimer = self.iTailTimer.wrapping_add(1);
                if self.iTailTimer >= 4 {
                    self.iTailTimer = 0;
                    self.iTailFrame = self.iTailFrame.wrapping_sub(22);

                    if self.iTailFrame < 66 {
                        self.iTailState = TAIL_NOT_IN_USE;

                        if player.powerup == 7 && game_values.leaflimit > 0 {
                            player.decrease_projectile_limit();
                        }
                    }
                }
            } else if self.iTailState == TAIL_SPIN_AND_SHAKE {
                iOffsetY = 52;
                self.iTailTimer = self.iTailTimer.wrapping_add(1);
                if self.iTailTimer >= 4 {
                    self.iTailTimer = 0;
                    self.iTailFrame = self.iTailFrame.wrapping_add(22);
                    if self.iTailFrame > 110 {
                        self.iTailState = TAIL_NOT_IN_USE;
                        self.iTailFrame = 66;

                        if player.powerup == 7 && game_values.leaflimit > 0 {
                            player.decrease_projectile_limit();
                        }
                    }
                }
            }

            //Draw tail will be called by the chicken if he is allowed to glide
            //but we don't want to draw the actual tail for the chicken
            if player.powerup == 7 {
                if self.iTailState == TAIL_NOT_IN_USE {
                    self.iTailTimer = self.iTailTimer.wrapping_add(1);
                    if self.iTailTimer >= 4 {
                        self.iTailTimer = 0;

                        if !player.inair && player.velx != 0.0f32 {
                            self.iTailFrame = self.iTailFrame.wrapping_add(22);
                            if self.iTailFrame > 66 {
                                self.iTailFrame = 22;
                            }
                        } else if !player.inair {
                            self.iTailFrame = 22;
                        } else if player.inair {
                            if player.vely <= 0.0f32 {
                                self.iTailFrame = 66;
                            } else {
                                self.iTailFrame = 110;
                            }
                        }
                    }
                }

                let mut fPlayerFacingRight = !game_values.reversewalk;
                if self.iTailState == TAIL_SPIN_AND_SHAKE {
                    if player.spin.is_reverse_walking() {
                        fPlayerFacingRight = game_values.reversewalk;
                    }
                } else {
                    fPlayerFacingRight = player.is_facing_right();
                }

                let x = player.left_x() as i32 + if fPlayerFacingRight { -18 } else { 18 };
                let y = player.top_y() as i32 + 6;
                let src = SDL_Rect {
                    x: self.iTailFrame as i32,
                    y: (if fPlayerFacingRight { 0 } else { 26 }) + iOffsetY as i32,
                    w: 22,
                    h: 26,
                };
                if player.iswarping() {
                    let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                    rm.spr_tail.draw_clip(x, y, &src, edge, player.get_warp_plane() as i32);
                } else {
                    rm.spr_tail.draw_src(x, y, &src);
                }
            }
        }
    }
}
