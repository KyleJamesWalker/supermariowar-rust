//! Port of src/smw/player_components/PlayerCollisions.cpp

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

#[derive(Default)]
pub struct PlayerCollisions {
    pub _alias: Aliased,
}

/// `block && !block->isTransparent() && !block->isHidden()`
fn is_solid_block(mut block: Ptr<dyn IO_BlockTrait>) -> bool {
    !block.is_null() && !block.is_transparent() && !block.is_hidden()
}

impl PlayerCollisions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn checktop(&mut self, player: &mut CPlayer) -> bool {
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

        unsafe {
            let leftTile: i32 = g_map.map(tile_x_left as i32, tile_y as i32);
            let rightTile: i32 = g_map.map(tile_x_right as i32, tile_y as i32);
            let leftBlock = g_map.block(tile_x_left, tile_y);
            let rightBlock = g_map.block(tile_x_right, tile_y);

            if (leftTile & tile_flag_solid) != 0
                || (rightTile & tile_flag_solid) != 0
                || is_solid_block(leftBlock)
                || is_solid_block(rightBlock)
            {
                player.set_yf((((tile_y as i32) << 5) + TILESIZE) as f32 + 0.2f32);
                return true;
            }
        }

        false
    }

    fn checkleft(&mut self, player: &mut CPlayer) -> bool {
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

        unsafe {
            let topTile: i32 = g_map.map(tile_x as i32, tile_y as i32);
            let bottomTile: i32 = g_map.map(tile_x as i32, tile_y2 as i32);
            let topBlock = g_map.block(tile_x, tile_y);
            let bottomBlock = g_map.block(tile_x, tile_y2);

            if (topTile & tile_flag_solid) != 0
                || (bottomTile & tile_flag_solid) != 0
                || is_solid_block(topBlock)
                || is_solid_block(bottomBlock)
            {
                player.set_xf((((tile_x as i32) << 5) + TILESIZE) as f32 + 0.2f32);
                player.flipsidesifneeded();
                return true;
            }
        }

        false
    }

    fn checkright(&mut self, player: &mut CPlayer) -> bool {
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

        unsafe {
            let topTile: i32 = g_map.map(tile_x as i32, tile_y as i32);
            let bottomTile: i32 = g_map.map(tile_x as i32, tile_y2 as i32);
            let topBlock = g_map.block(tile_x, tile_y);
            let bottomBlock = g_map.block(tile_x, tile_y2);

            if (topTile & tile_flag_solid) != 0
                || (bottomTile & tile_flag_solid) != 0
                || is_solid_block(topBlock)
                || is_solid_block(bottomBlock)
            {
                player.set_xf((((tile_x as i32) << 5) - PW) as f32 - 0.2f32);
                player.flipsidesifneeded();
                return true;
            }
        }

        false
    }

    pub fn checksides(&mut self, player: &mut CPlayer) {
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

        unsafe {
            if player.top_y() >= 0 {
                if (tile_y as i32) < MAPHEIGHT {
                    if tile_x_left >= 0 && (tile_x_left as i32) < MAPWIDTH {
                        let block = g_map.block(tile_x_left, tile_y);

                        if is_solid_block(block) || (g_map.map(tile_x_left as i32, tile_y as i32) & tile_flag_solid) != 0 {
                            iCase |= 0x01;
                        }
                    }

                    if tile_x_right >= 0 && (tile_x_right as i32) < MAPWIDTH {
                        let block = g_map.block(tile_x_right, tile_y);

                        if is_solid_block(block) || (g_map.map(tile_x_right as i32, tile_y as i32) & tile_flag_solid) != 0 {
                            iCase |= 0x02;
                        }
                    }
                }
            }

            // TODO: is this correct?
            if (player.top_y() as i32 + PW) as f32 >= 0.0f32 {
                if (tile_y2 as i32) < MAPHEIGHT {
                    if tile_x_left >= 0 && (tile_x_left as i32) < MAPWIDTH {
                        let block = g_map.block(tile_x_left, tile_y2);

                        if is_solid_block(block) || (g_map.map(tile_x_left as i32, tile_y2 as i32) & tile_flag_solid) != 0 {
                            iCase |= 0x04;
                        }
                    }

                    if tile_x_right >= 0 && (tile_x_right as i32) < MAPWIDTH {
                        let block = g_map.block(tile_x_right, tile_y2);

                        if is_solid_block(block) || (g_map.map(tile_x_right as i32, tile_y2 as i32) & tile_flag_solid) != 0 {
                            iCase |= 0x08;
                        }
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

    fn is_stomping(&mut self, mut player: Ptr<CPlayer>, mut other: Ptr<CPlayer>) -> bool {
        if player.fOldY + PH as f32 <= other.fOldY && player.bottom_y() >= other.top_y() {
            //don't reposition if player is warping when he kills the other player
            if player.isready() {
                player.set_yi((other.iy as i32 - PH) as i16); //set new position to top of other player
                let pp = player;
                player.collisions.checktop(pp.get());
                player.platform = Ptr::null();
            }

            let mut fKillPotential = false;
            if player.vely > 1.0f32 && !other.is_shielded() {
                fKillPotential = true;
            }

            player.bouncejump();

            if fKillPotential {
                let mut style = KillStyle::Stomp;
                if player.flying {
                    style = KillStyle::PWings;
                } else if player.kuriboshoe.is_on() {
                    style = KillStyle::KuriboShoe;
                } else if player.tail.is_in_use() {
                    style = KillStyle::Leaf;
                } else if player.extrajumps > 0 {
                    style = KillStyle::Feather;
                }

                player.killed_player(other, PlayerDeathStyle::Squish, style, false, false);
            } else {
                unsafe {
                    if game_values.gamemode.gamemode == game_mode_tag {
                        player.transfer_tag(other);
                    }

                    if game_values.gamemode.gamemode == game_mode_shyguytag {
                        player.transfer_shy_guy(other);
                    }
                }

                player.iSuicideCreditPlayerID = other.globalID;
                player.iSuicideCreditTimer = 20;
            }

            return true;
        }

        false
    }

    //handles a collision between two players (is being called if o1, o2 collide)
    pub fn handle_p2p(&mut self, mut o1: Ptr<CPlayer>, mut o2: Ptr<CPlayer>) {
        unsafe {
            //If teams tag each other
            if o1.teamID == o2.teamID {
                //Free teammates that are jailed
                if game_values.gamemode.gamemode == game_mode_jail && game_values.gamemodesettings.jail.tagfree {
                    let p1 = o1;
                    o1.jail.free_by_teammate(p1.get());
                    let p2 = o2;
                    o2.jail.free_by_teammate(p2.get());
                }

                //Transfer tag if assist is on
                if (game_values.gamemode.gamemode == game_mode_tag && game_values.teamcollision == TeamCollisionStyle::Assist)
                    || game_values.gamemodesettings.tag.tagontouch
                {
                    o1.transfer_tag(o2);
                }

                //Don't collision detect players on same team if friendly fire is turned off
                if game_values.teamcollision == TeamCollisionStyle::Off {
                    return;
                }

                //Team assist is enabled so allow powerup trading and super jumping
                if game_values.teamcollision == TeamCollisionStyle::Assist {
                    o1.bounce_assist_player(o2);
                    o2.bounce_assist_player(o1);

                    //Allow players on team to swap stored items
                    if o1.playerKeys.game_powerup().fPressed || o2.playerKeys.game_powerup().fPressed {
                        let iTempPowerup: i16 = game_values.gamepowerups[o1.globalID as usize];
                        game_values.gamepowerups[o1.globalID as usize] = game_values.gamepowerups[o2.globalID as usize];
                        game_values.gamepowerups[o2.globalID as usize] = iTempPowerup;

                        if_sound_on_play(&mut rm.sfx_storepowerup);

                        o1.playerKeys.game_powerup_mut().fPressed = false;
                        o1.playerKeys.game_powerup_mut().fDown = false;
                        o2.playerKeys.game_powerup_mut().fPressed = false;
                        o2.playerKeys.game_powerup_mut().fDown = false;
                    }

                    return;
                }
            }

            //Quit checking collision if either player is soft shielded
            if o1.shield.get_type() == SOFT || o2.shield.get_type() == SOFT {
                //Do tag transfer if there is one to do
                if game_values.gamemode.gamemode == game_mode_tag && game_values.gamemodesettings.tag.tagontouch {
                    o1.transfer_tag(o2);
                }

                if game_values.gamemode.gamemode == game_mode_shyguytag && game_values.gamemodesettings.shyguytag.tagtransfer != 1 {
                    o1.transfer_shy_guy(o2);
                }

                return;
            }

            //--- 1. kill frozen players ---
            let mut fFrozenDeath = false;
            if o1.frozen && !o1.is_shielded() && !o1.is_invincible() {
                o2.killed_player(o1, PlayerDeathStyle::Shatter, KillStyle::IceBlast, true, false);
                fFrozenDeath = true;
            }

            if o2.frozen && !o2.is_shielded() && !o2.is_invincible() {
                o1.killed_player(o2, PlayerDeathStyle::Shatter, KillStyle::IceBlast, true, false);
                fFrozenDeath = true;
            }

            if fFrozenDeath {
                return;
            }

            //--- 2. is player invincible? ---
            if o1.is_invincible() && !o2.is_shielded() && !o2.is_invincible() {
                o1.killed_player(o2, PlayerDeathStyle::Jump, KillStyle::Star, false, false);
                return;
            }

            if o2.is_invincible() && !o1.is_shielded() && !o1.is_invincible() {
                o2.killed_player(o1, PlayerDeathStyle::Jump, KillStyle::Star, false, false);
                return;
            }

            //If both players are warping, ignore collision
            if o1.iswarping() && o2.iswarping() {
                return;
            }

            //--- 3. stomping other player? ---
            if (o2.shield.get_type() == OFF || o2.shield.get_type() == HARD) && !o2.is_invincible() && self.is_stomping(o1, o2) {
                return;
            }
            if (o1.shield.get_type() == OFF || o1.shield.get_type() == HARD) && !o1.is_invincible() && self.is_stomping(o2, o1) {
                return;
            }

            //Quit checking collision if either player is hard shielded
            if o1.shield.get_type() == SOFT_WITH_STOMP || o2.shield.get_type() == SOFT_WITH_STOMP {
                //Do tag transfer if there is one to do
                if game_values.gamemode.gamemode == game_mode_tag && game_values.gamemodesettings.tag.tagontouch {
                    o1.transfer_tag(o2);
                }

                if game_values.gamemode.gamemode == game_mode_shyguytag && game_values.gamemodesettings.shyguytag.tagtransfer != 1 {
                    o1.transfer_shy_guy(o2);
                }

                return;
            }

            //--- 4. push back (horizontal) ---
            if o1.ix < o2.ix {
                //o1 is left -> o1 pushback left, o2 pushback right
                self.handle_p2p_pushback(o1, o2);
            } else {
                self.handle_p2p_pushback(o2, o1);
            }
        }
    }

    //handles a collision between a player and an object
    pub fn handle_p2o(&mut self, o1: Ptr<CPlayer>, mut o2: Ptr<dyn CObjectTrait>) -> bool {
        o2.collide_player(o1)
    }

    //calculates the new positions for both players when they are pushing each other
    fn handle_p2p_pushback(&mut self, mut o1: Ptr<CPlayer>, mut o2: Ptr<CPlayer>) {
        //o1 is left to o2
        //  |o1||o2|
        //-----------

        let p1 = o1;
        let p2 = o2;

        unsafe {
            //Transfer tag on touching other players
            if game_values.gamemode.gamemode == game_mode_tag && game_values.gamemodesettings.tag.tagontouch {
                o1.transfer_tag(o2);
            }

            if game_values.gamemode.gamemode == game_mode_shyguytag && game_values.gamemodesettings.shyguytag.tagtransfer != 1 {
                o1.transfer_shy_guy(o2);
            }
        }

        let mut overlapcollision = false;
        if (o1.ix as i32 + PW) < 320 && o2.ix > 320 {
            overlapcollision = true;
        }

        if o1.iswarping() {
            if overlapcollision {
                //o2 reposition to the right side of o1, o1 stays
                o2.set_xi((o1.ix as i32 - PW + App::screenWidth - 1) as i16);
                o2.collisions.checkleft(p2.get());
            } else {
                o2.set_xi((o1.ix as i32 + PW + 1) as i16);
                o2.collisions.checkright(p2.get());
            }

            return;
        } else if o2.iswarping() {
            if overlapcollision {
                //o1 reposition to the left side of o2, o2 stays
                o1.set_xi((o2.ix as i32 + PW - App::screenWidth - 1) as i16);
                o1.collisions.checkright(p1.get());
            } else {
                o1.set_xi((o2.ix as i32 - PW - 1) as i16);
                o1.collisions.checkleft(p1.get());
            }

            return;
        } else if !o1.iswarping() && !o2.iswarping() {
            //both objects moving - calculate middle and set both objects

            if overlapcollision {
                let middle: i16 = (o2.ix as i32 - App::screenWidth + ((o1.ix as i32 + PW) - o2.ix as i32 - App::screenWidth) / 2) as i16; //no ABS needed (o1->x < o2->x -> o1->x+w > o2->x !)
                o1.set_xi((middle as i32 + 1) as i16); //o1 is left
                o2.set_xi((middle as i32 - PW + App::screenWidth - 1) as i16); //o2 is right

                o1.collisions.checkright(p1.get());
                o2.collisions.checkleft(p2.get());
            } else {
                let middle: i16 = (o2.ix as i32 + ((o1.ix as i32 + PW) - o2.ix as i32) / 2) as i16; //no ABS needed (o1->x < o2->x -> o1->x+w > o2->x !)
                o1.set_xi((middle as i32 - PW - 1) as i16); //o1 is left
                o2.set_xi((middle as i32 + 1) as i16); //o2 is right

                o1.collisions.checkleft(p1.get());
                o2.collisions.checkright(p2.get());
            }
        }

        let absv1: f32;
        let absv2: f32;

        let mut dPlayer1Pushback: f32 = 1.5f32;
        let mut dPlayer2Pushback: f32 = 1.5f32;

        if o1.kuriboshoe.is_on() && !o2.kuriboshoe.is_on() {
            dPlayer1Pushback = 0.5f32;
            dPlayer2Pushback = 2.5f32;
        } else if !o1.kuriboshoe.is_on() && o2.kuriboshoe.is_on() {
            dPlayer1Pushback = 2.5f32;
            dPlayer2Pushback = 0.5f32;
        }

        if overlapcollision {
            absv1 = (if o1.velx < 0.0f32 { o1.velx } else { -1.0f32 }) * dPlayer2Pushback; //o1 is on the left side (only positive velx counts)
            absv2 = (if o2.velx > 0.0f32 { o2.velx } else { 1.0f32 }) * dPlayer1Pushback; //o2 right (only negative velx counts)
        } else {
            absv1 = (if o1.velx > 0.0f32 { o1.velx } else { 1.0f32 }) * dPlayer2Pushback; //o1 is on the left side (only positive velx counts)
            absv2 = (if o2.velx < 0.0f32 { o2.velx } else { -1.0f32 }) * dPlayer1Pushback; //o2 right (only negative velx counts)
        }

        if o1.state == PlayerState::Ready {
            o1.velx = cap_side_velocity(absv2);
            o1.iSuicideCreditPlayerID = o2.globalID;
            o1.iSuicideCreditTimer = 62;
        }

        if o2.state == PlayerState::Ready {
            o2.velx = cap_side_velocity(absv1);
            o2.iSuicideCreditPlayerID = o1.globalID;
            o2.iSuicideCreditTimer = 62;
        }
    }
}
