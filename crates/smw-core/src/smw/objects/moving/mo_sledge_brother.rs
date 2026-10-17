//! Port of src/smw/objects/moving/MO_SledgeBrother.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_FallingObject;
use crate::common::game::App;
use crate::common::game_mode::game_mode_boss_minigame;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::match_types::Boss;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::mini_boss::CGM_Boss_MiniGame;

use crate::smw::object_container::CObjectContainer;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::co_bomb::CO_Bomb;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::moving::mo_sledge_hammer::MO_SledgeHammer;
use crate::smw::objects::moving::mo_super_fireball::MO_SuperFireball;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::objects::overmap::wo_thwomp::OMO_Thwomp;
use crate::smw::player::{get_player_from_global_id, CPlayer};

fn push_bombs(g: &mut Game, container: &CObjectContainer, x: i16, y: i16) {
    let mut gp = g.ptr();
    let list = container.list();
    for i in 0..list.len() {
        let mut obj = list[i];
        let Some(bomb) = obj.as_any().downcast_mut::<CO_Bomb>() else {
            continue;
        };
        if bomb.has_owner() {
            continue;
        }

        let bombx: i32 = bomb.x() + (bomb.w() >> 1) - x as i32;
        let bomby: i32 = bomb.y() + (bomb.h() >> 1) - y as i32;

        let dist: i32 = bombx * bombx + bomby * bomby;

        if dist < 10000 {
            let signX: f32 = if bombx > 0 { 1.0f32 } else { -1.0f32 };
            bomb.velx += signX * (RANDOM_INT(gp.get(), 30) as f32 / 10.0f32 + 4.0f32);
            bomb.vely -= RANDOM_INT(gp.get(), 30) as f32 / 10.0f32 + 6.0f32;
        }
    }
}

//------------------------------------------------------------------------------
// class sledge brother
//------------------------------------------------------------------------------
//{jump, throw, turn, wait, taunt} otherwise move
pub static g_iSledgeBrotherActions: [[[i16; 5]; 5]; 3] = [
    [[0, 50, 65, 75, 85], [3, 53, 66, 76, 86], [5, 55, 70, 78, 83], [5, 70, 85, 85, 90], [5, 75, 90, 90, 90]],
    [[0, 30, 60, 80, 85], [5, 40, 65, 80, 85], [5, 50, 65, 75, 85], [10, 55, 65, 70, 90], [10, 55, 65, 65, 90]],
    [[0, 50, 65, 75, 85], [3, 53, 65, 75, 85], [5, 60, 72, 80, 88], [5, 65, 80, 85, 90], [5, 75, 90, 90, 90]],
];

pub static g_iSledgeBrotherNeedAction: [[[i16; 6]; 5]; 3] = [
    [[50, 5, 5, 10, 10, 8], [30, 4, 5, 10, 10, 8], [20, 4, 5, 12, 12, 8], [12, 4, 5, 15, 15, 10], [8, 4, 5, 15, 15, 12]],
    [[50, 5, 5, 10, 15, 8], [30, 4, 5, 10, 12, 8], [20, 4, 5, 12, 10, 8], [15, 4, 5, 15, 8, 10], [10, 4, 5, 15, 6, 12]],
    [[50, 5, 8, 10, 10, 8], [30, 5, 8, 12, 12, 10], [20, 4, 8, 15, 15, 15], [15, 4, 8, 15, 15, 15], [12, 3, 8, 15, 15, 15]],
];

pub static g_iSledgeBrotherMaxAction: [[[i16; 5]; 5]; 3] = [
    [[1, 1, 1, 2, 2], [1, 2, 1, 2, 2], [1, 3, 1, 2, 2], [1, 3, 1, 1, 1], [1, 3, 1, 0, 0]],
    [[1, 1, 1, 2, 2], [1, 2, 1, 2, 2], [1, 3, 1, 2, 2], [1, 3, 1, 1, 1], [1, 3, 1, 0, 0]],
    [[1, 1, 1, 2, 2], [1, 2, 1, 2, 2], [1, 2, 1, 1, 1], [1, 3, 1, 1, 1], [1, 3, 1, 0, 0]],
];

