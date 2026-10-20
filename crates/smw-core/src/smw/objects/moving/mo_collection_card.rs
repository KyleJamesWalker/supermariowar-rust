//! Card for card collection mode: players hold up to three, scored by their values.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::ticks;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::globals::*;
use crate::impl_base;

use crate::smw::objects::moving::moving_object::{
    io_moving_object_collision_detection_checksides, io_moving_object_draw, io_moving_object_update, IO_MovingObject, IO_MovingObjectTrait,
};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class collection card (for card collection mode)
//------------------------------------------------------------------------------
/// Collectible card. A placed card flips to show its value when taken; a card a player lost is taken at once.
#[derive(Clone)]
pub struct MO_CollectionCard {
    pub io_moving_object: IO_MovingObject,

    /// Ticks at the current spot, or ticks shown face up after being taken.
    pub timer: i16,
    /// 0 placed at a spawn point, 1 knocked loose from a player (falls, expires).
    pub r#type: i16,
    /// Card value 0 to 2, stored in two bits of the player's `subscore[1]`.
    pub value: i16,

    /// Tick counter for the sparkle overlay; it steps a frame every 4 ticks.
    pub sparkleanimationtimer: i16,
    /// X offset (pixels) of the current sparkle frame.
    pub sparkledrawframe: i16,

    /// Ticks before a loose card can be taken; it expires once this passes minus the card life setting.
    pub uncollectabletime: i16,
}
impl_base!(MO_CollectionCard => io_moving_object: IO_MovingObject);

impl MO_CollectionCard {
    pub fn new(g: &mut Game, nspr: SpriteId, iType: i16, iValue: i16, iUncollectableTime: i16, vel: Vec2f, pos: Vec2s) -> Self {
        let mut o = MO_CollectionCard {
            io_moving_object: IO_MovingObject::new(g, nspr, pos, 6, 8, -1, -1, -1, -1, 0, 0, 32, 32),
            timer: 0,
            r#type: 0,
            value: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            uncollectabletime: 0,
        };

        o.state = 1;
        o.objectType = object_moving;
        o.movingObjectType = movingobject_collectioncard;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.r#type = iType;
        o.value = iValue;

        o.uncollectabletime = iUncollectableTime;
        o.velx = vel.x;
        o.vely = vel.y;

        if iType == 0 {
            o.place_card(g);
            o.fObjectCollidesWithMap = false;
        } else {
            io_moving_object_collision_detection_checksides(g, &mut o);
            o.animationOffsetY = ((o.value as i32 + 1) << 5) as i16;
        }
        o
    }

    pub fn get_type(&self) -> i16 {
        self.r#type
    }
    pub fn get_value(&self) -> i16 {
        self.value
    }

    /// Moves the card to a random spawn point more than 150 pixels from other cards, trying up to 33 times.
    pub fn place_card(&mut self, g: &mut Game) {
        let mut gp = g.ptr();

        self.timer = 0;

        let mut x: i16 = 0;
        let mut y: i16 = 0;
        let mut iAttempts: i16 = 32;
        while (!gp.g_map.findspawnpoint(g, 5, &mut x, &mut y, self.collisionWidth, self.collisionHeight, false)
            || g.objectcontainer[1].get_closest_moving_object(x, y, movingobject_collectioncard) <= 150.0f32)
            && {
                let a = iAttempts;
                iAttempts -= 1;
                a > 0
            }
        {}

        self.set_xi(x);
        self.set_yi(y);
    }
}

impl CObjectTrait for MO_CollectionCard {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        let mut gp = g.ptr();
        // If it is not collectable, return
        if (self.r#type == 1 && self.uncollectabletime > 0) || self.state != 1 {
            return false;
        }

        if_sound_on_play(g, &mut gp.rm.sfx_areatag);

        let value = self.value as i32;

        // Add this card to the team's score
        if g.player(player).score(g).subscore[0] < 3 {
            let shift = (g.player(player).score(g).subscore[0] as i32) << 1;
            g.player(player).score(g).subscore[1] = (g.player(player).score(g).subscore[1] as i32 | (value << shift)) as i16;
            g.player(player).score(g).subscore[0] += 1;
        } else {
            g.player(player).score(g).subscore[1] = (g.player(player).score(g).subscore[1] as i32 & !48) as i16; // Clear previous card in 3rd slot
            g.player(player).score(g).subscore[1] = (g.player(player).score(g).subscore[1] as i32 | (value << 4)) as i16; // Set card to newly collected one in 3rd slot
        }

        g.player(player).score(g).subscore[2] = 0;

        if self.r#type == 1 {
            self.dead = true;
        } else {
            self.state = 2;
            self.animationspeed = 4;
            self.animationtimer = 0;
            self.animationOffsetY = ((value + 1) << 5) as i16;
            self.drawframe = 96;
        }

        self.timer = 0;

        false
    }

    fn update(&mut self, g: &mut Game) {
        if self.r#type == 1 || self.state < 3 {
            self.animate();
        }

        // A taken card flips over to show its value, then disappears.
        if self.state == 2 && self.drawframe == 0 {
            self.state = 3;
            self.timer = 0;
        } else if self.state == 3 {
            self.timer += 1;
            if self.timer as i32 > ticks(200) {
                self.dead = true;
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&g.rm, g.rm.spr_fireballexplosion, self.ix, self.iy, 3, 8));
            }
        }

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        if self.r#type == 0 {
            self.timer += 1;
            if self.timer as i32 > ticks(1500) {
                self.place_card(g);
            }
        } else {
            self.applyfriction();
            io_moving_object_update(g, self);

            self.uncollectabletime -= 1;
            if (self.uncollectabletime as i32) < -(g.game_values.gamemodesettings.collection.cardlife as i32) {
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&g.rm, g.rm.spr_fireballexplosion, self.ix, self.iy, 3, 8));
                self.dead = true;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        io_moving_object_draw(g, self);

        // Draw sparkles
        gp.rm.sprite(gp.rm.spr_shinesparkle).draw_src(
            g,
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
        );
    }
}

impl IO_MovingObjectTrait for MO_CollectionCard {
    crate::impl_io_moving_object_plumbing!();
}
