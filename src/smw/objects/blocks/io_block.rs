//! Port of src/smw/objects/blocks/IO_Block.cpp

use crate::common::game::App;
use crate::common::game_values::{game_values, if_sound_on_play};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, object_block, CObject, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::{rm, Ptr};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{get_player_from_global_id, CPlayer};

pub fn io_block_new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> IO_Block {
    let cobject = CObject::new(nspr, pos);
    let mut b = IO_Block {
        iBumpPlayerID: 0,
        iBumpTeamID: 0,
        fposx: 0.0,
        fposy: 0.0,
        iposx: 0,
        iposy: 0,
        hidden: false,
        ishiddentype: false,
        iHiddenTimer: 0,
        col: 0,
        row: 0,
        cobject,
    };
    b.objectType = object_block;

    b.iBumpPlayerID = -1;
    b.iBumpTeamID = -1;

    b.fposx = b.fx;
    b.fposy = b.fy;

    b.iposx = pos.x;
    b.iposy = pos.y;

    b.col = (pos.x as i32 / TILESIZE) as i16;
    b.row = (pos.y as i32 / TILESIZE) as i16;

    b.hidden = false;
    b.ishiddentype = false;
    b.iHiddenTimer = 0;
    b
}

pub fn io_block_draw<T: IO_BlockTrait + ?Sized>(this: &mut T) {
    let b = this.block();
    b.spr.draw(b.ix as i32, b.iy as i32);
}

pub fn io_block_update<T: IO_BlockTrait + ?Sized>(this: &mut T) {
    unsafe {
        let b = this.block_mut();
        if b.ishiddentype && !b.hidden {
            if game_values.hiddenblockrespawn > 0 && {
                b.iHiddenTimer += 1;
                b.iHiddenTimer > game_values.hiddenblockrespawn
            } {
                b.iHiddenTimer = 0;
                b.hidden = true;
                this.reset();

                let b = this.block();
                g_map.update_tile_gap(b.col, b.row);
            }
        }
    }
}

pub fn io_block_reset<T: IO_BlockTrait + ?Sized>(this: &mut T) {
    this.block_mut().state = 0;
}

pub fn io_block_collide_player_dir<T: IO_BlockTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
    if direction == 2 {
        this.hittop_player(player, useBehavior)
    } else if direction == 0 {
        this.hitbottom_player(player, useBehavior)
    } else if direction == 1 {
        this.hitleft_player(player, useBehavior)
    } else {
        this.hitright_player(player, useBehavior)
    }
}

pub fn io_block_hittop_player<T: IO_BlockTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
    if useBehavior {
        let iposy = this.block().iposy;
        player.set_yf((iposy as i32 - PH) as f32 - 0.2);
        player.inair = false;
        player.fallthrough = false;
        player.killsinrowinair = 0;
        player.extrajumps = 0;
        player.vely = GRAVITATION;
    }

    false
}

pub fn io_block_hitbottom_player<T: IO_BlockTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
    if useBehavior {
        player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
        let b = this.block();
        player.set_yf((b.iposy as i32 + b.ih as i32) as f32 + 0.2);
    }

    false
}

pub fn io_block_hitright_player<T: IO_BlockTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
    if useBehavior {
        let b = this.block();
        player.set_xf((b.iposx as i32 + b.iw as i32) as f32 + 0.2);
        player.fOldX = player.fx;

        if player.velx < 0.0 {
            player.velx = 0.0;
        }

        if player.oldvelx < 0.0 {
            player.oldvelx = 0.0;
        }
    }

    false
}

pub fn io_block_hitleft_player<T: IO_BlockTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
    if useBehavior {
        let b = this.block();
        player.set_xf((b.iposx as i32 - PW) as f32 - 0.2);
        player.fOldX = player.fx;

        if player.velx > 0.0 {
            player.velx = 0.0;
        }

        if player.oldvelx > 0.0 {
            player.oldvelx = 0.0;
        }
    }

    false
}

pub fn io_block_collide_object_dir<T: IO_BlockTrait + ?Sized>(this: &mut T, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
    if direction == 2 {
        this.hittop_object(object)
    } else if direction == 0 {
        this.hitbottom_object(object)
    } else if direction == 1 {
        this.hitleft_object(object)
    } else {
        this.hitright_object(object)
    }
}

pub fn io_block_hittop_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let iposy = this.block().iposy;
    let ch = object.collisionHeight;
    object.set_yf((iposy as i32 - ch as i32) as f32 - 0.2);
    object.fOldY = object.fy;
    object.vely = object.bottom_bounce();
    true
}

pub fn io_block_hitbottom_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let b = this.block();
    object.set_yf((b.iposy as i32 + b.ih as i32) as f32 + 0.2);
    object.fOldY = object.fy;
    object.vely = -object.vely;
    true
}

