//! Jail state for jail mode: a jailed player is slowed until freed or the timer ends.

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::JailStyle;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::{PHOFFSET, PWOFFSET};
use crate::globals::*;
use crate::smw::player::CPlayer;

#[derive(Clone, Default)]
pub struct PlayerJail {
    /// Team that jailed the player, or -1.
    pub owner_teamID: i8,
    /// Jail sprite color: the jailer's color in owned style, -1 for the neutral sprite.
    pub color: i8,
    /// Ticks left in jail; above 0 means jailed.
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

    /// Jails the player for `timetofree` ticks on behalf of `inflictor`'s team.
    pub fn lock_in_by(&mut self, g: &mut Game, inflictor: &mut CPlayer) {
        self.timer = g.game_values.gamemodesettings.jail.timetofree;
        self.owner_teamID = inflictor.teamID as i8;

        if g.game_values.gamemodesettings.jail.style == JailStyle::Owned {
            self.color = inflictor.colorID as i8;
        } else {
            self.color = -1;
        }
    }

    /// Frees the player after they kill someone; plays a hit sound if they were not jailed.
    pub fn escape(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        if self.is_active() {
            self.timer = 0;
            self.owner_teamID = -1;

            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_poof,
                (player.center_x() as i32 - 24) as i16,
                (player.center_y() as i32 - 24) as i16,
                4,
                5,
            ));
            if_sound_on_play(g, &mut gp.rm.sfx_transform);
        } else {
            if_sound_on_play(g, &mut gp.rm.sfx_hit);
        }
    }

    /// Frees the player when a teammate touches them.
    pub fn free_by_teammate(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        if self.is_active() {
            self.timer = 0;
            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_fireballexplosion,
                (player.center_x() as i32 - 16) as i16,
                (player.center_y() as i32 - 16) as i16,
                3,
                8,
            ));
            if_sound_on_play(g, &mut gp.rm.sfx_transform);
        }
    }

    /// Counts the jail time down; with `timetofree` of 1 or less, jail never expires on its own.
    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        if self.is_active() && g.game_values.gamemodesettings.jail.timetofree > 1 {
            self.timer -= 1;
            if self.timer <= 0 {
                self.timer = 0;
                self.owner_teamID = -1;
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                    g.rm.spr_fireballexplosion,
                    (player.center_x() as i32 - 16) as i16,
                    (player.center_y() as i32 - 16) as i16,
                    3,
                    8,
                ));
                if_sound_on_play(g, &mut gp.rm.sfx_transform);
            }
        }
    }

    pub fn draw(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        unsafe {
            if self.is_active() {
                let x = player.left_x() as i32 - PWOFFSET - 6;
                let y = player.top_y() as i32 - PHOFFSET - 6;
                let src = Rect { x: (self.color as i32 + 1) * 44, y: 0, w: 44, h: 44 };
                if player.iswarping() {
                    let edge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                    gp.rm.sprite(gp.rm.spr_jail).draw_clip(g, x, y, &src, edge, player.get_warp_plane() as i32);
                } else {
                    gp.rm.sprite(gp.rm.spr_jail).draw_src(g, x, y, &src);
                }
            }
        }
    }
}