pub static g_iSledgeBrotherWaitTime: [[[i16; 2]; 5]; 3] = [
    [[30, 50], [25, 45], [20, 40], [15, 30], [10, 20]],
    [[30, 50], [25, 45], [20, 40], [15, 30], [10, 20]],
    [[30, 50], [25, 45], [20, 40], [15, 30], [10, 20]],
];

#[derive(Clone)]

pub struct MO_SledgeBrother {
    pub io_moving_object: IO_MovingObject,

    pub iType: i16,

    pub iActionState: i16,
    pub iDestLocationX: [i16; 5],

    pub location: i16,

    pub throwing_timer: i16,

    pub hit_timer: i16,
    pub hit_movement_timer: i16,
    pub hit_offset_y: i16,

    pub leg_offset_x: i16,
    pub leg_movement_timer: i16,

    pub arm_offset_x: i16,
    pub arm_movement_timer: i16,

    pub taunt_timer: i16,
    pub wait_timer: i16,

    pub hit_points: i16,
    pub face_right: bool,

    pub iDestX: i16,

    pub iPlatformY: i16,
    pub need_action: [i16; 6],

    pub last_action: i16,
    pub last_action_count: i16,
}
impl_base!(MO_SledgeBrother => io_moving_object: IO_MovingObject);

impl MO_SledgeBrother {
    pub fn new(g: &mut Game, nspr: SpriteId, platformY: i16, r#type: Boss) -> Self {
        let mut o = MO_SledgeBrother {
            io_moving_object: IO_MovingObject::new(g, nspr, Vec2s::zero(), 8, 0, 32, 56, 8, 8, -1, -1, -1, -1),
            iType: 0,
            iActionState: 0,
            iDestLocationX: [0; 5],
            location: 0,
            throwing_timer: 0,
            hit_timer: 0,
            hit_movement_timer: 0,
            hit_offset_y: 0,
            leg_offset_x: 0,
            leg_movement_timer: 0,
            arm_offset_x: 0,
            arm_movement_timer: 0,
            taunt_timer: 0,
            wait_timer: 0,
            hit_points: 0,
            face_right: false,
            iDestX: 0,
            iPlatformY: 0,
            need_action: [0; 6],
            last_action: 0,
            last_action_count: 0,
        };

        o.iType = r#type as i16; // FIXME
        o.state = 1;
        o.iActionState = 0;
        o.location = 2;

        o.ih = (g.rm.sprite(o.spr).get_height() / 3) as i16;

        o.movingObjectType = movingobject_sledgebrother;

        o.inair = true;
        o.throwing_timer = 0;

        o.hit_timer = 0;
        o.hit_movement_timer = 0;
        o.hit_offset_y = (o.iType as i32 * 64) as i16;

        o.leg_offset_x = 0;
        o.leg_movement_timer = 0;

        o.arm_offset_x = 0;
        o.arm_movement_timer = 0;

        o.taunt_timer = 0;

        o.wait_timer = 0;

        o.hit_points = g.game_values.gamemodesettings.boss.hitpoints;
        o.face_right = false;

        o.vely = 0.0f32;
        o.velx = 0.0f32;

        o.iPlatformY = (platformY as i32 - o.collisionHeight as i32) as i16;
        let iPlatformY = o.iPlatformY;
        o.set_yi(iPlatformY);

        for iLocation in 0..5i16 {
            o.iDestLocationX[iLocation as usize] = (84 * iLocation as i32 + 128) as i16;
        }

        o.iDestX = o.iDestLocationX[o.location as usize];
        let iDestX = o.iDestX;
        o.set_xi(iDestX);

        for iAction in 0..6usize {
            o.need_action[iAction] = 0;
        }

        o.last_action = -1;
        o.last_action_count = 0;
        o
    }

