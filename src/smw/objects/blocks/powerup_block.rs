//! Port of src/smw/objects/blocks/PowerupBlock.cpp

use crate::common::game_mode::*;
use crate::common::game_values::{game_values, if_sound_on_play};
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::objectgame::createpowerup;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class powerup block
//------------------------------------------------------------------------------
pub struct B_PowerupBlock {
    pub io_block: IO_Block,

    pub iCountWeight: i16,

    pub timer: i16,
    pub side: bool,
    pub iNumSprites: i16,
    pub animationSpeed: i16,
    pub drawFrame: i16,
    pub animationTimer: i16,
    pub animationWidth: i16,

    pub settings: [i16; NUM_POWERUPS as usize],
}
impl_base!(B_PowerupBlock => io_block: IO_Block);

impl B_PowerupBlock {
    /// `piSettings` must hold at least `NUM_POWERUPS` entries (`objdata.iSettings`).
    pub fn new(nspr1: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, fHidden: bool, piSettings: &[i16]) -> Self {
        let mut b = B_PowerupBlock {
            io_block: IO_Block::new(nspr1, pos),
            iCountWeight: 0,
            timer: 0,
            side: false,
            iNumSprites: 0,
            animationSpeed: 0,
            drawFrame: 0,
            animationTimer: 0,
            animationWidth: 0,
            settings: [0; NUM_POWERUPS as usize],
        };

        b.iw = (b.spr.get_width() as i16) >> 2;
        b.ih = (b.spr.get_height() as i16) >> 1; //This sprite has two images (unused and used blocks)
        b.collisionWidth = b.iw;
        b.timer = 0;
        b.side = true;
        b.iNumSprites = iNumSpr;
        b.animationSpeed = aniSpeed;
        b.animationTimer = 0;
        b.animationWidth = b.spr.get_width() as i16;
        b.drawFrame = 0;

        b.hidden = fHidden;
        b.ishiddentype = fHidden;

        unsafe {
            let n = NUM_POWERUPS as usize;
            if piSettings[0] == -1 || game_values.overridepowerupsettings == 1 {
                //Game Only
                for iSetting in 0..n {
                    b.settings[iSetting] = game_values.powerupweights[iSetting];
                }
            } else if game_values.overridepowerupsettings == 0 {
                //Map Only
                for iSetting in 0..n {
                    b.settings[iSetting] = piSettings[iSetting];
                }
            } else if game_values.overridepowerupsettings == 2 {
                //Average
                for iSetting in 0..n {
                    b.settings[iSetting] = (piSettings[iSetting] as i32 + game_values.powerupweights[iSetting] as i32) as i16;
                }
            } else if game_values.overridepowerupsettings == 3 {
                //Weighted
                let mut dMapWeightCount: f32 = 0.0;
                for iPowerup in 0..n {
                    dMapWeightCount += piSettings[iPowerup] as f32;
                }

                let mut dGameWeightCount: f32 = 0.0;
                for iPowerup in 0..n {
                    dGameWeightCount += game_values.powerupweights[iPowerup] as f32;
                }

                for iSetting in 0..n {
                    let dWeight: f32 = (piSettings[iSetting] as f32 / dMapWeightCount + game_values.powerupweights[iSetting] as f32 / dGameWeightCount) * 100.0f32;

                    //Cap lowest value at 1
                    if dWeight < 1.0f32 && dWeight > 0.0f32 {
                        b.settings[iSetting] = 1;
                    } else {
                        b.settings[iSetting] = dWeight as i16;
                    }
                }
            }
        }

        b.iCountWeight = 0;
        for iPowerup in 0..NUM_POWERUPS as usize {
            b.iCountWeight = (b.iCountWeight as i32 + b.settings[iPowerup] as i32) as i16;
        }
        b
    }
}

/// Virtual interface added by `B_PowerupBlock` (`SelectPowerup`).
pub trait B_PowerupBlockTrait: IO_BlockTrait {
    fn pb(&self) -> &B_PowerupBlock;
    fn pb_mut(&mut self) -> &mut B_PowerupBlock;

    fn select_powerup(&mut self) -> i16 {
        b_powerup_block_select_powerup(self)
    }
}

