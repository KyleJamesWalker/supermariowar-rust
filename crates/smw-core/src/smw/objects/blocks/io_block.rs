//! Default block behavior shared by every block: drawing, hidden-block respawn, and collision by side.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, object_block, CObject, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::Ptr;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{get_player_from_global_id, CPlayer};

/// Builds the shared block state at `pos` (pixels) and records its tile column and row.
pub fn io_block_new(g: &mut Game, nspr: SpriteId, pos: Vec2s) -> IO_Block {
    let mut gp = g.ptr();
    let cobject = CObject::new(gp.get(), nspr, pos);
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

/// Draws the whole block sprite.
pub fn io_block_draw<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let b = this.block();
    g.rm.sprite(b.spr).draw(g, b.ix as i32, b.iy as i32);
}

/// Hides a revealed hidden block again after the hidden-block respawn setting.
pub fn io_block_update<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let b = this.block_mut();
    if b.ishiddentype && !b.hidden {
        if g.game_values.hiddenblockrespawn > 0 && {
            b.iHiddenTimer += 1;
            b.iHiddenTimer > g.game_values.hiddenblockrespawn
        } {
            b.iHiddenTimer = 0;
            b.hidden = true;
            this.reset();

            let b = this.block();
            g.update_tile_gap(b.col, b.row);
        }
    }
}

/// Returns the block to its idle state.
pub fn io_block_reset<T: IO_BlockTrait + ?Sized>(this: &mut T) {
    this.block_mut().state = 0;
}

/// Routes a player hit by side: 0 from below, 1 into the left side, 2 onto the top, 3 into the right side.
pub fn io_block_collide_player_dir<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId, direction: i16, useBehavior: bool) -> bool {
    if direction == 2 {
        this.hittop_player(g, player, useBehavior)
    } else if direction == 0 {
        this.hitbottom_player(g, player, useBehavior)
    } else if direction == 1 {
        this.hitleft_player(g, player, useBehavior)
    } else {
        this.hitright_player(g, player, useBehavior)
    }
}

/// With `useBehavior`, lands the player on top of the block.
pub fn io_block_hittop_player<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, useBehavior: bool) -> bool {
    if useBehavior {
        let iposy = this.block().iposy;
        g.player(player).set_yf((iposy as i32 - PH) as f32 - 0.2);
        g.player(player).inair = false;
        g.player(player).fallthrough = false;
        g.player(player).killsinrowinair = 0;
        let gid = g.player(player).globalID;
        crate::smw::stats::touched(g, gid);
        g.player(player).extrajumps = 0;
        g.player(player).vely = GRAVITATION;
    }

    false
}

/// With `useBehavior`, bounces the player down off the block's underside.
pub fn io_block_hitbottom_player<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, useBehavior: bool) -> bool {
    if useBehavior {
        g.player(player).vely = cap_falling_velocity(-g.player(player).vely * BOUNCESTRENGTH);
        let b = this.block();
        g.player(player).set_yf((b.iposy as i32 + b.ih as i32) as f32 + 0.2);
    }

    false
}

/// With `useBehavior`, stops the player against the block's right side.
pub fn io_block_hitright_player<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, useBehavior: bool) -> bool {
    if useBehavior {
        let b = this.block();
        g.player(player).set_xf((b.iposx as i32 + b.iw as i32) as f32 + 0.2);
        g.player(player).fOldX = g.player(player).fx;

        if g.player(player).velx < 0.0 {
            g.player(player).velx = 0.0;
        }

        if g.player(player).oldvelx < 0.0 {
            g.player(player).oldvelx = 0.0;
        }
    }

    false
}

/// With `useBehavior`, stops the player against the block's left side.
pub fn io_block_hitleft_player<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, useBehavior: bool) -> bool {
    if useBehavior {
        let b = this.block();
        g.player(player).set_xf((b.iposx as i32 - PW) as f32 - 0.2);
        g.player(player).fOldX = g.player(player).fx;

        if g.player(player).velx > 0.0 {
            g.player(player).velx = 0.0;
        }

        if g.player(player).oldvelx > 0.0 {
            g.player(player).oldvelx = 0.0;
        }
    }

    false
}

/// Routes an object hit by side, numbered as in [`io_block_collide_player_dir`].
pub fn io_block_collide_object_dir<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
    if direction == 2 {
        this.hittop_object(g, object)
    } else if direction == 0 {
        this.hitbottom_object(g, object)
    } else if direction == 1 {
        this.hitleft_object(g, object)
    } else {
        this.hitright_object(g, object)
    }
}

/// Lands the object on the block with its own landing bounce.
pub fn io_block_hittop_object<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let iposy = this.block().iposy;
    let ch = object.collisionHeight;
    object.set_yf((iposy as i32 - ch as i32) as f32 - 0.2);
    object.fOldY = object.fy;
    object.vely = object.bottom_bounce(g);
    true
}