    pub fn hit(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        if g.player(player).is_shielded() {
            return false;
        }

        g.player(player).kill_player_map_hazard(g, false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill
    }

    pub fn damage(&mut self, g: &mut Game, playerID: i16) {
        let mut gp = g.ptr();
        if self.hit_timer != 0 || playerID == -1 {
            return;
        }

        // Find the player that shot this projectile so we can attribute a kill
        let killer: PlayerId = get_player_from_global_id(g, playerID);

        if !killer.is_none() {
            self.hit_points -= 1;
            if self.hit_points <= 0 {
                self.die(g);

                if g.mode().gamemode == game_mode_boss_minigame {
                    g.mode().get().as_any().downcast_mut::<CGM_Boss_MiniGame>().unwrap().set_winner(g, killer);
                }
            } else {
                self.hit_timer = 60;
                if_sound_on_play(g, &mut gp.rm.sfx_stun);
            }
        }
    }

    fn set_last_action(&mut self, r#type: i16) {
        if self.last_action != r#type {
            self.last_action_count = 1;
            self.last_action = r#type;
        } else {
            self.last_action_count += 1;
        }
    }

    fn randomaction(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let randaction: i32 = RANDOM_INT(gp.get(), 100);

        let t = self.iType as usize;
        let d = g.game_values.gamemodesettings.boss.difficulty as usize;
        let need = &g_iSledgeBrotherNeedAction[t][d];
        let actions = &g_iSledgeBrotherActions[t][d];
        let max = &g_iSledgeBrotherMaxAction[t][d];
        let waittime = &g_iSledgeBrotherWaitTime[t][d];
        let (last_action, last_action_count) = (self.last_action, self.last_action_count);

        if self.need_action[0] > need[0] {
            self.jump();
        } else if self.need_action[1] > need[1] {
            self.throwprojectile(g);
        } else if self.need_action[2] > need[2] {
            self.turn(g);
        } else if self.need_action[3] > need[3] {
            self.wait(g, waittime[0], waittime[1]);
        } else if self.need_action[4] > need[4] {
            self.taunt(g);
        } else if self.need_action[5] > need[5] {
            self.r#move(g);
        }
        // then do action based on probability
        else if (last_action != 0 || last_action_count < max[0]) && randaction < actions[0] as i32 {
            self.jump();
        } else if (last_action != 1 || last_action_count < max[1]) && randaction < actions[1] as i32 {
            self.throwprojectile(g);
        } else if (last_action != 2 || last_action_count < max[2]) && randaction < actions[2] as i32 {
            self.turn(g);
        } else if (last_action != 2 || last_action_count < max[3]) && randaction < actions[3] as i32 {
            self.wait(g, waittime[0], waittime[1]);
        } else if (last_action != 2 || last_action_count < max[4]) && randaction < actions[4] as i32 {
            self.taunt(g);
        } else {
            self.r#move(g);
        }

        for iAction in 0..6usize {
            self.need_action[iAction] += 1;
        }
    }

