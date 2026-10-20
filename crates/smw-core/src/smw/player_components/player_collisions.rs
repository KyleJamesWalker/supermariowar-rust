//! Player collision handling: against map tiles, other players, and objects.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::game::App;
use crate::common::game_mode::{game_mode_jail, game_mode_shyguytag, game_mode_tag};
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::global_constants::*;
use crate::common::io_block::IO_BlockTrait;
use crate::common::object_base::{cap_side_velocity, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::tile_types::tile_flag_solid;
use crate::globals::*;
use crate::smw::player::{CPlayer, PlayerDeathStyle, PlayerState};
use crate::smw::player_components::player_shield::{HARD, OFF, SOFT, SOFT_WITH_STOMP};

#[derive(Clone, Default)]
/// Stateless; groups the player's collision routines.
pub struct PlayerCollisions {
    pub _alias: Aliased,
}

/// True for a block that exists and is neither transparent nor hidden.
fn is_solid_block(mut block: Ptr<dyn IO_BlockTrait>) -> bool {
    !block.is_null() && !block.is_transparent() && !block.is_hidden()
}

impl PlayerCollisions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes the player down out of a solid tile or block at their top edge; returns true on a hit.
    pub fn checktop(&mut self, g: &mut Game, player: &mut CPlayer) -> bool {
        if (player.top_y() as f32) < 0.0f32 {
            return false;
        }

        let tile_y: i16 = (player.top_y() as i32 / TILESIZE) as i16;

        if tile_y < 0 || tile_y as i32 >= MAPHEIGHT {
            return false;
        }

        let tile_x_left: i16 = (player.left_x() as i32 / TILESIZE) as i16;

        if tile_x_left < 0 || tile_x_left as i32 >= MAPWIDTH {
            return false;
        }

        let tile_x_right: i16 = if player.right_x() >= 640 {
            ((player.right_x() as i32 - 640) / TILESIZE) as i16
        } else {
            (player.right_x() as i32 / TILESIZE) as i16
        };

        if tile_x_right < 0 || tile_x_right as i32 >= MAPWIDTH {
            return false;
        }

        let leftTile: i32 = g.g_map.map(tile_x_left as i32, tile_y as i32);
        let rightTile: i32 = g.g_map.map(tile_x_right as i32, tile_y as i32);
        let leftBlock = g.map_block(tile_x_left, tile_y);
        let rightBlock = g.map_block(tile_x_right, tile_y);

        if (leftTile & tile_flag_solid) != 0
            || (rightTile & tile_flag_solid) != 0
            || is_solid_block(leftBlock)
            || is_solid_block(rightBlock)
        {
            player.set_yf((((tile_y as i32) << 5) + TILESIZE) as f32 + 0.2f32);
            return true;
        }

        false
    }

    /// Pushes the player right out of a solid tile or block at their left edge; returns true on a hit.
    fn checkleft(&mut self, g: &mut Game, player: &mut CPlayer) -> bool {
        if player.fy < 0.0f32 {
            return false;
        }

        let tile_y: i16 = ((player.fy as i16) as i32 / TILESIZE) as i16;

        if tile_y < 0 || tile_y as i32 >= MAPHEIGHT {
            return false;
        }

        let tile_y2: i16 = (((player.fy as i16) as i32 + PH) / TILESIZE) as i16;

        if tile_y2 < 0 || tile_y2 as i32 >= MAPHEIGHT {
            return false;
        }

        let tile_x: i16 = (player.ix as i32 / TILESIZE) as i16;

        if tile_x < 0 || tile_x as i32 >= MAPWIDTH {
            return false;
        }

        let topTile: i32 = g.g_map.map(tile_x as i32, tile_y as i32);
        let bottomTile: i32 = g.g_map.map(tile_x as i32, tile_y2 as i32);
        let topBlock = g.map_block(tile_x, tile_y);
        let bottomBlock = g.map_block(tile_x, tile_y2);

        if (topTile & tile_flag_solid) != 0
            || (bottomTile & tile_flag_solid) != 0
            || is_solid_block(topBlock)
            || is_solid_block(bottomBlock)
        {
            player.set_xf((((tile_x as i32) << 5) + TILESIZE) as f32 + 0.2f32);
            player.flipsidesifneeded();
            return true;
        }

        false
    }

    /// Pushes the player left out of a solid tile or block at their right edge; returns true on a hit.
    fn checkright(&mut self, g: &mut Game, player: &mut CPlayer) -> bool {
        if player.fy < 0.0f32 {
            return false;
        }

        let tile_y: i16 = ((player.fy as i16) as i32 / TILESIZE) as i16;

        if tile_y < 0 || tile_y as i32 >= MAPHEIGHT {
            return false;
        }

        let tile_y2: i16 = (((player.fy as i16) as i32 + PH) / TILESIZE) as i16;

        if tile_y2 < 0 || tile_y2 as i32 >= MAPHEIGHT {
            return false;
        }

        let tile_x: i16 = if player.right_x() >= 640 {
            ((player.right_x() as i32 - 640) / TILESIZE) as i16
        } else {
            (player.right_x() as i32 / TILESIZE) as i16
        };

        if tile_x < 0 || tile_x as i32 >= MAPWIDTH {
            return false;
        }

        let topTile: i32 = g.g_map.map(tile_x as i32, tile_y as i32);
        let bottomTile: i32 = g.g_map.map(tile_x as i32, tile_y2 as i32);
        let topBlock = g.map_block(tile_x, tile_y);
        let bottomBlock = g.map_block(tile_x, tile_y2);

        if (topTile & tile_flag_solid) != 0
            || (bottomTile & tile_flag_solid) != 0
            || is_solid_block(topBlock)
            || is_solid_block(bottomBlock)
        {
            player.set_xf((((tile_x as i32) << 5) - PW) as f32 - 0.2f32);
            player.flipsidesifneeded();
            return true;
        }

        false
    }

    /// Pushes the player out of solid tiles touching any of their four corners.
    pub fn checksides(&mut self, g: &mut Game, player: &mut CPlayer) {
        //First figure out where the corners of this object are touching
        let mut iCase: u8 = 0;

        let tile_x_left: i16 = (player.left_x() as i32 >> 5) as i16;

        let tile_x_right: i16 = if player.right_x() >= 640 {
            ((player.right_x() as i32 - 640) >> 5) as i16
        } else {
            (player.right_x() as i32 >> 5) as i16
        };

        let tile_y: i16 = (player.top_y() as i32 >> 5) as i16;
        let tile_y2: i16 = (player.bottom_y() as i32 >> 5) as i16;

        if player.top_y() >= 0 {
            if (tile_y as i32) < MAPHEIGHT {
                if tile_x_left >= 0 && (tile_x_left as i32) < MAPWIDTH {
                    let block = g.map_block(tile_x_left, tile_y);

                    if is_solid_block(block) || (g.g_map.map(tile_x_left as i32, tile_y as i32) & tile_flag_solid) != 0 {
                        iCase |= 0x01;
                    }
                }

                if tile_x_right >= 0 && (tile_x_right as i32) < MAPWIDTH {
                    let block = g.map_block(tile_x_right, tile_y);

                    if is_solid_block(block) || (g.g_map.map(tile_x_right as i32, tile_y as i32) & tile_flag_solid) != 0 {
                        iCase |= 0x02;
                    }
                }
            }
        }

        // TODO: is this correct?
        if (player.top_y() as i32 + PW) as f32 >= 0.0f32 {
            if (tile_y2 as i32) < MAPHEIGHT {
                if tile_x_left >= 0 && (tile_x_left as i32) < MAPWIDTH {
                    let block = g.map_block(tile_x_left, tile_y2);

                    if is_solid_block(block) || (g.g_map.map(tile_x_left as i32, tile_y2 as i32) & tile_flag_solid) != 0 {
                        iCase |= 0x04;
                    }
                }

                if tile_x_right >= 0 && (tile_x_right as i32) < MAPWIDTH {
                    let block = g.map_block(tile_x_right, tile_y2);

                    if is_solid_block(block) || (g.g_map.map(tile_x_right as i32, tile_y2 as i32) & tile_flag_solid) != 0 {
                        iCase |= 0x08;
                    }
                }
            }
        }

        let left_edge = (((tile_x_left as i32) << 5) + TILESIZE) as f32 + 0.2f32;
        let right_edge = (((tile_x_right as i32) << 5) - PW) as f32 - 0.2f32;
        let top_edge = (((tile_y as i32) << 5) + TILESIZE) as f32 + 0.2f32;
        let bottom_edge = (((tile_y2 as i32) << 5) - PH) as f32 - 0.2f32;

        //Then determine which way is the best way to move this object out of the solid map areas
        match iCase {
            //Do nothing
            //[ ][ ]
            //[ ][ ]
            0 => {}

            //[X][ ]
            //[ ][ ]
            1 => {
                if player.left_x() as i32 + (PW >> 1) > ((tile_x_left as i32) << 5) + TILESIZE {
                    player.set_xf(left_edge);
                    player.flipsidesifneeded();
                } else {
                    player.set_yf(top_edge);
                }
            }

            //[ ][X]
            //[ ][ ]
            2 => {
                if (player.left_x() as i32 + (PW >> 1)) < ((tile_x_right as i32) << 5) {
                    player.set_xf(right_edge);
                    player.flipsidesifneeded();
                } else {
                    player.set_yf(top_edge);
                }
            }

            //[X][X]
            //[ ][ ]
            3 => {
                player.set_yf(top_edge);
            }

            //[ ][ ]
            //[X][ ]
            4 => {
                if player.left_x() as i32 + (PW >> 1) > ((tile_x_left as i32) << 5) + TILESIZE {
                    player.set_xf(left_edge);
                    player.flipsidesifneeded();
                } else {
                    player.set_yf(bottom_edge);
                }
            }

            //[X][ ]
            //[X][ ]
            5 => {
                player.set_xf(left_edge);
                player.flipsidesifneeded();
            }

            //[ ][X]
            //[X][ ]
            6 => {
                if player.left_x() as i32 + (PW >> 1) > ((tile_x_left as i32) << 5) + TILESIZE {
                    player.set_yf(top_edge);
                    player.set_xf(left_edge);
                    player.flipsidesifneeded();
                } else {
                    player.set_yf(bottom_edge);
                    player.set_xf(right_edge);
                    player.flipsidesifneeded();
                }
            }

            //[X][X]
            //[X][ ]
            7 => {
                player.set_yf(top_edge);
                player.set_xf(left_edge);
                player.flipsidesifneeded();
            }

            //[ ][ ]
            //[ ][X]
            8 => {
                if (player.left_x() as i32 + (PW >> 1)) < ((tile_x_right as i32) << 5) {
                    player.set_xf(right_edge);
                    player.flipsidesifneeded();
                } else {
                    player.set_yf(bottom_edge);
                }
            }

            //[X][ ]
            //[ ][X]
            9 => {
                if player.left_x() as i32 + (PW >> 1) > ((tile_x_left as i32) << 5) + TILESIZE {
                    player.set_yf(bottom_edge);
                    player.set_xf(left_edge);
                    player.flipsidesifneeded();
                } else {
                    player.set_yf(top_edge);
                    player.set_xf(right_edge);
                    player.flipsidesifneeded();
                }
            }

            //[ ][X]
            //[ ][X]
            10 => {
                player.set_xf(right_edge);
                player.flipsidesifneeded();
            }

            //[X][X]
            //[ ][X]
            11 => {
                player.set_yf(top_edge);
                player.set_xf(right_edge);
                player.flipsidesifneeded();
            }

            //[ ][ ]
            //[X][X]
            12 => {
                player.set_yf(bottom_edge);
            }

            //[X][ ]
            //[X][X]
            13 => {
                player.set_yf(bottom_edge);
                player.set_xf(left_edge);
                player.flipsidesifneeded();
            }

            //[ ][X]
            //[X][X]
            14 => {
                player.set_yf(bottom_edge);
                player.set_xf(right_edge);
                player.flipsidesifneeded();
            }

            //If object is completely inside a block, default to moving it down
            //[X][X]
            //[X][X]
            15 => {
                player.set_yf((((tile_y2 as i32) << 5) + TILESIZE) as f32 + 0.2f32);
            }

            _ => {}
        }
    }

    /// If `player` came down onto `other`, bounces them and kills `other` when falling fast enough
    /// and `other` is unshielded; returns true on any landing.
    fn is_stomping(&mut self, g: &mut Game, mut player: PlayerId, mut other: PlayerId) -> bool {
        if g.player(player).fOldY + PH as f32 <= g.player(other).fOldY && g.player(player).bottom_y() >= g.player(other).top_y() {
            //don't reposition if player is warping when he kills the other player
            if g.player(player).isready() {
                g.player(player).set_yi((g.player(other).iy as i32 - PH) as i16); //set new position to top of other player
                let pp = player;
                g.player(player).collisions.checktop(g, g.player(pp));
                g.player(player).platform = PlatformId::NONE;
            }

            let mut fKillPotential = false;
            if g.player(player).vely > vel(1.0f32) && !g.player(other).is_shielded() {
                fKillPotential = true;
            }

            g.player(player).bouncejump(g);

            if fKillPotential {
                let mut style = KillStyle::Stomp;
                if g.player(player).flying {
                    style = KillStyle::PWings;
                } else if g.player(player).kuriboshoe.is_on() {
                    style = KillStyle::KuriboShoe;
                } else if g.player(player).tail.is_in_use() {
                    style = KillStyle::Leaf;
                } else if g.player(player).extrajumps > 0 {
                    style = KillStyle::Feather;
                }

                g.player(player).killed_player(g, other, PlayerDeathStyle::Squish, style, false, false);
            } else {
                if g.mode().gamemode == game_mode_tag {
                    g.player(player).transfer_tag(g, other);
                }

                if g.mode().gamemode == game_mode_shyguytag {
                    g.player(player).transfer_shy_guy(g, other);
                }

                g.player(player).iSuicideCreditPlayerID = g.player(other).globalID;
                g.player(player).iSuicideCreditTimer = ticks(20) as i16;
            }

            return true;
        }

        false
    }

    /// Resolves two overlapping players: team rules, freezes, invincibility, stomps, then pushback.
    pub fn handle_p2p(&mut self, g: &mut Game, mut o1: PlayerId, mut o2: PlayerId) {
        let mut gp = g.ptr();
        //If teams tag each other
        if g.player(o1).teamID == g.player(o2).teamID {
            //Free teammates that are jailed
            if g.mode().gamemode == game_mode_jail && g.game_values.gamemodesettings.jail.tagfree {
                let p1 = o1;
                g.player(o1).jail.free_by_teammate(g, g.player(p1));
                let p2 = o2;
                g.player(o2).jail.free_by_teammate(g, g.player(p2));
            }

            //Transfer tag if assist is on
            if (g.mode().gamemode == game_mode_tag && g.game_values.teamcollision == TeamCollisionStyle::Assist)
                || g.game_values.gamemodesettings.tag.tagontouch
            {
                g.player(o1).transfer_tag(g, o2);
            }

            //Don't collision detect players on same team if friendly fire is turned off
            if g.game_values.teamcollision == TeamCollisionStyle::Off {
                return;
            }

            //Team assist is enabled so allow powerup trading and super jumping
            if g.game_values.teamcollision == TeamCollisionStyle::Assist {
                g.player(o1).bounce_assist_player(g, o2);
                g.player(o2).bounce_assist_player(g, o1);

                //Allow players on team to swap stored items
                if g.player(o1).keys(g).game_powerup().fPressed || g.player(o2).keys(g).game_powerup().fPressed {
                    let (id1, id2) = (g.player(o1).globalID as usize, g.player(o2).globalID as usize);
                    g.game_values.gamepowerups.swap(id1, id2);

                    if_sound_on_play(g, &mut gp.rm.sfx_storepowerup);

                    g.player(o1).keys(g).game_powerup_mut().fPressed = false;
                    g.player(o1).keys(g).game_powerup_mut().fDown = false;
                    g.player(o2).keys(g).game_powerup_mut().fPressed = false;
                    g.player(o2).keys(g).game_powerup_mut().fDown = false;
                }

                return;
            }
        }

        //Quit checking collision if either player is soft shielded
        if g.player(o1).shield.get_type() == SOFT || g.player(o2).shield.get_type() == SOFT {
            //Do tag transfer if there is one to do
            if g.mode().gamemode == game_mode_tag && g.game_values.gamemodesettings.tag.tagontouch {
                g.player(o1).transfer_tag(g, o2);
            }

            if g.mode().gamemode == game_mode_shyguytag && g.game_values.gamemodesettings.shyguytag.tagtransfer != 1 {
                g.player(o1).transfer_shy_guy(g, o2);
            }

            return;
        }

        //--- 1. kill frozen players ---
        let mut fFrozenDeath = false;
        if g.player(o1).frozen && !g.player(o1).is_shielded() && !g.player(o1).is_invincible() {
            g.player(o2).killed_player(g, o1, PlayerDeathStyle::Shatter, KillStyle::IceBlast, true, false);
            fFrozenDeath = true;
        }

        if g.player(o2).frozen && !g.player(o2).is_shielded() && !g.player(o2).is_invincible() {
            g.player(o1).killed_player(g, o2, PlayerDeathStyle::Shatter, KillStyle::IceBlast, true, false);
            fFrozenDeath = true;
        }

        if fFrozenDeath {
            return;
        }

        //--- 2. is player invincible? ---
        if g.player(o1).is_invincible() && !g.player(o2).is_shielded() && !g.player(o2).is_invincible() {
            g.player(o1).killed_player(g, o2, PlayerDeathStyle::Jump, KillStyle::Star, false, false);
            return;
        }

        if g.player(o2).is_invincible() && !g.player(o1).is_shielded() && !g.player(o1).is_invincible() {
            g.player(o2).killed_player(g, o1, PlayerDeathStyle::Jump, KillStyle::Star, false, false);
            return;
        }

        //If both players are warping, ignore collision
        if g.player(o1).iswarping() && g.player(o2).iswarping() {
            return;
        }

        //--- 3. stomping other player? ---
        if (g.player(o2).shield.get_type() == OFF || g.player(o2).shield.get_type() == HARD) && !g.player(o2).is_invincible() && self.is_stomping(g, o1, o2) {
            return;
        }
        if (g.player(o1).shield.get_type() == OFF || g.player(o1).shield.get_type() == HARD) && !g.player(o1).is_invincible() && self.is_stomping(g, o2, o1) {
            return;
        }

        //Quit checking collision if either player is hard shielded
        if g.player(o1).shield.get_type() == SOFT_WITH_STOMP || g.player(o2).shield.get_type() == SOFT_WITH_STOMP {
            //Do tag transfer if there is one to do
            if g.mode().gamemode == game_mode_tag && g.game_values.gamemodesettings.tag.tagontouch {
                g.player(o1).transfer_tag(g, o2);
            }

            if g.mode().gamemode == game_mode_shyguytag && g.game_values.gamemodesettings.shyguytag.tagtransfer != 1 {
                g.player(o1).transfer_shy_guy(g, o2);
            }

            return;
        }

        //--- 4. push back (horizontal) ---
        if g.player(o1).ix < g.player(o2).ix {
            //o1 is left -> o1 pushback left, o2 pushback right
            self.handle_p2p_pushback(g, o1, o2);
        } else {
            self.handle_p2p_pushback(g, o2, o1);
        }
    }

    /// Lets the object handle touching the player; returns the object's result.
    pub fn handle_p2o(&mut self, g: &mut Game, o1: PlayerId, mut o2: Ptr<dyn CObjectTrait>) -> bool {
        o2.collide_player(g, o1)
    }

    /// Separates two side-by-side players (`o1` on the left) and bounces them apart, handling
    /// screen wrap; a kuribo shoe pushes harder.
    fn handle_p2p_pushback(&mut self, g: &mut Game, mut o1: PlayerId, mut o2: PlayerId) {
        //o1 is left to o2
        //  |o1||o2|
        //-----------

        let p1 = o1;
        let p2 = o2;

        //Transfer tag on touching other players
        if g.mode().gamemode == game_mode_tag && g.game_values.gamemodesettings.tag.tagontouch {
            g.player(o1).transfer_tag(g, o2);
        }

        if g.mode().gamemode == game_mode_shyguytag && g.game_values.gamemodesettings.shyguytag.tagtransfer != 1 {
            g.player(o1).transfer_shy_guy(g, o2);
        }

        let mut overlapcollision = false;
        if (g.player(o1).ix as i32 + PW) < 320 && g.player(o2).ix > 320 {
            overlapcollision = true;
        }

        if g.player(o1).iswarping() {
            if overlapcollision {
                //o2 reposition to the right side of o1, o1 stays
                g.player(o2).set_xi((g.player(o1).ix as i32 - PW + App::screenWidth - 1) as i16);
                g.player(o2).collisions.checkleft(g, g.player(p2));
            } else {
                g.player(o2).set_xi((g.player(o1).ix as i32 + PW + 1) as i16);
                g.player(o2).collisions.checkright(g, g.player(p2));
            }

            return;
        } else if g.player(o2).iswarping() {
            if overlapcollision {
                //o1 reposition to the left side of o2, o2 stays
                g.player(o1).set_xi((g.player(o2).ix as i32 + PW - App::screenWidth - 1) as i16);
                g.player(o1).collisions.checkright(g, g.player(p1));
            } else {
                g.player(o1).set_xi((g.player(o2).ix as i32 - PW - 1) as i16);
                g.player(o1).collisions.checkleft(g, g.player(p1));
            }

            return;
        } else if !g.player(o1).iswarping() && !g.player(o2).iswarping() {
            //both objects moving - calculate middle and set both objects

            if overlapcollision {
                let middle: i16 = (g.player(o2).ix as i32 - App::screenWidth + ((g.player(o1).ix as i32 + PW) - g.player(o2).ix as i32 - App::screenWidth) / 2) as i16; //no ABS needed (o1->x < o2->x -> o1->x+w > o2->x !)
                g.player(o1).set_xi((middle as i32 + 1) as i16); //o1 is left
                g.player(o2).set_xi((middle as i32 - PW + App::screenWidth - 1) as i16); //o2 is right

                g.player(o1).collisions.checkright(g, g.player(p1));
                g.player(o2).collisions.checkleft(g, g.player(p2));
            } else {
                let middle: i16 = (g.player(o2).ix as i32 + ((g.player(o1).ix as i32 + PW) - g.player(o2).ix as i32) / 2) as i16; //no ABS needed (o1->x < o2->x -> o1->x+w > o2->x !)
                g.player(o1).set_xi((middle as i32 - PW - 1) as i16); //o1 is left
                g.player(o2).set_xi((middle as i32 + 1) as i16); //o2 is right

                g.player(o1).collisions.checkleft(g, g.player(p1));
                g.player(o2).collisions.checkright(g, g.player(p2));
            }
        }

        let absv1: f32;
        let absv2: f32;

        let mut dPlayer1Pushback: f32 = 1.5f32;
        let mut dPlayer2Pushback: f32 = 1.5f32;

        if g.player(o1).kuriboshoe.is_on() && !g.player(o2).kuriboshoe.is_on() {
            dPlayer1Pushback = 0.5f32;
            dPlayer2Pushback = 2.5f32;
        } else if !g.player(o1).kuriboshoe.is_on() && g.player(o2).kuriboshoe.is_on() {
            dPlayer1Pushback = 2.5f32;
            dPlayer2Pushback = 0.5f32;
        }

        if overlapcollision {
            absv1 = (if g.player(o1).velx < 0.0f32 { g.player(o1).velx } else { vel(-1.0f32) }) * dPlayer2Pushback; //o1 is on the left side (only positive velx counts)
            absv2 = (if g.player(o2).velx > 0.0f32 { g.player(o2).velx } else { vel(1.0f32) }) * dPlayer1Pushback; //o2 right (only negative velx counts)
        } else {
            absv1 = (if g.player(o1).velx > 0.0f32 { g.player(o1).velx } else { vel(1.0f32) }) * dPlayer2Pushback; //o1 is on the left side (only positive velx counts)
            absv2 = (if g.player(o2).velx < 0.0f32 { g.player(o2).velx } else { vel(-1.0f32) }) * dPlayer1Pushback; //o2 right (only negative velx counts)
        }

        if g.player(o1).state == PlayerState::Ready {
            g.player(o1).velx = cap_side_velocity(absv2);
            g.player(o1).iSuicideCreditPlayerID = g.player(o2).globalID;
            g.player(o1).iSuicideCreditTimer = TICKS_PER_SECOND as i16;
        }

        if g.player(o2).state == PlayerState::Ready {
            g.player(o2).velx = cap_side_velocity(absv1);
            g.player(o2).iSuicideCreditPlayerID = g.player(o1).globalID;
            g.player(o2).iSuicideCreditTimer = TICKS_PER_SECOND as i16;
        }
    }
}