pub fn io_block_hitright_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let b = this.block();
    object.set_xf((b.iposx as i32 + b.iw as i32) as f32 + 0.2);
    object.fOldX = object.fx;

    if object.velx < 0.0 {
        object.velx = -object.velx;
    }

    true
}

pub fn io_block_hitleft_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let iposx = this.block().iposx;
    let cw = object.collisionWidth;
    object.set_xf((iposx as i32 - cw as i32) as f32 - 0.2);
    object.fOldX = object.fx;

    if object.velx > 0.0 {
        object.velx = -object.velx;
    }

    true
}

pub fn io_block_bounce_moving_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) {
    unsafe {
        let r#type = object.get_moving_object_type();
        if r#type == movingobject_goomba || r#type == movingobject_koopa || r#type == movingobject_buzzybeetle || r#type == movingobject_spiny {
            if_sound_on_play(&mut rm.sfx_kicksound);

            let enemy = object.as_walking_enemy().unwrap();
            let style = enemy.we().get_kill_style();

            enemy.die_and_drop_shell(true, true);

            let iBumpPlayerID = this.block().iBumpPlayerID;
            if !game_values.gamemode.gameover && iBumpPlayerID >= 0 {
                let mut player = get_player_from_global_id(iBumpPlayerID);

                if !player.is_null() {
                    player.add_killer_award(Ptr::null(), style);
                    player.score.adjust_score(1);
                }
            }
        } else if r#type == movingobject_shell {
            let shell = object.as_any().downcast_mut::<CO_Shell>().unwrap();
            shell.flip();
        } else if r#type == movingobject_throwblock {
            object.die();
        } else if r#type == movingobject_throwbox {
            object.die();
        } else {
            object.vely = -VELNOTEBLOCKREPEL;
        }
    }
}

pub fn io_block_kill_players_and_objects_inside_block<T: IO_BlockTrait + ?Sized>(this: &mut T, playerID: i16) {
    unsafe {
        let iposx = this.block().iposx;
        let iposy = this.block().iposy;

        //Loop through players
        for i in 0..players.len() {
            let mut player = players[i];
            if !player.isready() {
                continue;
            }

            let mut iSwapSides: i16 = 0;
            if player.fOldX >= (iposx as i32 + TILESIZE) as f32 {
                iSwapSides = -App::screenWidth as i16;
            }

            if player.fOldX + PW as f32 + iSwapSides as f32 >= iposx as f32
                && player.fOldX + (iSwapSides as f32) < (iposx as i32 + TILESIZE) as f32
                && player.fOldY + PH as f32 >= iposy as f32
                && player.fOldY < (iposy as i32 + TILESIZE) as f32
            {
                player.iSuicideCreditPlayerID = playerID;
                player.iSuicideCreditTimer = 1;
                player.kill_player_map_hazard(true, KillStyle::Environment, true, -1);
            }
        }

        //Loop through objects
        for iLayer in 0..3usize {
            let count = objectcontainer[iLayer].list().len();
            for i in 0..count {
                let mut obj: Ptr<dyn CObjectTrait> = objectcontainer[iLayer].list()[i];
                let movingobject: &mut dyn IO_MovingObjectTrait = match obj.as_io_moving_object() {
                    Some(m) => m,
                    None => continue,
                };
                if !movingobject.mo().collides_with_map() {
                    continue;
                }

                let fOldX = movingobject.mo().fOldX;
                let fOldY = movingobject.mo().fOldY;

                let mut iSwapSides: i16 = 0;
                if fOldX >= (iposx as i32 + TILESIZE) as f32 {
                    iSwapSides = -App::screenWidth as i16;
                }

                if fOldX + PW as f32 + iSwapSides as f32 >= iposx as f32
                    && fOldX + (iSwapSides as f32) < (iposx as i32 + TILESIZE) as f32
                    && fOldY + PH as f32 >= iposy as f32
                    && fOldY < (iposy as i32 + TILESIZE) as f32
                {
                    movingobject.kill_object_map_hazard(playerID);
                }
            }
        }
    }
}

/// Implements `block`, `block_mut`, `as_block_ptr` for a type that derefs to `IO_Block`.
#[macro_export]
macro_rules! impl_io_block_plumbing {
    () => {
        fn block(&self) -> &$crate::common::io_block::IO_Block {
            self
        }
        fn block_mut(&mut self) -> &mut $crate::common::io_block::IO_Block {
            self
        }
        fn as_block_ptr(&mut self) -> $crate::globals::Ptr<dyn $crate::common::io_block::IO_BlockTrait> {
            $crate::globals::Ptr::from_mut(self as &mut dyn $crate::common::io_block::IO_BlockTrait)
        }
    };
}
