//! Port of src/smw/player_components/PlayerTanookiSuit.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::*;
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::player::CPlayer;
use crate::smw::player_components::player_shield::{OFF, SOFT_WITH_STOMP};
use sdl2::sys::SDL_Rect;

#[derive(Default)]
pub struct PlayerTanookiSuit {
    pub tanooki_on: bool,
    pub statue_lock: bool,
    pub statue_timer: u16,
    pub statue_uses_left: u16,
    pub _alias: Aliased,
}

impl PlayerTanookiSuit {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        self.tanooki_on = false;
        self.statue_lock = false;
        self.statue_timer = 0;
        self.statue_uses_left = 0;
    }

    fn can_turn_into_statue(&mut self, player: &mut CPlayer) -> bool {
        player.playerKeys.game_turbo().fPressed
            && player.playerKeys.game_down().fDown
            && !self.statue_lock
            && player.powerupused.is_none()
            && !player.lockfire
            && !player.kuriboshoe.is_on()
            && !player.tail.is_in_use()
    }

    fn start_super_stomping(&mut self, player: &mut CPlayer) {
        let pp = Ptr::from_mut(player);
        player.superstomp.start_super_stomping(pp.get());

        // Become soft shielded (with stomp ability)
        player.shield.set_type(SOFT_WITH_STOMP);
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        if !self.tanooki_on || !player.isready() {
            return;
        }

        unsafe {
            // If the down key is ever released, manually untransform by releasing the down key
            if !player.playerKeys.game_down().fDown && self.statue_timer != 0 {
                // Untransform from statue
                self.statue_timer = 1;
            }
            // Become the tanooki
            else if self.can_turn_into_statue(player) {
                // set the amount of time you get to remain in statue form
                self.statue_timer = 123;

                // perform tansformation effects
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_poof),
                    (player.center_x() as i32 - 24) as i16,
                    (player.center_y() as i32 - 24) as i16,
                    4,
                    5,
                ));
                if_sound_on_play(&mut rm.sfx_transform);

                // Neutralize lateral velocity
                let tv: f32 = 1.6f32;
                if player.velx > tv {
                    player.velx = tv;
                } else if player.velx < -tv {
                    player.velx = -tv;
                }

                // Cause statue to super stomp to ground
                if player.can_super_stomp() {
                    self.start_super_stomping(player);
                }

                // Prevent you from shooting
                player.lockfire = true;

                // Prevent you from falling through solid-on-top blocks
                player.lockfall = true;

                // Prevent you from becoming the statue twice before touching the ground
                self.statue_lock = true;

                //If we were flying or spinning when we became the statue, clear those states
                player.clear_powerup_states();
            }

            // If not super stomping currently
            if !player.superstomp.is_stomping() {
                // Count down the statue timer, which leads to a forced detransformation
                if self.statue_timer == 1 {
                    // Untransform from statue
                    self.statue_timer = 0;

                    // Release invincibility
                    player.shield.set_type(OFF);

                    // Slight upward velocity to escape spikes / lava
                    if !player.inair {
                        player.vely = -8.0 as f32;
                    }

                    // perform transformation effects
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_poof),
                        (player.center_x() as i32 - 24) as i16,
                        (player.center_y() as i32 - 24) as i16,
                        4,
                        5,
                    ));
                    if_sound_on_play(&mut rm.sfx_transform);

                    //Decrease the amount of tanooki uses, if feature is turned on
                    if game_values.tanookilimit > 0 || self.statue_uses_left > 0 {
                        self.statue_uses_left = self.statue_uses_left.wrapping_sub(1);
                        if self.statue_uses_left == 0 {
                            self.tanooki_on = false;
                            self.statue_uses_left = 0;
                            if_sound_on_play(&mut rm.sfx_powerdown);
                        }
                    }
                }
                // Player is a statue
                else if self.statue_timer > 0 {
                    // Prevent player from shooting while a statue
                    player.lockfire = true;

                    // Prevent player from jumping
                    player.lockjump = true;

                    // Don't fall through passable platforms
                    player.fallthrough = false;

                    // Decrement statue timer3
                    self.statue_timer -= 1;

                    // Become soft shielded (with stomp ability)
                    player.shield.set_type(SOFT_WITH_STOMP);

                    self.statue_lock = true;
                }
            }
        }
    }

    pub fn is_on(&self) -> bool {
        self.tanooki_on
    }

    pub fn is_statue(&self) -> bool {
        !self.not_statue()
    }

    pub fn not_statue(&self) -> bool {
        self.statue_timer == 0
    }

    pub fn is_blinking(&self) -> bool {
        (self.statue_timer < 50) && (self.statue_timer / 3 % 2 != 0)
    }

    pub fn on_pickup(&mut self) {
        unsafe {
            if_sound_on_play(&mut rm.sfx_collectpowerup);
            self.tanooki_on = true;

            if game_values.tanookilimit > 0 {
                self.statue_uses_left = game_values.tanookilimit as u16;
            }
        }
    }

    pub fn allow_statue(&mut self) {
        self.statue_lock = false;
    }

    pub fn draw_statue(&mut self, player: &mut CPlayer) {
        debug_assert!(self.is_statue());

        //Blink the statue if the time is almost up
        if player.isready() && self.is_blinking() {
            return;
        }

        //Draw the statue
        let x = player.left_x() as i32 - PWOFFSET;
        let y = player.top_y() as i32 - 31;
        let src = SDL_Rect { x: (player.get_color_id() as i32) << 5, y: 0, w: 32, h: 58 };
        unsafe {
            if player.iswarping() {
                let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                rm.spr_statue.draw_clip(x, y, &src, edge, player.get_warp_plane() as i32);
            } else {
                rm.spr_statue.draw_src(x, y, &src);
            }
        }
    }
}