/// Pushes the object below the block and reverses its vertical speed.
pub fn io_block_hitbottom_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let b = this.block();
    object.set_yf((b.iposy as i32 + b.ih as i32) as f32 + 0.2);
    object.fOldY = object.fy;
    object.vely = -object.vely;
    true
}

/// Pushes the object out of the right side and sends it right.
pub fn io_block_hitright_object<T: IO_BlockTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
    let b = this.block();
    object.set_xf((b.iposx as i32 + b.iw as i32) as f32 + 0.2);
    object.fOldX = object.fx;

    if object.velx < 0.0 {
        object.velx = -object.velx;
    }

    true
}

/// Pushes the object out of the left side and sends it left.
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

/// Bumps an object standing on a block hit from below: enemies die leaving a flipped shell, shells flip, thrown blocks break, others hop.
pub fn io_block_bounce_moving_object<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut gp = g.ptr();
    let r#type = object.get_moving_object_type();
    if r#type == movingobject_goomba || r#type == movingobject_koopa || r#type == movingobject_buzzybeetle || r#type == movingobject_spiny {
        if_sound_on_play(g, &mut gp.rm.sfx_kicksound);

        let enemy = object.as_walking_enemy().unwrap();
        let style = enemy.we().get_kill_style();

        enemy.die_and_drop_shell(g, true, true);

        let iBumpPlayerID = this.block().iBumpPlayerID;
        if !g.mode().gameover && iBumpPlayerID >= 0 {
            let mut player = get_player_from_global_id(g, iBumpPlayerID);

            if !player.is_none() {
                g.player(player).add_killer_award(g, PlayerId::NONE, style);
                g.player(player).score(g).adjust_score(g, 1);
            }
        }
    } else if r#type == movingobject_shell {
        let shell = object.as_any().downcast_mut::<CO_Shell>().unwrap();
        shell.flip(g);
    } else if r#type == movingobject_throwblock {
        object.die(g);
    } else if r#type == movingobject_throwbox {
        object.die(g);
    } else {
        object.vely = -VELNOTEBLOCKREPEL;
    }
}

/// Kills players and map-colliding objects overlapping this tile, crediting `playerID`; called when the block turns solid.
pub fn io_block_kill_players_and_objects_inside_block<T: IO_BlockTrait + ?Sized>(g: &mut Game, this: &mut T, playerID: i16) {
    let iposx = this.block().iposx;
    let iposy = this.block().iposy;

    //Loop through players
    for i in 0..g.players.len() {
        let mut player = g.players[i];
        if !g.player(player).isready() {
            continue;
        }

        let mut iSwapSides: i16 = 0;
        if g.player(player).fOldX >= (iposx as i32 + TILESIZE) as f32 {
            iSwapSides = -App::screenWidth as i16;
        }

        if g.player(player).fOldX + PW as f32 + iSwapSides as f32 >= iposx as f32
            && g.player(player).fOldX + (iSwapSides as f32) < (iposx as i32 + TILESIZE) as f32
            && g.player(player).fOldY + PH as f32 >= iposy as f32
            && g.player(player).fOldY < (iposy as i32 + TILESIZE) as f32
        {
            g.player(player).iSuicideCreditPlayerID = playerID;
            g.player(player).iSuicideCreditTimer = 1;
            g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, true, -1);
        }
    }

    //Loop through objects
    for iLayer in 0..3usize {
        let count = g.objectcontainer[iLayer].list().len();
        for i in 0..count {
            let mut obj: Ptr<dyn CObjectTrait> = g.objectcontainer[iLayer].list()[i];
            let movingobject: &mut dyn IO_MovingObjectTrait = match obj.as_io_moving_object() {
                Some(m) => m,
                None => continue,
            };
            if !movingobject.mo().collides_with_map() {
                continue;
            }

            let fOldX = movingobject.mo().fOldX;
            let fOldY = movingobject.mo().fOldY;
            let w = movingobject.mo().collisionWidth;
            let h = movingobject.mo().collisionHeight;

            let mut iSwapSides: i16 = 0;
            if fOldX >= (iposx as i32 + TILESIZE) as f32 {
                iSwapSides = -App::screenWidth as i16;
            }

            if fOldX + w as f32 + iSwapSides as f32 >= iposx as f32
                && fOldX + (iSwapSides as f32) < (iposx as i32 + TILESIZE) as f32
                && fOldY + h as f32 >= iposy as f32
                && fOldY < (iposy as i32 + TILESIZE) as f32
            {
                movingobject.kill_object_map_hazard(g, playerID);
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
