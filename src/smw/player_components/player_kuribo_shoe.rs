//! Port of src/smw/player_components/PlayerKuriboShoe.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::{PHOFFSET, PWOFFSET};
use crate::common::math::vec2::Vec2s;
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::carriable::co_kuribo_shoe::CO_KuriboShoe;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::moving_object::io_moving_object_collision_detection_checksides;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

pub type KuriboShoeType = i32;
pub const NONE: KuriboShoeType = 0;
pub const NORMAL: KuriboShoeType = 1;
pub const STICKY: KuriboShoeType = 2;

const kuriboShoe_exitCode: [u8; 4] = [4, 8, 4, 8];

#[derive(Default)]
pub struct PlayerKuriboShoe {
    pub r#type: KuriboShoeType,
    pub animationTimer: u8,
    pub animationFrame: u8,
    pub exitTimer: u8,
    pub exitIndex: u8,
    pub _alias: Aliased,
}

impl PlayerKuriboShoe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.r#type = NONE;
        self.animationTimer = 0;
        self.animationFrame = 0;
        self.exitTimer = 0;
        self.exitIndex = 0;
    }

    // Return true if the player is inside a shoe
    pub fn is_on(&self) -> bool {
        self.r#type > 0
    }

    pub fn get_type(&self) -> KuriboShoeType {
        self.r#type
    }

    pub fn set_type(&mut self, newtype: KuriboShoeType) {
        self.r#type = newtype;
    }

    pub fn update(&mut self, player: &mut CPlayer, keymask: u8) {
        self.update_getting_out_of_the_shoe(player, keymask); //Free player from the kuribo shoe
        self.update_super_stomp(player);
        self.update_animation();
    }

    // Free player from the kuribo shoe
    fn update_getting_out_of_the_shoe(&mut self, player: &mut CPlayer, keymask: u8) {
        unsafe {
            if self.is_on() && player.isready() && !player.frozen {
                if self.exitIndex > 0 {
                    self.exitTimer = self.exitTimer.wrapping_add(1);
                    if self.exitTimer >= 32 {
                        self.exitIndex = 0;
                        self.exitTimer = 0;
                    }
                }

                if keymask as i32 & kuriboShoe_exitCode[self.exitIndex as usize] as i32 != 0 {
                    self.exitIndex = self.exitIndex.wrapping_add(1);
                } else if keymask as i32 & !(kuriboShoe_exitCode[self.exitIndex as usize] as i32) != 0 {
                    self.exitIndex = 0;
                    self.exitTimer = 0;
                }

                if self.exitIndex == 4 && self.exitTimer < 32 {
                    let shoe = Ptr::new_box(CO_KuriboShoe::new(
                        Ptr::from_mut(&mut rm.spr_kuriboshoe),
                        Vec2s::new((player.ix as i32 - PWOFFSET) as i16, (player.iy as i32 - PHOFFSET - 2) as i16),
                        self.r#type == STICKY,
                    ));
                    io_moving_object_collision_detection_checksides(shoe.get());
                    objectcontainer[1].add(shoe);
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (player.center_x() as i32 - 16) as i16,
                        (player.center_y() as i32 - 16) as i16,
                        3,
                        8,
                    ));

                    self.exitIndex = 0;
                    self.exitTimer = 0;
                    if_sound_on_play(&mut rm.sfx_transform);
                    self.r#type = NONE;

                    player.superstomp.reset();
                }
            }
        }
    }

    fn update_super_stomp(&mut self, player: &mut CPlayer) {
        if self.is_on() && player.can_super_stomp() && player.wants_to_super_stomp() {
            if player.high_jumped() {
                Ptr::from_mut(&mut player.superstomp).get().start_super_stomping(player);
            }
        }
    }

    fn update_animation(&mut self) {
        if self.r#type > 0 {
            self.animationTimer = self.animationTimer.wrapping_add(1);
            if self.animationTimer > 7 {
                self.animationTimer = 0;
                self.animationFrame = self.animationFrame.wrapping_add(32);

                if self.animationFrame > 32 {
                    self.animationFrame = 0;
                }
            }
        }
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        unsafe {
            if self.r#type > 0 {
                let x = player.left_x() as i32 - PWOFFSET;
                let y = player.top_y() as i32 - PHOFFSET;
                let src = SDL_Rect {
                    x: self.animationFrame as i32 + (if self.r#type == STICKY { 64 } else { 0 }),
                    y: if (player.sprite_state & 0x1) == 0 { 0 } else { 32 },
                    w: 32,
                    h: 32,
                };
                if player.iswarping() {
                    let edge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                    rm.spr_kuriboshoe.draw_clip(x, y, &src, edge, player.get_warp_plane() as i32);
                } else {
                    rm.spr_kuriboshoe.draw_src(x, y, &src);
                }
            }
        }
    }
}