/// Implements `pb`, `pb_mut` for a type that derefs to `B_PowerupBlock`.
#[macro_export]
macro_rules! impl_powerup_block_plumbing {
    () => {
        fn pb(&self) -> &$crate::smw::objects::blocks::powerup_block::B_PowerupBlock {
            self
        }
        fn pb_mut(&mut self) -> &mut $crate::smw::objects::blocks::powerup_block::B_PowerupBlock {
            self
        }
    };
}

pub fn b_powerup_block_draw<T: B_PowerupBlockTrait + ?Sized>(this: &mut T) {
    let b = this.pb();
    if !b.hidden {
        b.spr.draw_src(
            b.ix as i32,
            b.iy as i32,
            &SDL_Rect { x: b.drawFrame as i32, y: if b.state == 0 { 0 } else { b.ih as i32 }, w: b.iw as i32, h: b.ih as i32 },
        );
    }
}

pub fn b_powerup_block_update<T: B_PowerupBlockTrait + ?Sized>(this: &mut T) {
    io_block_update(this);

    unsafe {
        if this.pb().state > 0 {
            {
                let b = this.pb_mut();
                let fy = b.fy + b.vely;
                b.set_yf(fy);
            }

            let state = this.pb().state;
            let dy = (this.pb().fposy - this.pb().fy).abs();
            if state == 1 && dy > 10.0f32 {
                let b = this.pb_mut();
                b.vely = -b.vely;
                b.state = 2;
                b.iBumpPlayerID = -1;
            } else if state == 2 && dy < VELBLOCKBOUNCE {
                let (pos, side) = {
                    let b = this.pb_mut();
                    b.vely = 0.0f32;
                    b.state = 3;
                    let iposy = b.iposy;
                    b.set_yi(iposy);

                    (Vec2s::new(b.ix, b.iy), b.side)
                };

                let gm = game_values.gamemode.gamemode;
                let gs = &game_values.gamemodesettings;
                if gm == game_mode_health && RANDOM_INT(100) < gs.health.percentextralife as i32 {
                    createpowerup(HEALTH_POWERUP as i16, pos, side, true);
                } else if (gm == game_mode_timelimit && RANDOM_INT(100) < gs.time.percentextratime as i32)
                    || (gm == game_mode_star && RANDOM_INT(100) < gs.star.percentextratime as i32)
                {
                    createpowerup(TIME_POWERUP as i16, pos, side, true);
                } else if (gm == game_mode_coins && RANDOM_INT(100) < gs.coins.percentextracoin as i32)
                    || (gm == game_mode_greed && RANDOM_INT(100) < gs.greed.percentextracoin as i32)
                {
                    createpowerup(COIN_POWERUP as i16, pos, side, true);
                } else if gm == game_mode_jail && RANDOM_INT(100) < gs.jail.percentkey as i32 {
                    createpowerup(JAIL_KEY_POWERUP as i16, pos, side, true);
                } else {
                    let iType = this.select_powerup();
                    createpowerup(iType, pos, side, true);
                }

                if_sound_on_play(&mut rm.sfx_sprout);
            } else if state == 3 {
                if game_values.itemrespawntime > 0 && {
                    let b = this.pb_mut();
                    b.timer += 1;
                    b.timer >= game_values.itemrespawntime
                } {
                    this.reset();
                }
            }
        }
    }

    let b = this.pb_mut();
    b.animationTimer += 1;
    if b.animationTimer >= b.animationSpeed {
        b.animationTimer = 0;

        b.drawFrame += b.iw;
        if b.drawFrame >= b.animationWidth {
            b.drawFrame = 0;
        }
    }
}

pub fn b_powerup_block_reset<T: B_PowerupBlockTrait + ?Sized>(this: &mut T) {
    let b = this.pb_mut();
    b.timer = 0;
    b.state = 0;
}

pub fn b_powerup_block_collide_player_dir<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
    if this.pb().hidden {
        let b = this.pb();
        if player.fOldY >= (b.iposy as i32 + b.ih as i32) as f32 && direction == 0 {
            return this.hitbottom_player(player, useBehavior);
        }

        return true;
    }

    io_block_collide_player_dir(this, player, direction, useBehavior)
}

