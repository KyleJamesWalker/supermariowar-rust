//! Warp pipe travel: sliding into a warp, picking an exit, and sliding out.

use crate::game::Game;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::{ShieldStyle, WarpLockStyle};
use crate::common::global_constants::*;
use crate::common::io_block::IO_BlockTrait;
use crate::common::map::*;
use crate::globals::*;
use crate::smw::player::{CPlayer, PlayerState};

#[derive(Clone, Default)]
pub struct PlayerWarpStatus {
    /// Pixels travelled into the entrance, or pixels left to clear the exit.
    pub warpcounter: i16,
    /// Connection group of the warp being used; the exit is chosen from the same group.
    pub warpconnection: i16,
    /// Id of the entrance warp exit, excluded when picking where to come out.
    pub warpid: i16,
    /// Screen coordinate of the pipe mouth, in pixels; sprites are clipped past it.
    pub warpplane: i16,
    /// The fraction of a pixel of warp travel carried into the next tick.
    pub step_remainder: f32,
    pub _alias: Aliased,
}

impl PlayerWarpStatus {
    pub fn new() -> Self {
        PlayerWarpStatus { warpcounter: 0, warpconnection: 0, warpid: 0, warpplane: 0, step_remainder: 0.0, _alias: Aliased::default() }
    }

    fn increasewarpcounter(&mut self, g: &mut Game, player: &mut CPlayer, iGoal: i16) {
        self.warpcounter = self.warpcounter.wrapping_add(1);
        if self.warpcounter > iGoal {
            self.warpcounter = iGoal;
            self.choose_warp_exit(g, player);
        }
    }

    fn decreasewarpcounter(&mut self, player: &mut CPlayer) {
        self.warpcounter = self.warpcounter.wrapping_sub(1);
        if self.warpcounter < 0 {
            self.warpcounter = 0;
            player.state = PlayerState::Ready;
        }
    }

    pub fn get_warp_plane(&self) -> i16 {
        self.warpplane
    }

