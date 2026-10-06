//! Port of src/smw/player_components/PlayerJail.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::JailStyle;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::{PHOFFSET, PWOFFSET};
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

#[derive(Default)]
pub struct PlayerJail {
    pub owner_teamID: i8,
    pub color: i8,
    pub timer: i16,
    pub _alias: Aliased,
}

impl PlayerJail {
    pub fn new() -> Self {
        PlayerJail { owner_teamID: -1, color: 0, timer: 0, _alias: Aliased::default() }
    }

    pub fn is_active(&self) -> bool {
        self.timer > 0
    }

    pub fn set_color(&mut self, color: i8) {
        assert!(color < 4 && color >= -1);
        self.color = color;
    }

    pub fn get_color(&self) -> i8 {
        self.color
    }

    pub fn lock_in_by(&mut self, inflictor: &mut CPlayer) {
        unsafe {
            self.timer = game_values.gamemodesettings.jail.timetofree;
            self.owner_teamID = inflictor.teamID as i8;

            if game_values.gamemodesettings.jail.style == JailStyle::Owned {
                self.color = inflictor.colorID as i8;
            } else {
                self.color = -1;
            }
        }
    }

    pub fn escape(&mut self, player: &mut CPlayer) {
        unsafe {
            if self.is_active() {
                self.timer = 0;
                self.owner_teamID = -1;

                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_poof),
                    (player.center_x() as i32 - 24) as i16,
                    (player.center_y() as i32 - 24) as i16,
                    4,
                    5,
                ));
                if_sound_on_play(&mut rm.sfx_transform);
            } else {
                if_sound_on_play(&mut rm.sfx_hit);
            }
        }
    }

    pub fn free_by_teammate(&mut self, player: &mut CPlayer) {
        unsafe {
            if self.is_active() {
                self.timer = 0;
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (player.center_x() as i32 - 16) as i16,
                    (player.center_y() as i32 - 16) as i16,
                    3,
                    8,
                ));
                if_sound_on_play(&mut rm.sfx_transform);
            }
        }
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        unsafe {
            if self.is_active() && game_values.gamemodesettings.jail.timetofree > 1 {
                self.timer -= 1;
                if self.timer <= 0 {
                    self.timer = 0;
                    self.owner_teamID = -1;
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (player.center_x() as i32 - 16) as i16,
                        (player.center_y() as i32 - 16) as i16,
                        3,
                        8,
                    ));
                    if_sound_on_play(&mut rm.sfx_transform);
                }
            }
        }
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        unsafe {
            if self.is_active() {
                let x = player.left_x() as i32 - PWOFFSET - 6;
                let y = player.top_y() as i32 - PHOFFSET - 6;
                let src = SDL_Rect { x: (self.color as i32 + 1) * 44, y: 0, w: 44, h: 44 };
                if player.iswarping() {
                    let edge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                    rm.spr_jail.draw_clip(x, y, &src, edge, player.get_warp_plane() as i32);
                } else {
                    rm.spr_jail.draw_src(x, y, &src);
                }
            }
        }
    }
}