    fn r#move(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let moveright = if self.location == 0 {
            true
        } else if self.location == 4 {
            false
        } else {
            RANDOM_INT(gp.get(), 2) == 0
        };

        self.set_last_action(3);

        if moveright {
            self.location += 1;
        } else {
            self.location -= 1;
        }

        self.iDestX = self.iDestLocationX[self.location as usize];
        self.iActionState = 4;

        self.face_right = self.iDestX > self.ix;

        self.need_action[5] = 0;
    }

    fn throwprojectile(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        self.set_last_action(1);

        self.throwing_timer = 20;
        self.iActionState = 3;
        self.arm_offset_x = 96;

        let (ix, iy, iw) = (self.ix as i32, self.iy as i32, self.iw as i32);
        if self.iType == 0 {
            let fHammerVelX: f32 = (RANDOM_INT(gp.get(), 9) + 2) as f32 / 2.0f32 - if self.face_right { 0.0f32 } else { 6.0f32 };
            let pos = Vec2s::new(((if self.face_right { ix + 32 } else { ix }) - self.collisionOffsetX as i32) as i16, self.iy);
            let obj = Ptr::new_box(MO_SledgeHammer::new(
                g,
                rm.spr_sledgehammer,
                pos,
                8,
                Vec2f::new(fHammerVelX, -HAMMERTHROW),
                5,
                -1,
                -1,
                -1,
                false,
            ));
            gp.objectcontainer[2].add(g, obj);
        } else if self.iType == 1 {
            let fBombVelX: f32 = (RANDOM_INT(gp.get(), 5) + 12) as f32 / 2.0f32 - if self.face_right { 0.0f32 } else { 14.0f32 };
            let fBombVelY: f32 = -(RANDOM_INT(gp.get(), 13) as f32) / 2.0f32 - 6.0f32;
            let pos = Vec2s::new((if self.face_right { ix + iw - 32 } else { ix - 20 }) as i16, iy as i16);
            let timetolive = (RANDOM_INT(gp.get(), 60) + 120) as i16;
            let obj = Ptr::new_box(CO_Bomb::new(
                g,
                rm.spr_bomb,
                pos,
                Vec2f::new(fBombVelX, fBombVelY),
                4,
                -1,
                -1,
                -1,
                timetolive,
            ));
            gp.objectcontainer[2].add(g, obj);
        } else if self.iType == 2 {
            let fFireVelX: f32 = (RANDOM_INT(gp.get(), 9) + 6) as f32 / 2.0f32 - if self.face_right { 0.0f32 } else { 10.0f32 };
            let fFireVelY: f32 = RANDOM_INT(gp.get(), 17) as f32 / 2.0f32 - 4.0f32;
            let pos = Vec2s::new((if self.face_right { ix + iw - 32 } else { ix - 16 }) as i16, iy as i16);
            let obj = Ptr::new_box(MO_SuperFireball::new(
                g,
                rm.spr_superfireball,
                pos,
                4,
                Vec2f::new(fFireVelX, fFireVelY),
                4,
                -1,
                -1,
                -1,
            ));
            gp.objectcontainer[2].add(g, obj);
        }

        self.need_action[1] = 0;
    }

    fn taunt(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.set_last_action(2);

        gp.rm.sfx_boomerang.play_loop(g, 3);
        self.taunt_timer = 60;
        self.iActionState = 5;

        self.need_action[4] = 0;

        // If this is a bomb brother, push bombs away when taunting
        if self.iType == 1 {
            push_bombs(g, &gp.objectcontainer[2], (self.ix as i32 + 32) as i16, (self.iy as i32 + 32) as i16);
        }
    }

    fn turn(&mut self, g: &mut Game) {
        self.face_right = !self.face_right;
        let waittime = g_iSledgeBrotherWaitTime[self.iType as usize][g.game_values.gamemodesettings.boss.difficulty as usize];
        self.wait(g, waittime[0] >> 1, waittime[1] >> 1);

        self.need_action[2] = 0;
    }

    fn jump(&mut self) {
        self.set_last_action(0);

        self.vely = -VELJUMP;
        self.iActionState = 2;

        self.need_action[0] = 0;
    }

    fn wait(&mut self, g: &mut Game, min: i16, max: i16) {
        let mut gp = g.ptr();
        self.set_last_action(2);

        self.wait_timer = RandomNumberGenerator::generator(gp.get()).get_integer_range(min as i32, max as i32) as i16;
        self.iActionState = 1;

        self.need_action[3] = 0;
    }
}

impl CObjectTrait for MO_SledgeBrother {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.rm.sprite(self.spr).draw_src(
            g,
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &Rect {
                x: self.leg_offset_x as i32 + self.arm_offset_x as i32 + if self.face_right { 0 } else { 192 },
                y: self.hit_offset_y as i32,
                w: self.iw as i32,
                h: self.ih as i32,
            },
        );