    /// Moves a warping player one pixel per original-rate tick, carrying the remainder.
    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer) {
        debug_assert!(player.iswarping());

        player.fOldY = player.fy;
        player.fOldX = player.fx;

        debug_assert!(self.warpcounter >= 0);

        self.step_remainder += vel(1.0);
        while self.step_remainder >= 1.0 && player.iswarping() {
            self.step_remainder -= 1.0;
            self.step(g, player);
        }

        debug_assert!(self.warpcounter >= 0);
    }

    /// One pixel of warp travel.
    fn step(&mut self, g: &mut Game, player: &mut CPlayer) {
        match player.state {
            PlayerState::EnteringWarpLeft => {
                player.set_xi((player.left_x() as i32 - 1) as i16);
                self.increasewarpcounter(g, player, (PW + PWOFFSET) as i16);
            }
            PlayerState::EnteringWarpRight => {
                player.set_xi((player.left_x() as i32 + 1) as i16);
                self.increasewarpcounter(g, player, (PW + PWOFFSET) as i16);
            }
            PlayerState::EnteringWarpUp => {
                player.set_yi((player.top_y() as i32 - 1) as i16);
                self.increasewarpcounter(g, player, (TILESIZE - PHOFFSET) as i16);
            }
            PlayerState::EnteringWarpDown => {
                player.set_yi((player.top_y() as i32 + 1) as i16);
                self.increasewarpcounter(g, player, (PH + PHOFFSET) as i16);
            }
            PlayerState::LeavingWarpLeft => {
                player.set_xi((player.left_x() as i32 - 1) as i16);
                self.decreasewarpcounter(player);
            }
            PlayerState::LeavingWarpRight => {
                player.set_xi((player.left_x() as i32 + 1) as i16);
                self.decreasewarpcounter(player);
            }
            PlayerState::LeavingWarpUp => {
                player.set_yi((player.top_y() as i32 - 1) as i16);
                self.decreasewarpcounter(player);
            }
            PlayerState::LeavingWarpDown => {
                player.set_yi((player.top_y() as i32 + 1) as i16);
                self.decreasewarpcounter(player);
            }
            _ => {
                debug_assert!(false);
            }
        }
    }

    /// Starts entering `warp` and applies the entrance warp lock.
    pub fn enter_warp(&mut self, g: &mut Game, player: &mut CPlayer, warp: Ptr<Warp>) {
        let mut gp = g.ptr();
        debug_assert!(warp.direction != WARP_UNDEFINED);

        match warp.direction {
            WARP_DOWN => {
                player.state = PlayerState::EnteringWarpDown;
                player.vely = 0.0f32;
                player.velx = 0.0f32;
                self.warpplane = (player.bottom_y() as i32 + 1) as i16;
            }
            WARP_LEFT => {
                player.state = PlayerState::EnteringWarpLeft;
                player.vely = 0.0f32;
                player.velx = vel(-1.0f32);
                self.warpplane = player.left_x();
            }
            WARP_UP => {
                player.state = PlayerState::EnteringWarpUp;
                player.vely = 0.0f32;
                player.velx = 0.0f32;
                self.warpplane = player.top_y();
            }
            WARP_RIGHT => {
                player.state = PlayerState::EnteringWarpRight;
                player.vely = 0.0f32;
                player.velx = vel(1.0f32);
                self.warpplane = (player.right_x() as i32 + 1) as i16;
            }
            _ => {
                debug_assert!(false);
            }
        }
        player.oldvelx = player.velx;

        self.warpconnection = warp.connection;
        self.warpid = warp.id;

        if g.game_values.warplocktime > 0 {
            match g.game_values.warplockstyle {
                WarpLockStyle::EntranceOnly | WarpLockStyle::EntranceAndExit => {
                    g.g_map.warpexits[warp.id as usize].locktimer = g.game_values.warplocktime;
                }
                WarpLockStyle::EntireConnection => {
                    g.g_map.lockconnection(self.warpconnection as i32);
                }
                WarpLockStyle::AllWarps => {
                    g.g_map.lockconnection(-1);
                }
                _ => {}
            }
        }

        if_sound_on_play(g, &mut gp.rm.sfx_pipe);
    }

    /// Moves the player to a random exit in the connection, triggers the block it faces,
    /// refreshes the shield and applies the exit warp lock.
    pub fn choose_warp_exit(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        let mut exit: Ptr<WarpExit> = gp.g_map.get_random_warp_exit(g, self.warpconnection as i32, self.warpid as i32);
        debug_assert!(!exit.is_null());
        player.set_xi(exit.x);
        player.set_yi(exit.y);
        player.fOldX = player.fx;
        player.fOldY = player.fy;
        player.oldvelx = player.velx;

        player.lockjump = false;
        player.clear_powerup_states();

        //Trigger block that we warp into
        let mut iCol: i16 = (player.left_x() as i32 / TILESIZE) as i16;
        let iRow: i16 = (player.top_y() as i32 / TILESIZE) as i16;
        let mut block: Ptr<dyn IO_BlockTrait>;

        debug_assert!(exit.direction != WARP_EXIT_UNDEFINED);

        match exit.direction {
            WARP_EXIT_UP => {
                player.state = PlayerState::LeavingWarpUp;
                player.velx = 0.0f32;
                player.vely = vel(-4.0f32);
                self.warpcounter = (PH + PHOFFSET) as i16;
                self.warpplane = ((exit.warpy as i32) << 5) as i16;

                if iRow as i32 - 1 >= 0 {
                    block = g.map_block(iCol, (iRow as i32 - 1) as i16);

                    if !block.is_null() && !block.is_transparent() && !block.is_hidden() {
                        block.trigger_behavior(g);
                    }
                }
            }
            WARP_EXIT_RIGHT => {
                player.state = PlayerState::LeavingWarpRight;
                player.velx = vel(1.0f32);
                player.vely = vel(1.0f32);
                self.warpcounter = (PW + PWOFFSET) as i16;
                self.warpplane = (((exit.warpx as i32) << 5) + TILESIZE) as i16;

                if iCol as i32 + 1 >= 20 {
                    iCol = (iCol as i32 - 20) as i16;
                }

                block = g.map_block((iCol as i32 + 1) as i16, iRow);

                if !block.is_null() && !block.is_transparent() && !block.is_hidden() {
                    block.trigger_behavior(g);
                }
            }
            WARP_EXIT_DOWN => {
                player.state = PlayerState::LeavingWarpDown;
                player.velx = 0.0f32;
                player.vely = vel(1.1f32);
                player.inair = true;
                self.warpcounter = (TILESIZE - PHOFFSET) as i16;
                self.warpplane = (((exit.warpy as i32) << 5) + TILESIZE) as i16;

                if (iRow as i32 + 1) < 15 {
                    block = g.map_block(iCol, (iRow as i32 + 1) as i16);

                    if !block.is_null() && !block.is_transparent() && !block.is_hidden() {
                        block.trigger_behavior(g);
                    }
                }
            }
            WARP_EXIT_LEFT => {
                player.state = PlayerState::LeavingWarpLeft;
                player.velx = vel(-1.0f32);
                player.vely = vel(1.0f32);
                self.warpcounter = (PW + PWOFFSET) as i16;
                self.warpplane = ((exit.warpx as i32) << 5) as i16;

                if (iCol as i32 - 1) < 0 {
                    iCol = (iCol as i32 + 20) as i16;
                }

                block = g.map_block((iCol as i32 - 1) as i16, iRow);

                if !block.is_null() && !block.is_transparent() && !block.is_hidden() {
                    block.trigger_behavior(g);
                }
            }
            _ => {
                debug_assert!(false);
            }
        }

        //Make player shielded when exiting the warp (if that option is turned on)
        if g.game_values.shieldstyle != ShieldStyle::NoShield {
            if !player.is_shielded() || (player.shield.time_left() as i32) < g.game_values.shieldtime as i32 {
                player.shield.reset(g);
            }
        }

        //Lock the warp (if that option is turned on)
        if g.game_values.warplocktime > 0 {
            if g.game_values.warplockstyle == WarpLockStyle::ExitOnly || g.game_values.warplockstyle == WarpLockStyle::EntranceAndExit {
                //Lock the warp exit
                exit.locktimer = g.game_values.warplocktime;
            }
        }
    }
}