pub fn b_powerup_block_hittop_player<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
    io_block_hittop_player(this, player, useBehavior);

    unsafe {
        if this.pb().state == 1 {
            let mut iKillType = PlayerKillType::NonKill;
            let (iBumpPlayerID, iBumpTeamID) = (this.pb().iBumpPlayerID, this.pb().iBumpTeamID);
            if iBumpPlayerID >= 0
                && !player.is_invincible_on_bottom()
                && (player.teamID != iBumpTeamID || game_values.teamcollision == TeamCollisionStyle::On)
            {
                iKillType = player_killed_player(iBumpPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Bounce, false, false);
            }

            if PlayerKillType::NonKill == iKillType {
                player.vely = -VELNOTEBLOCKREPEL;
            }
        } else if useBehavior {
            player.vely = GRAVITATION;

            if this.pb().state == 0 {
                if player.is_super_stomping() {
                    let b = this.pb_mut();
                    b.state = 1;
                    b.vely = -VELBLOCKBOUNCE;
                    b.side = (player.ix as i32 + HALFPW) < (b.ix as i32 + (b.iw as i32 >> 1));
                }
            }
        }
    }

    false
}

pub fn b_powerup_block_hitbottom_player<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
    if useBehavior {
        unsafe {
            //Player bounces off
            player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
            {
                let b = this.pb();
                player.set_yf((b.iposy as i32 + b.ih as i32) as f32 + 0.2f32);
            }

            if this.pb().hidden {
                this.pb_mut().hidden = false;
                io_block_kill_players_and_objects_inside_block(this, player.globalID);
            }

            let (col, row) = (this.pb().col, this.pb().row);
            g_map.update_tile_gap(col, row);

            if this.pb().state == 0 {
                if_sound_on_play(&mut rm.sfx_bump);

                let b = this.pb_mut();
                b.iBumpPlayerID = player.globalID;
                b.iBumpTeamID = player.teamID;

                b.vely = -VELBLOCKBOUNCE;
                b.state = 1;
                b.side = (player.ix as i32 + HALFPW) < (b.ix as i32 + (b.iw as i32 >> 1));
            }
        }
    }

    false
}

pub fn b_powerup_block_collide_object_dir<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
    if this.pb().hidden {
        return true;
    }

    io_block_collide_object_dir(this, object, direction)
}

pub fn b_powerup_block_hittop_object<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let iposy = this.pb().iposy;
    let ch = object.collisionHeight;
    object.set_yf((iposy as i32 - ch as i32) as f32 - 0.2f32);
    object.fOldY = object.fy;

    let r#type = object.get_moving_object_type();
    if r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
        if this.pb().state == 0 {
            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);
            }

            let b = this.pb_mut();
            b.iBumpPlayerID = -1;
            b.vely = -VELBLOCKBOUNCE;
            b.state = 1;
            b.side = false;
        }

        object.vely = object.bottom_bounce();
    } else if this.pb().state == 1 && object.bounce == GRAVITATION {
        io_block_bounce_moving_object(this, object);
        return false;
    } else {
        object.vely = object.bottom_bounce();
    }

    true
}

pub fn b_powerup_block_hitright_object<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    //Object bounces off
    {
        let b = this.pb();
        object.set_xf((b.iposx as i32 + b.iw as i32) as f32 + 0.2f32);
    }
    object.fOldX = object.fx;

    if object.velx < 0.0f32 {
        object.velx = -object.velx;
    }

    let r#type = object.get_moving_object_type();
    if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox || r#type == movingobject_attackzone {
        if r#type == movingobject_shell {
            if object.state != 1 {
                return false;
            }
        }

        //If the box isn't moving fast enough, then don't do anything
        if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
            return false;
        }

        if this.pb().state == 0 {
            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);
            }

            let b = this.pb_mut();
            b.iBumpPlayerID = -1;
            b.vely = -VELBLOCKBOUNCE;
            b.state = 1;
            b.side = false;
        }

        return true;
    }

    false
}