        if self.hit_timer != 0 {
            for iHeart in 0..self.hit_points {
                gp.rm.sprite(gp.rm.spr_scorehearts).draw_src(
                    g,
                    self.ix as i32 - self.collisionOffsetX as i32 + iHeart as i32 * 8,
                    self.iy as i32 - self.collisionOffsetY as i32 - 18,
                    &Rect { x: 0, y: 0, w: 16, h: 16 },
                );
            }
        }
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        if self.iActionState == 0 {
            if self.hit_timer <= 0 {
                self.randomaction(g);
            }
        } else if self.iActionState == 1 {
            self.wait_timer -= 1;
            if self.wait_timer <= 0 {
                self.iActionState = 0;
            }
        } else if self.iActionState == 2 {
            let (fy, vely) = (self.fy, self.vely);
            self.set_yf(fy + vely);
            self.vely += GRAVITATION;

            if self.iy >= self.iPlatformY {
                self.iActionState = 0;

                if self.iType == 0 {
                    // Shake screen and kill players
                    if_sound_on_play(g, &mut gp.rm.sfx_thunder);
                    g.game_values.flags.screenshaketimer = 20;
                    g.game_values.flags.screenshakeplayerid = -1;
                    g.game_values.flags.screenshaketeamid = -1;
                    g.game_values.flags.screenshakekillinair = false;
                    g.game_values.flags.screenshakekillscount = 0;
                } else if self.iType == 1 {
                    // Spawn thwomps
                    if_sound_on_play(g, &mut gp.rm.sfx_thunder);

                    let numThwomps: i16 = (RANDOM_INT(gp.get(), 5) + 6) as i16;

                    for _iThwomp in 0..numThwomps {
                        let x = RANDOM_INT(gp.get(), 591) as i16;
                        let nspeed = 2.0f32 + RANDOM_INT(gp.get(), 20) as f32 / 10.0f32;
                        let obj = Ptr::new_box(OMO_Thwomp::new(g, rm.spr_thwomp, x, nspeed));
                        gp.objectcontainer[2].add(g, obj);
                    }
                } else if self.iType == 2 {
                    // Spawn lots of podobos
                    if_sound_on_play(g, &mut gp.rm.sfx_thunder);

                    let numPodobos: i16 = (RANDOM_INT(gp.get(), 5) + 8) as i16;

                    for _iPodobo in 0..numPodobos {
                        let x = RANDOM_INT(gp.get(), 608) as i16;
                        let dVelY = -(RANDOM_INT(gp.get(), 9) as f32 / 2.0f32) - 9.0f32;
                        let obj = Ptr::new_box(MO_Podobo::new(
                            g,
                            rm.spr_podobo,
                            Vec2s::new(x, App::screenHeight as i16),
                            dVelY,
                            -1,
                            -1,
                            -1,
                            false,
                        ));
                        gp.objectcontainer[2].add(g, obj);
                    }
                }
            }
        } else if self.iActionState == 3 {
            self.throwing_timer -= 1;
            if self.throwing_timer <= 0 {
                self.iActionState = 0;
                self.throwing_timer = 0;
                self.arm_offset_x = 0;
            }
        } else if self.iActionState == 4 {
            // move towards destination
            if self.ix < self.iDestX {
                self.ix += if g.game_values.gamemodesettings.boss.difficulty >= 3 { 2 } else { 1 };

                if self.ix >= self.iDestX {
                    self.ix = self.iDestX;
                    self.iActionState = 0;
                    self.leg_offset_x = 0;
                    self.leg_movement_timer = 0;
                }
            } else if self.ix > self.iDestX {
                self.ix -= if g.game_values.gamemodesettings.boss.difficulty >= 3 { 2 } else { 1 };

                if self.ix <= self.iDestX {
                    self.ix = self.iDestX;
                    self.iActionState = 0;
                    self.leg_offset_x = 0;
                    self.leg_movement_timer = 0;
                }
            }

            if self.iActionState != 0 && {
                self.leg_movement_timer += 1;
                self.leg_movement_timer == 8
            } {
                self.leg_movement_timer = 0;

                if self.leg_offset_x == 0 {
                    self.leg_offset_x = 48;
                } else {
                    self.leg_offset_x = 0;
                }
            }
        } else if self.iActionState == 5 {
            // If we are done taunting, reset arm/legs back to normal state
            self.taunt_timer -= 1;
            if self.taunt_timer <= 0 {
                self.iActionState = 0;
                self.arm_offset_x = 0;
                self.leg_offset_x = 0;
                self.arm_movement_timer = 0;
                self.leg_movement_timer = 0;
            } else {
                // otherwise move them around
                self.arm_movement_timer += 1;
                if self.arm_movement_timer == 8 {
                    self.arm_movement_timer = 0;

                    if self.arm_offset_x == 0 {
                        self.arm_offset_x = 96;
                    } else {
                        self.arm_offset_x = 0;
                    }
                }

                self.leg_movement_timer += 1;
                if self.leg_movement_timer == 6 {
                    self.leg_movement_timer = 0;

                    if self.leg_offset_x == 0 {
                        self.leg_offset_x = 48;
                    } else {
                        self.leg_offset_x = 0;
                    }
                }
            }
        }

