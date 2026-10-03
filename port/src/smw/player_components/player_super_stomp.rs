//! Port of src/smw/player_components/PlayerSuperStomp.cpp

use crate::common::eyecandy::EC_SuperStompExplosion;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::mo_attack_zone::MO_AttackZone;
use crate::smw::player::CPlayer;

const SSTOMP_STRENGTH_TIMER: u16 = 40;

#[derive(Default)]
pub struct PlayerSuperStomp {
    pub superstomp_lock: bool,
    pub floating_timer: i16,
    pub is_stomping: bool,
    pub iSuperStompExitTimer: i16,
    pub _alias: Aliased,
}

impl PlayerSuperStomp {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        self.is_stomping = false;
        self.floating_timer = 0;
        self.iSuperStompExitTimer = 0;
        self.superstomp_lock = false;
    }

    pub fn update_on_ground_hit(&mut self, player: &mut CPlayer) {
        if player.inair {
            return;
        }

        //If the player is touching the ground, then free the lock on super stomping
        self.superstomp_lock = false;

        //If they are touching the ground and they are not the statue, allow them to become the statue again
        if player.tanookisuit.not_statue() {
            player.tanookisuit.allow_statue();
        }

        //If they were super stomping and they are not in the air anymore (i.e. on the ground), then create the
        //super stomp attack zone, play the sound and show the stomp gfx
        if self.is_stomping {
            unsafe {
                eyecandy[2].emplace(EC_SuperStompExplosion::new(Ptr::from_mut(&mut rm.spr_superstomp), player.center_x(), player.bottom_y(), 4));
                if_sound_on_play(&mut rm.sfx_bobombsound);
                self.is_stomping = false;

                objectcontainer[1].add(Ptr::new_box(MO_AttackZone::new(
                    player.get_global_id(),
                    player.get_team_id(),
                    Vec2s::new((player.left_x() as i32 - 32) as i16, (player.top_y() as i32 + 10) as i16),
                    Vec2s::new(32, 15),
                    8,
                    KillStyle::KuriboShoe,
                    false,
                )));
                objectcontainer[1].add(Ptr::new_box(MO_AttackZone::new(
                    player.get_global_id(),
                    player.get_team_id(),
                    Vec2s::new(player.right_x(), (player.top_y() as i32 + 10) as i16),
                    Vec2s::new(32, 15),
                    8,
                    KillStyle::KuriboShoe,
                    false,
                )));
            }
        }
    }

    pub fn start_super_stomping(&mut self, player: &mut CPlayer) {
        // this function should be called to start the super stomping sequence
        debug_assert!(!self.superstomp_lock);
        debug_assert!(!self.is_stomping);
        debug_assert!(self.floating_timer <= 0);

        self.floating_timer = 8;
        self.superstomp_lock = true; // Stop player from super stomping twice before touching the ground
        player.lockfall = true;
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        //Hold super stomping player in mid air then accelerate them downwards
        if self.floating_timer > 0 {
            self.floating_timer -= 1;
            if self.floating_timer <= 0 {
                self.is_stomping = true;
                self.iSuperStompExitTimer = SSTOMP_STRENGTH_TIMER as i16;
                player.vely = VELSUPERSTOMP;

                player.lockfall = true;
            } else {
                player.velx = 0.0f32;
                player.vely = 0.0f32;
            }
        }

        if self.is_stomping && {
            self.iSuperStompExitTimer = self.iSuperStompExitTimer.wrapping_sub(1);
            self.iSuperStompExitTimer <= 0
        } {
            self.is_stomping = false;
        }
    }

    // Player is in the stomping phase
    pub fn is_stomping(&self) -> bool {
        self.is_stomping
    }

    // Player is in any of the SuperStomping phases (floating/stomping/falling)
    pub fn is_in_super_stomp_state(&self) -> bool {
        self.superstomp_lock
    }
}