pub fn b_powerup_block_hitleft_object<T: B_PowerupBlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    //Object bounces off
    let iposx = this.pb().iposx;
    let cw = object.collisionWidth;
    object.set_xf((iposx as i32 - cw as i32) as f32 - 0.2f32);
    object.fOldX = object.fx;

    if object.velx > 0.0f32 {
        object.velx = -object.velx;
    }

    let r#type = object.get_moving_object_type();
    if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox || r#type == movingobject_attackzone {
        if r#type == movingobject_shell {
            if object.state != 1 {
                return false;
            }
        }

        //If the box isn't moving fast enough, then don't do anything
        if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
            return false;
        }

        if this.pb().state == 0 {
            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);
            }

            let b = this.pb_mut();
            b.iBumpPlayerID = -1;
            b.vely = -VELBLOCKBOUNCE;
            b.state = 1;
            b.side = true;
        }

        return true;
    }

    false
}

pub fn b_powerup_block_trigger_behavior<T: B_PowerupBlockTrait + ?Sized>(this: &mut T) {
    if this.pb().state == 0 {
        unsafe {
            if_sound_on_play(&mut rm.sfx_bump);
        }

        let b = this.pb_mut();
        b.iBumpPlayerID = -1;
        b.vely = -VELBLOCKBOUNCE;
        b.state = 1;
        b.side = true;
    }
}

pub fn b_powerup_block_select_powerup<T: B_PowerupBlockTrait + ?Sized>(this: &mut T) -> i16 {
    let b = this.pb();
    if b.iCountWeight == 0 {
        return NO_POWERUP as i16;
    }

    let iRandPowerup: i32 = RANDOM_INT(b.iCountWeight as i32) + 1;
    let mut iSelectedPowerup: i32 = 0;

    let mut iPowerupWeightCount: i32 = b.settings[iSelectedPowerup as usize] as i32;

    while iPowerupWeightCount < iRandPowerup {
        iSelectedPowerup += 1;
        iPowerupWeightCount += b.settings[iSelectedPowerup as usize] as i32;
    }

    iSelectedPowerup as i16
}

/// Forwards the `IO_Block` virtuals that `B_PowerupBlock` overrides; shared by `B_PowerupBlock` and `B_ViewBlock`.
#[macro_export]
macro_rules! impl_powerup_block_io_block_overrides {
    () => {
        fn reset(&mut self) {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_reset(self)
        }
        fn collide_player_dir(&mut self, player: $crate::globals::Ptr<$crate::smw::player::CPlayer>, direction: i16, useBehavior: bool) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_collide_player_dir(self, player, direction, useBehavior)
        }
        fn collide_object_dir(
            &mut self,
            object: $crate::globals::Ptr<dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait>,
            direction: i16,
        ) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_collide_object_dir(self, object, direction)
        }
        fn hittop_player(&mut self, player: $crate::globals::Ptr<$crate::smw::player::CPlayer>, useBehavior: bool) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_hittop_player(self, player, useBehavior)
        }
        fn hitbottom_player(&mut self, player: $crate::globals::Ptr<$crate::smw::player::CPlayer>, useBehavior: bool) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_hitbottom_player(self, player, useBehavior)
        }
        fn hittop_object(&mut self, object: $crate::globals::Ptr<dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait>) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_hittop_object(self, object)
        }
        fn hitright_object(&mut self, object: $crate::globals::Ptr<dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait>) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_hitright_object(self, object)
        }
        fn hitleft_object(&mut self, object: $crate::globals::Ptr<dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait>) -> bool {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_hitleft_object(self, object)
        }
        fn trigger_behavior(&mut self) {
            $crate::smw::objects::blocks::powerup_block::b_powerup_block_trigger_behavior(self)
        }
    };
}

impl CObjectTrait for B_PowerupBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }
    fn draw(&mut self) {
        b_powerup_block_draw(self)
    }
    fn update(&mut self) {
        b_powerup_block_update(self)
    }
}

impl IO_BlockTrait for B_PowerupBlock {
    crate::impl_io_block_plumbing!();
    crate::impl_powerup_block_io_block_overrides!();
}

impl B_PowerupBlockTrait for B_PowerupBlock {
    crate::impl_powerup_block_plumbing!();
}