        if self.hit_timer > 0 {
            self.hit_timer -= 1;
            if self.hit_timer <= 0 {
                self.hit_offset_y = (self.iType as i32 * 64) as i16;
                self.hit_timer = 0;
            } else {
                self.hit_movement_timer += 1;
                if self.hit_movement_timer == 2 {
                    self.hit_movement_timer = 0;

                    self.hit_offset_y += 64;

                    if self.hit_offset_y >= 192 {
                        self.hit_offset_y = 0;
                    }
                }
            }
        }
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut gp = g.ptr();
        if self.iActionState == 0 {
            return false;
        }

        if g.player(player).is_invincible() {
            if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
            self.die(g);

            if g.mode().gamemode == game_mode_boss_minigame {
                g.mode().get().as_any().downcast_mut::<CGM_Boss_MiniGame>().unwrap().set_winner(g, player);
            }
        } else {
            return self.hit(g, player);
        }

        false
    }

    fn collide_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut gp = g.ptr();
        if object.is_dead() {
            return;
        }

        let r#type = object.get_moving_object_type();

        // Ignore hammers and fireballs thrown from sledge brother
        if r#type == movingobject_sledgehammer || r#type == movingobject_superfireball {
            if object.iPlayerID == -1 {
                return;
            }
        }

        removeifprojectile(g, object, false, false);

        // These types of attacks damage the boss
        let fDamageWeapon =
            r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox || r#type == movingobject_explosion;

        // These don't damage him but still collide
        let fNoDamageWeapon = r#type == movingobject_fireball
            || r#type == movingobject_bulletbill
            || r#type == movingobject_hammer
            || r#type == movingobject_boomerang
            || r#type == movingobject_attackzone;

        if fDamageWeapon || fNoDamageWeapon {
            // If it is a shell but it is sitting, don't collide
            if r#type == movingobject_shell && object.get_state() == 2 {
                return;
            }

            // If it is a throw box but is sitting, dont' collide
            if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                return;
            }

            if fDamageWeapon {
                let iPlayerID = object.iPlayerID;
                self.damage(g, iPlayerID);

                if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox || r#type == movingobject_attackzone {
                    if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
                    object.die(g);
                }
            } else {
                if_sound_on_play(g, &mut gp.rm.sfx_hit);

                if r#type == movingobject_attackzone || r#type == movingobject_bulletbill {
                    object.die(g);
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_SledgeBrother {
    crate::impl_io_moving_object_plumbing!();

    fn die(&mut self, g: &mut Game) {
        self.dead = true;
        g.eyecandy[2].emplace(EC_FallingObject::new(&g.rm, 
            g.rm.spr_sledgebrothersdead,
            self.ix,
            self.iy,
            0.0f32,
            -VELJUMP / 2.0f32,
            1,
            0,
            0,
            (self.iType as i32 * 64) as i16,
            self.iw,
            self.ih,
        ));
    }
}
