//! Port of src/smw/player.cpp

use crate::common::eyecandy::{EC_Corpse, EC_Door, EC_FallingObject, EC_GravText, EC_SingleAnimation, Spotlight};
use crate::common::eyecandy_styles::{AwardStyle, SpawnStyle};
use crate::common::game::App;
use crate::common::game_mode::{game_mode_bonus, game_mode_shyguytag, game_mode_star};
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::{BoomerangStyle, StarStyle};
use crate::common::gfx::gfx_palette::{self, PlayerPalette};
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::gfx::SpriteStrip;
use crate::common::global_constants::*;
use crate::common::input::{COutputControl, DEVICE_KEYBOARD};
use crate::common::map::CMap;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::movingplatform::MovingPlatform;
use crate::common::object_base::{cap_falling_velocity, cap_side_velocity, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::score::CScore;
use crate::common::tile_types::*;
use crate::globals::*;
use crate::smw::ai::CPlayerAITrait;
use crate::smw::gamemodes::chicken::CGM_Chicken;
use crate::smw::gamemodes::shy_guy_tag::CGM_ShyGuyTag;
use crate::smw::gamemodes::star::CGM_Star;
use crate::smw::gamemodes::tag::CGM_Tag;
use crate::smw::gs_gameplay::{eyecandy, g_iWinningPlayer, noncolcontainer, objectcontainer, spotlightManager, swap_players};
use crate::smw::main::{g_iSwirlSpawnLocations, players};
use crate::smw::net::netplay;
use crate::smw::net_random::{self, Ev};
use crate::smw::objectgame::{check_secret, PowerupType};
use crate::smw::objects::carriable::co_bomb::CO_Bomb;
use crate::smw::objects::carriable::co_kuribo_shoe::CO_KuriboShoe;
use crate::smw::objects::carriable::co_shell::{ShellType, CO_Shell};
use crate::smw::objects::moving::mo_boomerang::MO_Boomerang;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::mo_fireball::MO_Fireball;
use crate::smw::objects::moving::mo_hammer::MO_Hammer;
use crate::smw::objects::moving::mo_ice_blast::MO_IceBlast;
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::moving::moving_object::io_moving_object_collision_detection_checksides;
use crate::smw::player_components::player_award_effects::PlayerAwardEffects;
use crate::smw::player_components::player_burnup_timer::PlayerBurnupTimer;
use crate::smw::player_components::player_cape::PlayerCape;
use crate::smw::player_components::player_card_collection::PlayerCardCollection;
use crate::smw::player_components::player_collisions::PlayerCollisions;
use crate::smw::player_components::player_invincibility::PlayerInvincibility;
use crate::smw::player_components::player_jail::PlayerJail;
use crate::smw::player_components::player_kuribo_shoe::{KuriboShoeType, PlayerKuriboShoe, STICKY};
use crate::smw::player_components::player_out_of_arena_timer::PlayerOutOfArenaTimer;
use crate::smw::player_components::player_secret_code::PlayerSecretCode;
use crate::smw::player_components::player_shield::PlayerShield;
use crate::smw::player_components::player_spin_status::PlayerSpinStatus;
use crate::smw::player_components::player_suicide_timer::PlayerSuicideTimer;
use crate::smw::player_components::player_super_stomp::PlayerSuperStomp;
use crate::smw::player_components::player_tail::PlayerTail;
use crate::smw::player_components::player_tanooki_suit::PlayerTanookiSuit;
use crate::smw::player_components::player_warp_status::PlayerWarpStatus;
use crate::smw::player_components::player_wings::PlayerWings;
use sdl2::sys::SDL_Rect;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlayerState {
    #[default]
    Waiting,
    Spawning,
    Dead,
    Ready,
    EnteringWarpUp,
    EnteringWarpRight,
    EnteringWarpDown,
    EnteringWarpLeft,
    LeavingWarpDown,
    LeavingWarpLeft,
    LeavingWarpUp,
    LeavingWarpRight,
}
crate::enum_from_u8!(PlayerState, 12);

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlayerAction {
    #[default]
    None,
    Bobomb,
    Fireball,
    Hammer,
    Boomerang,
    Iceblast,
    Bomb,
    SpinCape,
    SpinTail,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlayerDeathStyle {
    #[default]
    Jump,
    Squish,
    Shatter,
}

/// All C++ members are `pub` so friend classes and components reach them by their C++ names.
#[derive(Default)]
pub struct CPlayer {
    pub shyguy: bool,

    pub globalID: i16,
    pub teamID: i16,

    pub collisions: PlayerCollisions,
    pub awardeffects: PlayerAwardEffects,

    pub playerKeys: Ptr<COutputControl>,
    pub playerDevice: i16,

    pub tanookisuit: PlayerTanookiSuit,

    pub score: Ptr<CScore>,
    pub killsinrow: i16,
    pub killsinrowinair: i16,

    pub localID: i16,
    pub subTeamID: i16,
    pub colorID: i16,

    pub ix: i16,
    pub iy: i16,
    pub fx: f32,
    pub fy: f32,
    pub velx: f32,
    pub vely: f32,
    pub fOldX: f32,
    pub fOldY: f32,
    pub fPrecalculatedY: f32,
    pub oldvelx: f32,

    pub fNewSwapX: f32,
    pub fNewSwapY: f32,
    pub fOldSwapX: f32,
    pub fOldSwapY: f32,
    pub iSrcOffsetX: i16,
    pub pScoreboardSprite: Ptr<SpriteStrip>,

    pub iNewPowerupX: i16,
    pub iNewPowerupY: i16,
    pub iOldPowerupX: i16,
    pub iOldPowerupY: i16,

    pub inair: bool,
    pub onice: bool,

    pub lockjump: bool,
    pub lockfall: bool,
    pub lockfire: bool,
    pub throw_star: i16,

    pub extrajumps: i16,
    pub flying: bool,
    pub flyingtimer: i16,

    pub tail: PlayerTail,
    pub spin: PlayerSpinStatus,
    pub wings: PlayerWings,

    pub superjumptimer: i16,
    pub superjumptype: i16,
    pub hammertimer: i16,

    pub frictionslidetimer: i16,
    pub bobombsmoketimer: i16,
    pub rainsteptimer: i16,

    pub pPlayerAI: Option<Box<dyn CPlayerAITrait>>,

    pub sprites: Ptr<SpriteStrip>,

    pub sprite_state: u8,
    pub sprswitch: i16,

    pub invincibility: PlayerInvincibility,
    pub shield: PlayerShield,

    pub frozen: bool,
    pub frozentimer: i16,

    pub kuriboshoe: PlayerKuriboShoe,
    pub secretcode: PlayerSecretCode,
    pub cardcollection: PlayerCardCollection,

    pub superstomp: PlayerSuperStomp,
    pub burnup: PlayerBurnupTimer,
    pub outofarena: PlayerOutOfArenaTimer,
    pub suicidetimer: PlayerSuicideTimer,

    pub action: PlayerAction,

    pub powerup: i16,
    pub projectilelimit: i16,
    pub projectiles: i32,

    pub bobomb: bool,

    pub cape: PlayerCape,

    pub state: PlayerState,
    pub spawnradius: f32,
    pub spawnangle: f32,
    pub spawntimer: i16,

    pub waittimer: i16,
    pub respawncounter: Ptr<i16>,

    pub warpstatus: PlayerWarpStatus,

    pub powerupused: Option<PowerupType>,
    pub powerupradius: f32,
    pub powerupangle: f32,

    pub fAcceptingItem: bool,
    pub fPressedAcceptItem: bool,
    pub carriedItem: Ptr<dyn MO_CarriedObjectTrait>,

    pub platform: Ptr<MovingPlatform>,
    pub iHorizontalPlatformCollision: i16,
    pub iVerticalPlatformCollision: i16,
    pub iPlatformCollisionPlayerId: i16,

    pub ownerPlayerID: i16,
    pub ownerColorOffsetX: i16,

    pub jail: PlayerJail,

    pub fallthrough: bool,

    pub diedas: i16,

    pub spawntext: i16,

    pub iSuicideCreditPlayerID: i16,
    pub iSuicideCreditTimer: i16,

    pub sSpotlight: Ptr<Spotlight>,

    pub net_waitingForPowerupTrigger: bool,

    pub _alias: Aliased,
}

#[inline(always)]
fn b(v: i32) -> bool {
    v != 0
}

impl CScore {
    pub fn adjust_score(&mut self, iValue: i16) {
        unsafe {
            if game_values.gamemode.gameover || crate::smw::net_outcomes::score_locked() {
                return;
            }
        }

        self.score = (self.score as i32 + iValue as i32) as i16;

        if self.score < 0 {
            self.score = 0;
        }

        self.set_digit_counters();
    }
}

pub fn get_player_from_global_id(iGlobalID: i16) -> Ptr<CPlayer> {
    unsafe {
        for &player in players.iter() {
            if player.globalID == iGlobalID {
                return player;
            }
        }
    }

    Ptr::null()
}

pub fn should_update_sprite() -> bool {
    unsafe {
        if netplay.active {
            return true;
        }
    }

    unsafe { !game_values.flags.pausegame && !game_values.flags.exitinggame && !game_values.flags.swapplayers }
}

pub fn player_killed_player(iKiller: i16, killed: Ptr<CPlayer>, deathstyle: PlayerDeathStyle, style: KillStyle, fForce: bool, fKillCarriedItem: bool) -> PlayerKillType {
    let args = [0, iKiller as i32, killed.globalID as i32, deathstyle as i32, style as i32, fForce as i32, fKillCarriedItem as i32, -1];
    if let Some(result) = crate::smw::net_outcomes::kill(&args) {
        return result;
    }
    player_killed_player_now(iKiller, killed, deathstyle, style, fForce, fKillCarriedItem)
}

pub fn player_killed_player_now(iKiller: i16, mut killed: Ptr<CPlayer>, deathstyle: PlayerDeathStyle, style: KillStyle, fForce: bool, fKillCarriedItem: bool) -> PlayerKillType {
    unsafe {
        let mut killer = get_player_from_global_id(iKiller);

        if !killer.is_null() && killer.globalID != killed.globalID {
            killer.killed_player(killed, deathstyle, style, fForce, fKillCarriedItem)
        } else {
            killed.death_awards();

            let iKillType = game_values.gamemode.get().playerkilledself(killed, style);

            if PlayerKillType::Normal == iKillType || (fForce && PlayerKillType::NonKill == iKillType) {
                killed.die(PlayerDeathStyle::Jump, false, fKillCarriedItem);
            }

            if PlayerKillType::NonKill != iKillType {
                if deathstyle == PlayerDeathStyle::Shatter {
                    if_sound_on_play(&mut rm.sfx_breakblock);
                } else {
                    if_sound_on_play(&mut rm.sfx_deathsound);
                }
            }

            iKillType
        }
    }
}

fn gamemode_as<T: 'static>() -> Option<&'static mut T> {
    unsafe {
        let gm = game_values.gamemode;
        if gm.is_null() {
            return None;
        }
        gm.get().as_any().downcast_mut::<T>()
    }
}

impl CPlayer {
    /// C++ `new CPlayer(...)`; the object must not move because the constructor hands out `this`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        iGlobalID: i16,
        iLocalID: i16,
        iTeamID: i16,
        iSubTeamID: i16,
        iColorID: i16,
        nsprites: Ptr<SpriteStrip>,
        nscore: Ptr<CScore>,
        sRespawnCounter: Ptr<i16>,
        ai: Option<Box<dyn CPlayerAITrait>>,
    ) -> Ptr<CPlayer> {
        let mut this = Ptr::new_box(CPlayer {
            shyguy: false,
            globalID: iGlobalID,
            teamID: iTeamID,
            score: nscore,
            localID: iLocalID,
            subTeamID: iSubTeamID,
            colorID: iColorID,
            lockfall: false,
            lockfire: false,
            pPlayerAI: ai,
            sprite_state: PGFX_JUMPING_R as u8,
            sprswitch: 0,
            projectiles: 0,
            spawnradius: 0.0,
            spawnangle: 0.0,
            respawncounter: sRespawnCounter,
            powerupradius: 0.0,
            powerupangle: 0.0,
            fAcceptingItem: false,
            fPressedAcceptItem: false,
            carriedItem: Ptr::null(),
            ownerPlayerID: -1,
            ownerColorOffsetX: 0,
            spawntext: 20, // set it to 20 so there is an immediate text spawned upon winning
            iSuicideCreditPlayerID: -1,
            iSuicideCreditTimer: 0,
            net_waitingForPowerupTrigger: false,

            collisions: PlayerCollisions::new(),
            awardeffects: PlayerAwardEffects::new(),
            tanookisuit: PlayerTanookiSuit::new(),
            tail: PlayerTail::new(),
            spin: PlayerSpinStatus::new(),
            wings: PlayerWings::new(),
            invincibility: PlayerInvincibility::new(),
            shield: PlayerShield::new(),
            kuriboshoe: PlayerKuriboShoe::new(),
            secretcode: PlayerSecretCode::new(),
            cardcollection: PlayerCardCollection::new(),
            superstomp: PlayerSuperStomp::new(),
            burnup: PlayerBurnupTimer::new(),
            outofarena: PlayerOutOfArenaTimer::new(),
            suicidetimer: PlayerSuicideTimer::new(),
            cape: PlayerCape::new(),
            warpstatus: PlayerWarpStatus::new(),
            jail: PlayerJail::new(),
            ..Default::default()
        });

        //AI stuff
        let thisPtr = this;
        if let Some(ai) = this.get().pPlayerAI.as_mut() {
            ai.set_player(thisPtr);
        }

        unsafe {
            if netplay.active {
                if this.globalID == netplay.remotePlayerNumber as i16 {
                    this.playerKeys = Ptr::from_mut(&mut game_values.playerInput.outputControls[0]);
                    println!("[net] Player {} reads local input.", this.globalID);
                } else {
                    this.playerKeys = Ptr::from_mut(&mut netplay.netPlayerInput.outputControls[this.globalID as usize]);
                    println!("[net] Player {} reads network input.", this.globalID);
                }
            } else {
                this.playerKeys = Ptr::from_mut(&mut game_values.playerInput.outputControls[this.globalID as usize]);
            }

            this.playerDevice = game_values.playerInput.inputControls[this.globalID as usize].iDevice;
            this.sprites = nsprites;

            //Do this so we have a valid x,y to say the player is so other items that init with the player will get valid positions
            //The actual choosing of a spawning position happens later
            this.find_spawn_point();

            this.setup_new_player();
            *this.respawncounter.get() = 0;

            game_values.unlocksecret1part1[this.globalID as usize] = false;
        }

        this
    }

    pub fn init(&mut self) {
        println!("CPlayer::init()");
        if let Some(ai) = self.pPlayerAI.as_mut() {
            ai.init();
        }
    }

    /* Player info */

    pub fn get_global_id(&self) -> i16 {
        self.globalID
    }
    pub fn get_team_id(&self) -> i16 {
        self.teamID
    }
    pub fn get_color_id(&self) -> i16 {
        self.colorID
    }

    pub fn left_x(&self) -> i16 {
        self.ix
    }
    pub fn right_x(&self) -> i16 {
        (self.ix as i32 + PW) as i16
    }
    pub fn center_x(&self) -> i16 {
        (self.ix as i32 + HALFPW) as i16
    }
    pub fn top_y(&self) -> i16 {
        self.iy
    }
    pub fn bottom_y(&self) -> i16 {
        (self.iy as i32 + PH) as i16
    }
    pub fn center_y(&self) -> i16 {
        (self.iy as i32 + HALFPH) as i16
    }

    pub fn get_vel_x(&self) -> f32 {
        self.velx
    }
    pub fn get_vel_y(&self) -> f32 {
        self.vely
    }

    pub fn isready(&self) -> bool {
        self.state == PlayerState::Ready
    }
    pub fn isspawning(&self) -> bool {
        self.state == PlayerState::Spawning
    }
    pub fn iswarping(&self) -> bool {
        self.state > PlayerState::Ready
    }
    pub fn isdead(&self) -> bool {
        self.state == PlayerState::Dead
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
    pub fn get_warp_plane(&self) -> i16 {
        self.warpstatus.get_warp_plane()
    }
    pub fn get_warp_state(&self) -> i16 {
        self.state as i16 % 4
    }
    pub fn is_bobomb(&self) -> bool {
        self.bobomb
    }
    pub fn is_tanooki_statue(&self) -> bool {
        self.tanookisuit.is_statue()
    }

    pub fn jail(&mut self) -> &mut PlayerJail {
        &mut self.jail
    }
    pub fn shield(&mut self) -> &mut PlayerShield {
        &mut self.shield
    }

    pub fn corpse_type(&self, _: i16) -> i16 {
        self.diedas
    }

    pub fn get_scoreboard_sprite(&self) -> Ptr<SpriteStrip> {
        self.pScoreboardSprite
    }

    pub fn pressed_accept_item_key(&self) -> bool {
        self.fPressedAcceptItem
    }
    pub fn is_accepting_item(&self) -> bool {
        self.fAcceptingItem && self.tanookisuit.not_statue() && !self.kuriboshoe.is_on()
    }

    pub fn set_stored_powerup_type(&mut self, iPowerup: PowerupType) {
        self.set_stored_powerup(iPowerup as i16);
    }

    pub fn decrease_projectiles_count(&mut self) {
        if self.projectiles > 0 {
            self.projectiles -= 1;
        }
    }
    pub fn increase_projectiles_count(&mut self, amount: u32) {
        self.projectiles = (self.projectiles as u32).wrapping_add(amount) as i32;
    }

    pub fn set_xf(&mut self, xf: f32) {
        self.fx = xf;
        self.ix = self.fx as i16;
    }
    pub fn set_xi(&mut self, xi: i16) {
        self.ix = xi;
        self.fx = self.ix as f32;
    }
    pub fn set_yf(&mut self, yf: f32) {
        self.fy = yf;
        self.iy = self.fy as i16;
    }
    pub fn set_yi(&mut self, yi: i16) {
        self.iy = yi;
        self.fy = self.iy as f32;
    }

    pub fn this(&mut self) -> Ptr<CPlayer> {
        Ptr::from_mut(self)
    }

    pub fn keys(&self) -> &mut COutputControl {
        self.playerKeys.get()
    }

    pub fn clip_edge(&self) -> ClipEdge {
        unsafe { std::mem::transmute::<i32, ClipEdge>((self.state as i16 % 4) as i32) }
    }

    pub fn update_frozen_status(&mut self, keymask: i32) {
        if self.frozen {
            // `~(sprite_state & 0x1)` is never zero, so the left key always thaws.
            if b(keymask & 4) || (b(self.sprite_state as i32 & 0x1) && b(keymask & 8)) {
                self.frozentimer -= 5;
            }

            self.frozentimer -= 1;
            if self.frozentimer <= 0 {
                self.frozentimer = 0;
                self.frozen = false;

                //Shield the player after becoming unfrozen to protect against being frozen again
                self.shield.turn_on();
                unsafe {
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 + HALFPW - 16) as i16,
                        (self.iy as i32 + HALFPH - 16) as i16,
                        3,
                        8,
                    ));
                }
            }
        }
    }

    pub fn accelerate(&mut self, direction: f32) {
        // direction is
        //  1 on moving right
        // -1 on moving left
        debug_assert!(direction == 1.0 || direction == -1.0);

        unsafe {
            if self.onice {
                self.velx += VELMOVINGADDICE * direction;
            } else {
                self.velx += VELMOVINGADD * direction;
            }

            let mut maxVel: f32 = 0.0;
            if !self.frozen {
                let gmTag = gamemode_as::<CGM_Tag>();
                let isTagged = match &gmTag {
                    Some(t) => t.tagged() == Ptr::from_mut(self),
                    None => false,
                };

                if (game_values.flags.slowdownon != -1 && game_values.flags.slowdownon != self.teamID) || self.jail.is_active() {
                    maxVel = VELSLOWMOVING;
                } else if self.keys().game_turbo().fDown {
                    maxVel = VELTURBOMOVING + if isTagged { TAGGEDBOOST } else { 0.0 };
                } else {
                    maxVel = VELMOVING + if isTagged { TAGGEDBOOST } else { 0.0 };
                }
            }
            debug_assert!(maxVel >= 0.0);

            if (direction == 1.0 && self.velx > maxVel) || (direction == -1.0 && self.velx < -maxVel) {
                self.velx = maxVel * direction;
            }

            if !self.inair {
                //Make player hop or stick to ground in Kuribo's shoe
                if self.kuriboshoe.is_on() {
                    //This makes the shoe stick to the ground (for sticky shoes)
                    if self.kuriboshoe.get_type() == STICKY {
                        self.velx = 0.0;
                    } else {
                        //only allow the player to jump in the air from kuribo's shoe if we aren't bouncing on a note block
                        if self.superjumptimer <= 0 {
                            self.jump(direction as i16, 1.0, true);

                            self.superjumptype = 3;
                            self.superjumptimer = 16;
                        }
                    }
                }
                // If the player suddently moved to the other direction, play skid sound
                else if if direction > 0.0 { self.velx < 0.0 } else { self.velx > 0.0 } {
                    game_values.flags.playskidsound = true;
                }

                //If rain candy is turned on
                if (b(g_map.eyecandy[0] as i32 & 32) || b(g_map.eyecandy[1] as i32 & 32) || b(g_map.eyecandy[2] as i32 & 32))
                    && self.velx.abs() > VELMOVINGADD
                    && {
                        self.rainsteptimer += 1;
                        self.rainsteptimer > 7
                    }
                {
                    self.rainsteptimer = 0;
                    eyecandy[1].emplace(EC_SingleAnimation::new_rect(
                        Ptr::from_mut(&mut rm.spr_frictionsmoke),
                        self.ix,
                        (self.iy as i32 + PH - 14) as i16,
                        5,
                        3,
                        0,
                        16,
                        16,
                        16,
                    ));
                }
            }
        }
    }

    pub fn accelerate_right(&mut self) {
        self.accelerate(1.0);
    }

    pub fn accelerate_left(&mut self) {
        self.accelerate(-1.0);
    }

    pub fn decrease_velocity(&mut self) {
        //Stop ground velocity when wearing the sticky shoe
        if !self.inair && self.kuriboshoe.get_type() == STICKY {
            self.velx = 0.0;
            return;
        }

        //Add air/ground friction
        if self.velx > 0.0 {
            if self.inair {
                self.velx -= VELAIRFRICTION;
            } else if self.onice {
                self.velx -= VELICEFRICTION;
            } else {
                self.velx -= VELMOVINGFRICTION;
            }

            if self.velx < 0.0 {
                self.velx = 0.0;
            }
        } else if self.velx < 0.0 {
            if self.inair {
                self.velx += VELAIRFRICTION;
            } else if self.onice {
                self.velx += VELICEFRICTION;
            } else {
                self.velx += VELMOVINGFRICTION;
            }

            if self.velx > 0.0 {
                self.velx = 0.0;
            }
        }
    }

    pub fn can_super_stomp(&self) -> bool {
        self.inair && self.isready() && !self.superstomp.is_in_super_stomp_state()
    }

    pub fn wants_to_super_stomp(&self) -> bool {
        (self.keys().game_down().fPressed && self.playerDevice == DEVICE_KEYBOARD) || (self.keys().game_jump().fPressed && self.keys().game_down().fDown)
    }

    pub fn high_jumped(&self) -> bool {
        self.superjumptype != 3 || self.superjumptimer <= 0
    }

    pub fn is_invincible(&self) -> bool {
        self.invincibility.is_on()
    }

    pub fn is_shielded(&self) -> bool {
        self.shield.is_on()
    }

    pub fn is_invincible_on_bottom(&self) -> bool {
        self.is_invincible() || self.is_shielded() || self.kuriboshoe.is_on()
    }

    pub fn is_super_stomping(&self) -> bool {
        self.superstomp.is_stomping()
    }

    pub fn score(&mut self) -> &mut CScore {
        debug_assert!(!self.score.is_null());
        self.score.get()
    }

    pub fn set_corpse_type(&mut self, r#type: i16) {
        self.diedas = r#type;
    }

    pub fn update_waiting_for_respawn(&mut self) {
        debug_assert!(self.state == PlayerState::Waiting);

        unsafe {
            //use 31 frames to do 0.5 second increments
            if *self.respawncounter.get() > 0 && {
                self.waittimer += 1;
                self.waittimer >= 31
            } {
                self.waittimer = 0;
                *self.respawncounter.get() -= 1;
            }

            if *self.respawncounter.get() <= 0 {
                *self.respawncounter.get() = 0;

                if !net_random::event(Ev::Respawn, &[self.globalID as i32]) {
                    self.net_respawn();
                }
            }
        }
    }

    pub fn net_respawn(&mut self) {
        unsafe {
            if self.find_spawn_point() {
                //Make sure spawn point isn't inside a tile
                let this = self.this();
                self.collisions.checksides(this.get());

                self.state = PlayerState::Spawning;

                if game_values.spawnstyle == SpawnStyle::Instant {
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 + HALFPW - 16) as i16,
                        (self.iy as i32 + HALFPH - 16) as i16,
                        3,
                        8,
                    ));
                } else if game_values.spawnstyle == SpawnStyle::Door {
                    eyecandy[0].emplace(EC_Door::new(
                        Ptr::from_mut(&mut rm.spr_spawndoor),
                        Ptr::from_mut(&mut self.sprites.get()[self.sprite_state as usize]),
                        (self.ix as i32 + HALFPW - 16) as i16,
                        (self.iy as i32 + HALFPH - 16) as i16,
                        1,
                        self.iSrcOffsetX,
                        self.colorID,
                    ));
                }
            }
        }
    }

    pub fn net_podobo_rain(&mut self) {
        unsafe {
            let numPodobos = (RANDOM_INT(6) + 10) as i16;
            for _iPodobo in 0..numPodobos {
                let x = RANDOM_INT((App::screenWidth as f32 * 0.95f32) as i32) as i16;
                let vy = -((RANDOM_INT(9) as f32) / 2.0f32) - 9.0f32;
                objectcontainer[2].add(Ptr::new_box(MO_Podobo::new(
                    Ptr::from_mut(&mut rm.spr_podobo),
                    Vec2s::new(x, App::screenHeight as i16),
                    vy,
                    self.globalID,
                    self.teamID,
                    self.colorID,
                    false,
                )));
            }
            if_sound_on_play(&mut rm.sfx_thunder);
        }
    }

    pub fn net_throw_boomerang(&mut self, ix: i32, iy: i32, facingRight: bool) {
        unsafe {
            objectcontainer[2].add(Ptr::new_box(MO_Boomerang::new(
                Ptr::from_mut(&mut rm.spr_boomerang),
                Vec2s::new(ix as i16, (iy + HALFPH - 16) as i16),
                4,
                facingRight,
                5,
                self.globalID,
                self.teamID,
                self.colorID,
            )));
            self.projectiles += 1;

            if game_values.boomeranglimit > 0 {
                self.decrease_projectile_limit();
            }
        }
    }

    pub fn net_throw_bomb(&mut self, ix: i32, iy: i32, facingRight: bool) {
        let this = self.this();
        unsafe {
            let ttl = (RANDOM_INT(120) + 120) as i16;
            let mut bomb = Ptr::new_box(CO_Bomb::new(
                Ptr::from_mut(&mut rm.spr_bomb),
                Vec2s::new((ix + HALFPW - 14) as i16, (iy - 8) as i16),
                Vec2f::new(if facingRight { 3.0 } else { -3.0 }, -3.0),
                4,
                self.globalID,
                self.teamID,
                self.colorID,
                ttl,
            ));

            if self.accept_item(bomb.get().as_carried_ptr()) {
                bomb.owner = this;
                bomb.get().move_to_owner();
            }

            objectcontainer[1].add(bomb);
            self.projectiles += 1;

            self.hammertimer = 90;

            if_sound_on_play(&mut rm.sfx_fireball);

            if game_values.bombslimit > 0 {
                self.decrease_projectile_limit();
            }
        }
    }

    pub fn is_entering_warp(&self) -> bool {
        matches!(self.state, PlayerState::EnteringWarpUp | PlayerState::EnteringWarpRight | PlayerState::EnteringWarpDown | PlayerState::EnteringWarpLeft)
    }

    pub fn net_choose_warp_exit(&mut self) {
        let this = self.this();
        self.warpstatus.choose_warp_exit(this.get());
    }

    pub fn update_respawning(&mut self) {
        debug_assert!(self.isspawning());

        unsafe {
            match game_values.spawnstyle {
                SpawnStyle::Instant => {
                    self.state = PlayerState::Ready;
                }
                SpawnStyle::Door => {
                    //Wait for door eyecandy to open to let mario out (20 frames for door to appear and 16 frames to open)
                    self.spawntimer += 1;
                    if self.spawntimer > 36 {
                        self.spawntimer = 0;
                        self.state = PlayerState::Ready;
                    }
                }
                SpawnStyle::Swirl => {
                    self.spawntimer += 1;
                    if self.spawntimer >= 50 {
                        self.state = PlayerState::Ready;
                    } else if self.spawntimer % 2 != 0 {
                        let swirlindex = (self.spawntimer >> 1) as usize;
                        let ixoffset = (self.ix as i32 - PWOFFSET) as i16;
                        let iyoffset = (self.iy as i32 - PHOFFSET) as i16;
                        let iColorIdOffset = ((self.colorID as i32) << 5) as i16;

                        for iSwirl in 0..4usize {
                            eyecandy[2].emplace(EC_SingleAnimation::new_rect(
                                Ptr::from_mut(&mut rm.spr_spawnsmoke),
                                (ixoffset as i32 + g_iSwirlSpawnLocations[iSwirl][0][swirlindex] as i32) as i16,
                                (iyoffset as i32 + g_iSwirlSpawnLocations[iSwirl][1][swirlindex] as i32) as i16,
                                4,
                                4,
                                0,
                                iColorIdOffset,
                                32,
                                32,
                            ));
                        }
                    }
                }
            }
        }
    }

    pub fn trigger_powerup(&mut self) {
        let this = self.this();
        unsafe {
            match self.powerupused.unwrap() {
                PowerupType::PoisonMushroom => {}
                PowerupType::ExtraLife1 => {
                    game_values.gamemode.get().playerextraguy(this, 1);
                    if_sound_on_play(&mut rm.sfx_extraguysound);
                }
                PowerupType::ExtraLife2 => {
                    game_values.gamemode.get().playerextraguy(this, 2);
                    if_sound_on_play(&mut rm.sfx_extraguysound);
                }
                PowerupType::ExtraLife3 => {
                    game_values.gamemode.get().playerextraguy(this, 3);
                    if_sound_on_play(&mut rm.sfx_extraguysound);
                }
                PowerupType::ExtraLife5 => {
                    game_values.gamemode.get().playerextraguy(this, 5);
                    if_sound_on_play(&mut rm.sfx_extraguysound);
                }
                PowerupType::Fire => {
                    self.powerup = -1;
                    self.set_powerup(1);
                }
                PowerupType::Star => {
                    self.invincibility.turn_on(this.get());
                }
                PowerupType::Clock => {
                    self.turnslowdownon();
                    self.outofarena.reset();
                }
                PowerupType::Bobomb => {
                    self.powerup = -1;
                    self.bobomb = false;
                    self.set_powerup(0);
                }
                PowerupType::Pow => {
                    if_sound_on_play(&mut rm.sfx_thunder);
                    game_values.flags.screenshaketimer = 20;
                    game_values.flags.screenshakeplayerid = self.globalID;
                    game_values.flags.screenshaketeamid = self.teamID;
                    game_values.flags.screenshakekillinair = false;
                    game_values.flags.screenshakekillscount = 0;
                }
                PowerupType::BulletBill => {
                    game_values.bulletbilltimer[self.globalID as usize] = 400;
                    game_values.bulletbillspawntimer[self.globalID as usize] = 0;
                }
                PowerupType::Hammer => {
                    self.powerup = -1;
                    self.set_powerup(2);
                }
                PowerupType::ShellGreen => {
                    let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Green, Vec2s::zero(), true, true, true, false));
                    if objectcontainer[1].add(shell) {
                        shell.used_as_stored_powerup(this);
                    }
                }
                PowerupType::ShellRed => {
                    let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Red, Vec2s::zero(), false, true, true, false));
                    if objectcontainer[1].add(shell) {
                        shell.used_as_stored_powerup(this);
                    }
                }
                PowerupType::ShellSpiny => {
                    let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Spiny, Vec2s::zero(), false, false, true, true));
                    if objectcontainer[1].add(shell) {
                        shell.used_as_stored_powerup(this);
                    }
                }
                PowerupType::ShellBuzzy => {
                    let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Buzzy, Vec2s::zero(), false, true, false, false));
                    if objectcontainer[1].add(shell) {
                        shell.used_as_stored_powerup(this);
                    }
                }
                PowerupType::Mod => {
                    if_sound_on_play(&mut rm.sfx_thunder);
                    game_values.flags.screenshaketimer = 20;
                    game_values.flags.screenshakeplayerid = self.globalID;
                    game_values.flags.screenshaketeamid = self.teamID;
                    game_values.flags.screenshakekillinair = true;
                }
                PowerupType::Feather => {
                    self.powerup = -1;
                    self.set_powerup(3);
                }
                PowerupType::MysteryMushroom => {
                    swap_players(self.localID);
                }
                PowerupType::Boomerang => {
                    self.powerup = -1;
                    self.set_powerup(4);
                }
                PowerupType::Tanooki => {
                    self.tanookisuit.on_pickup();
                }
                PowerupType::IceWand => {
                    self.powerup = -1;
                    self.set_powerup(5);
                }
                PowerupType::Podobo => {
                    if !net_random::event(Ev::PodoboRain, &[self.globalID as i32]) {
                        self.net_podobo_rain();
                    }
                }
                PowerupType::Bomb => {
                    self.powerup = -1;
                    self.set_powerup(6);
                }
                PowerupType::Leaf => {
                    self.powerup = -1;
                    self.set_powerup(7);
                }
                PowerupType::PWings => {
                    self.powerup = -1;
                    self.set_powerup(8);
                }
                PowerupType::JailKey => {
                    self.jail.escape(this.get());
                }
            }
        }

        self.powerupused = None;
    }

    pub fn update_use_powerup(&mut self) {
        debug_assert!(self.powerupused.is_some());

        unsafe {
            self.powerupradius -= game_values.storedpowerupdelay as f32 / 2.0f32;
        }
        self.powerupangle += 0.05f32;

        if unsafe { netplay.active } && self.net_waitingForPowerupTrigger {
            return;
        }

        if self.powerupradius < 0.0 {
            self.trigger_powerup();
        }
    }

    pub fn get_player_palette(&self) -> PlayerPalette {
        unsafe {
            if self.is_invincible() {
                return self.invincibility.get_player_palette();
            }
            let this = Ptr::from_raw(self as *const CPlayer as *mut CPlayer);
            if game_values.gamemode.gamemode == game_mode_star {
                let starmode = gamemode_as::<CGM_Star>().unwrap();
                let starmodetype = starmode.getcurrentmodetype();
                if starmodetype != StarStyle::Multi && starmode.isplayerstar(this) {
                    return if starmodetype == StarStyle::Ztar { gfx_palette::ztarred } else { gfx_palette::got_shine };
                }
            }
            if let Some(gmTag) = gamemode_as::<CGM_Tag>() {
                if gmTag.tagged() == this {
                    return gfx_palette::tagged;
                }
            }
            if self.frozen {
                return gfx_palette::frozen;
            }
            if self.is_shielded() {
                return gfx_palette::shielded;
            }
            gfx_palette::normal
        }
    }

    pub fn try_falling_through_platform(&mut self, movement_direction: i16) {
        unsafe {
            //only if on the ground and the jump key was released somewhen after it was pressed the last time
            let mut fFellThrough = false;
            if self.keys().game_down().fDown {
                //Check to see what the player is standing on
                self.fPrecalculatedY = self.fy + self.vely;
                let under_tile_y = ((self.fPrecalculatedY as i16 as i32 + PH) / TILESIZE) as i16;

                let mut left_tile_x = (self.ix as i32 / TILESIZE) as i16;
                if (left_tile_x as i32) < 0 {
                    left_tile_x = (left_tile_x as i32 + MAPWIDTH) as i16;
                } else if left_tile_x as i32 >= MAPWIDTH {
                    left_tile_x = (left_tile_x as i32 - MAPWIDTH) as i16;
                }

                let right_tile_x: i16 = if self.right_x() as i32 >= App::screenWidth {
                    ((self.right_x() as i32 - App::screenWidth) / TILESIZE) as i16
                } else {
                    (self.right_x() as i32 / TILESIZE) as i16
                };

                let mut lefttile = g_map.map(left_tile_x as i32, under_tile_y as i32);
                let mut righttile = g_map.map(right_tile_x as i32, under_tile_y as i32);

                if (b(lefttile & tile_flag_solid_on_top) && (b(righttile & tile_flag_solid_on_top) || righttile == tile_flag_nonsolid || righttile == tile_flag_gap))
                    || (b(righttile & tile_flag_solid_on_top) && (b(lefttile & tile_flag_solid_on_top) || lefttile == tile_flag_nonsolid || lefttile == tile_flag_gap))
                {
                    fFellThrough = true;
                }

                if !fFellThrough && !self.platform.is_null() {
                    self.fPrecalculatedY += self.platform.fOldVelY;
                    let this = self.this();
                    self.platform.get().get_tile_types_from_player(this, &mut lefttile, &mut righttile);

                    if (b(lefttile & tile_flag_solid_on_top) && (b(righttile & tile_flag_solid_on_top) || righttile == tile_flag_nonsolid || righttile == tile_flag_gap))
                        || (b(righttile & tile_flag_solid_on_top) && (b(lefttile & tile_flag_solid_on_top) || lefttile == tile_flag_nonsolid || lefttile == tile_flag_gap))
                    {
                        fFellThrough = true;
                    }
                }
            }

            if fFellThrough {
                self.lockfall = true;
                self.fallthrough = true;
            } else {
                self.jump(movement_direction, 1.0, false);
                if_sound_on_play(&mut rm.sfx_jump);
            }

            self.lockjump = true;
        }
    }

    pub fn try_super_jumping(&mut self, movement_direction: i16) {
        debug_assert!(self.superjumptype > 0);

        unsafe {
            if self.superjumptype == 3 {
                //Kuribo's Shoe Jump
                self.jump(movement_direction, 1.0, false);
                if_sound_on_play(&mut rm.sfx_jump);
            }
            if self.superjumptype == 2 {
                self.vely = -VELSUPERJUMP;
                self.inair = true;
                if_sound_on_play(&mut rm.sfx_superspring);
            } else if self.superjumptype == 1 {
                self.vely = -VELTURBOJUMP;
                self.inair = true;
                if_sound_on_play(&mut rm.sfx_springjump);
            }
        }

        self.superjumptimer = 0;
        self.lockjump = true;
    }

    pub fn try_cape_double_jump(&mut self, movement_direction: i16) {
        debug_assert!(self.powerup == 3);
        if self.kuriboshoe.is_on() {
            return;
        }

        unsafe {
            if self.extrajumps < game_values.featherjumps {
                if game_values.featherlimit == 0 || self.projectilelimit > 0 {
                    if self.extrajumps < game_values.featherjumps {
                        self.jump(movement_direction, 0.8, false);
                        if_sound_on_play(&mut rm.sfx_capejump);
                        self.lockjump = true;
                    }

                    self.extrajumps += 1;
                }

                if game_values.featherlimit > 0 {
                    self.decrease_projectile_limit();
                }
            }
        }
    }

    pub fn try_start_flying(&mut self) {
        debug_assert!(self.powerup == 8);
        if self.kuriboshoe.is_on() || self.flying || self.extrajumps != 0 {
            return;
        }

        self.flying = true;
        unsafe {
            game_values.flags.playflyingsound = true;
        }

        self.lockjump = true;
        self.extrajumps += 1;
    }

    pub fn try_shaking_tail(&mut self) {
        if self.kuriboshoe.is_on() || self.spin.is_spin_in_progress() {
            return;
        }

        unsafe {
            let mut isGlidingChicken = false;
            if let Some(gmChicken) = gamemode_as::<CGM_Chicken>() {
                isGlidingChicken = gmChicken.chicken() == Ptr::from_mut(self) && game_values.gamemodesettings.chicken.glide && self.powerup == -1;
            }

            if self.powerup == 7 || isGlidingChicken {
                if game_values.leaflimit == 0 || self.projectilelimit > 0 {
                    let this = self.this();
                    self.tail.shake(this.get());
                }
            }
        }
    }

    pub fn try_releasing_powerup(&mut self) {
        unsafe {
            if self.tanookisuit.is_statue() || game_values.gamemode.gamemode == game_mode_bonus {
                return;
            }

            // Don't allow usage of the poison powerup, it sticks with you and don't allow shyguys to use powerups
            if game_values.gamepowerups[self.globalID as usize] <= 0 || self.shyguy {
                return;
            }

            // Don't allow releasing another powerup when you're in the middle of releasing one
            if self.powerupused.is_some() {
                return;
            }

            self.powerupused = Some(PowerupType::from_u8(game_values.gamepowerups[self.globalID as usize] as u8));
            game_values.gamepowerups[self.globalID as usize] = -1;

            self.powerupradius = 100.0;
            if netplay.active {
                self.powerupangle = 0.0;
            } else {
                self.powerupangle = RANDOM_INT(1000) as f32 * 0.00628f32;
            }

            if_sound_on_play(&mut rm.sfx_storedpowerupsound);

            // only the locally controlled player should be able to send requests
            if netplay.active && self.globalID == netplay.remotePlayerNumber as i16 {
                if netplay.theHostIsMe {
                    // this happens when the game host player triggers a powerup
                    netplay.client.local_gamehost.send_powerup_start_by_gh();

                    // otherwise, when a remote client presses the trigger button,
                    // it sends a powerup trigger request, so we don't have to handle
                    // this on the host side here
                } else {
                    self.net_waitingForPowerupTrigger = true;
                    netplay.client.send_powerup_request();
                }
            }
        }
    }

    pub fn use_special_powerup(&mut self) {
        self.fAcceptingItem = self.carriedItem.is_null();

        unsafe {
            if !self.lockfire {
                if self.bobomb {
                    //If we're a bob-omb, explode
                    self.action = PlayerAction::Bobomb;
                } else if self.powerup == 1 && self.projectiles < 2 {
                    if game_values.fireballlimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::Fireball;
                    }
                } else if self.powerup == 2 && self.projectiles < 2 && self.hammertimer == 0 {
                    if game_values.hammerlimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::Hammer;
                    }
                } else if self.powerup == 3 && !self.spin.is_spin_in_progress() && !self.kuriboshoe.is_on() {
                    if game_values.featherlimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::SpinCape;
                    }
                } else if self.powerup == 4 && self.projectiles < 1 {
                    //only allow one boomerang
                    if game_values.boomeranglimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::Boomerang;
                    }
                } else if self.powerup == 5 && self.projectiles < 1 {
                    if game_values.wandlimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::Iceblast;
                    }
                } else if self.powerup == 6 && self.projectiles < 2 && self.hammertimer == 0 {
                    if game_values.bombslimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::Bomb;
                    }
                } else if self.powerup == 7 && !self.spin.is_spin_in_progress() && !self.kuriboshoe.is_on() {
                    //Racoon tail spin
                    if game_values.leaflimit == 0 || self.projectilelimit > 0 {
                        self.action = PlayerAction::SpinTail;
                    }
                }

                self.lockfire = true;
            }
        }
    }

    pub fn release_carried_item(&mut self) {
        self.lockfire = false;
        self.fAcceptingItem = false;

        if !self.carriedItem.is_null() {
            let item = self.carriedItem.get();
            if self.keys().game_down().fDown {
                MO_CarriedObjectTrait::drop(item);
            } else {
                //Make sure the owner of the object we are kicking is this player
                item.owner = Ptr::from_mut(self);

                //Make sure the shell/block is out in front of player before kicking it
                if item.get_moving_object_type() == movingobject_shell || item.get_moving_object_type() == movingobject_throwblock {
                    item.move_to_owner();
                }

                item.kick();
            }

            self.carriedItem = Ptr::null();
        }
    }

    pub fn update_flying_status(&mut self) {
        if self.flying {
            //If they player was frozen while flying, cause them to stop flying
            //or if they have been flying for a while, stop them flying
            if self.frozen || {
                self.flyingtimer += 1;
                self.flyingtimer > 200
            } {
                self.flyingtimer = 0;
                self.flying = false;

                if unsafe { game_values.pwingslimit } > 0 {
                    self.decrease_projectile_limit();
                }
            } else {
                //otherwise allow them to rise and swoop while flying
                if self.keys().game_down().fDown && self.vely < 1.0 {
                    self.vely += 1.0;
                } else if !self.keys().game_down().fDown && self.vely > -1.0 {
                    self.vely -= 1.5;
                    self.inair = true;
                }
            }
        }
    }

    pub fn enable_free_fall(&mut self) {
        self.lockjump = false; //the jump key is not pressed: the player may jump again if he is on the ground

        if self.vely < -VELSTOPJUMP {
            self.vely = -VELSTOPJUMP;
        }

        if self.flying {
            self.flying = false;
            self.flyingtimer = 0;

            if unsafe { game_values.pwingslimit } > 0 {
                self.decrease_projectile_limit();
            }
        }
    }

    pub fn r#move(&mut self) {
        let this = self.this();
        unsafe {
            //Call the AI if cpu controlled
            if self.state == PlayerState::Ready {
                if self.pPlayerAI.is_some() {
                    //Calculate movement every 4th frame (speed up optimization)
                    if game_values.cputurn == self.globalID {
                        self.cpu_think();

                        if self.keys().game_jump().fDown || self.keys().game_left().fDown || self.keys().game_right().fDown {
                            self.suicidetimer.reset();
                        }
                    }
                    //Let go of the jump button so that we clear "lockjump" so we can jump again when we hit the ground if we want to
                    if self.inair && self.vely > 0.0 && (self.powerup != 3 || self.lockjump) {
                        self.keys().game_jump_mut().fDown = false;
                    }
                }
            }

            if self.is_invincible() {
                game_values.flags.playinvinciblesound = true;
            }

            if self.flying {
                game_values.flags.playflyingsound = true;
            }

            let k = self.keys();
            let keymask: i32 = (if k.game_jump().fPressed { 1 } else { 0 })
                | (if k.game_down().fPressed { 2 } else { 0 })
                | (if k.game_left().fPressed { 4 } else { 0 })
                | (if k.game_right().fPressed { 8 } else { 0 })
                | (if k.game_turbo().fPressed { 16 } else { 0 })
                | (if k.game_powerup().fPressed { 32 } else { 0 });

            let keymaskdown: i32 = (if k.game_jump().fDown { 1 } else { 0 })
                | (if k.game_down().fDown { 2 } else { 0 })
                | (if k.game_left().fDown { 4 } else { 0 })
                | (if k.game_right().fDown { 8 } else { 0 })
                | (if k.game_turbo().fDown { 16 } else { 0 })
                | (if k.game_powerup().fDown { 32 } else { 0 });

            //If any key was pressed, reset the suicide timer
            if keymask != 0 || (keymaskdown != 0 && (self.velx < -1.0 || self.velx > 1.0)) {
                self.suicidetimer.reset();
            }

            self.secretcode.update(this.get(), keymask as u8);
            self.kuriboshoe.update(this.get(), keymask as u8);
            self.cardcollection.update(this.get(), keymask as u8);
            self.tanookisuit.update(this.get());
            self.superstomp.update_on_ground_hit(this.get());

            if self.hammertimer > 0 {
                self.hammertimer -= 1;
            }

            self.spin.update(this.get());

            if self.throw_star > 0 {
                self.throw_star -= 1;
            }

            if self.iSuicideCreditPlayerID >= 0 {
                self.iSuicideCreditTimer -= 1;
                if self.iSuicideCreditTimer <= 0 {
                    self.iSuicideCreditTimer = 0;
                    self.iSuicideCreditPlayerID = -1;
                }
            }

            self.iSrcOffsetX = (32 * self.get_player_palette()) as i16;

            if !self.isready() {
                if self.state == PlayerState::Waiting {
                    self.update_waiting_for_respawn();
                    return;
                }

                if self.isspawning() {
                    self.update_respawning();
                } else if self.iswarping() {
                    self.warpstatus.update(this.get());
                }
            } else if self.powerupused.is_some() {
                self.update_use_powerup();
            }

            self.invincibility.update(this.get()); // Animate invincibility

            self.update_frozen_status(keymask);

            //if player is warping or spawning don't pay attention to controls
            if self.isready() {
                //Super stomp
                self.superstomp.update(this.get());

                //If player is shaking tail, slow decent
                self.tail.slow_descent(this.get());

                self.shield.update();

                let mut movement_direction: i16 = 0; //move left-right-no: -1.. left 0 no 1 ... right

                //Used for bouncing off of note blocks
                if self.superjumptimer > 0 {
                    self.superjumptimer -= 1;
                }

                if !self.frozen {
                    //Determine move direction pressed
                    if self.tanookisuit.not_statue() && !self.superstomp.is_stomping() {
                        if self.keys().game_right().fDown {
                            movement_direction = 1;
                        }

                        if self.keys().game_left().fDown {
                            movement_direction = -1;
                        }
                    }

                    //jump pressed?
                    if self.keys().game_jump().fDown {
                        if !self.lockjump && self.tanookisuit.not_statue() && !self.superstomp.is_stomping() {
                            if !self.inair && self.superjumptimer == 0 {
                                self.try_falling_through_platform(movement_direction);
                            } else if self.superjumptimer > 0 {
                                self.try_super_jumping(movement_direction);
                            } else if self.powerup == 3 {
                                self.try_cape_double_jump(movement_direction);
                            } else if self.powerup == 8 {
                                //Start pwings flight
                                self.try_start_flying();
                            }
                            //This must come last or gliding chickens can't use powerups before this statement
                            else {
                                self.try_shaking_tail();
                            }
                        }
                    } else {
                        self.enable_free_fall();
                    }

                    if self.keys().game_down().fDown {
                        if !self.lockfall && !self.inair && (self.playerDevice == DEVICE_KEYBOARD || self.pPlayerAI.is_some()) {
                            self.lockfall = true;
                            self.fallthrough = true;
                        }
                    } else {
                        self.lockfall = false;
                    }

                    //POWERUP RELEASE
                    if self.keys().game_powerup().fDown {
                        self.try_releasing_powerup();
                    }

                    self.fPressedAcceptItem = self.keys().game_turbo().fPressed;

                    //Projectiles
                    if self.keys().game_turbo().fDown {
                        self.use_special_powerup();
                    } else {
                        self.release_carried_item();
                    }
                }

                self.update_flying_status();

                if movement_direction == 1 {
                    self.accelerate_right();
                } else if movement_direction == -1 {
                    self.accelerate_left();
                } else {
                    self.decrease_velocity();
                }

                if !self.inair {
                    if game_values.unlocksecret3part2[self.globalID as usize] > 0 {
                        game_values.unlocksecret3part2[self.globalID as usize] -= 1;
                    }
                }

                self.fOldX = self.fx;
                self.fOldY = self.fy;

                if game_values.windaffectsplayers {
                    self.oldvelx = self.velx;
                    let windx: f32 = game_values.flags.gamewindx / if self.kuriboshoe.is_on() { 3.0f32 } else { 1.5f32 };
                    self.velx = cap_side_velocity(self.velx + windx);
                }

                self.collision_detection_map();

                //If the player died or entered a warp, don't reset his velocity
                if game_values.windaffectsplayers && self.state == PlayerState::Ready {
                    self.velx = self.oldvelx;
                }
            }

            //Player can be killed by map so only do this code if he is still living
            if self.isready() {
                //Deal with terminal burnup velocity
                self.burnup.update(this.get());
                if self.isdead() {
                    return;
                }

                //Kill the player if he is standing still for too long
                self.suicidetimer.update(this.get());
                if self.isdead() {
                    return;
                }

                //Deal with out of arena timer
                self.outofarena.update(this.get());
                if self.isdead() {
                    return;
                }

                //Deal with release from jail timer
                self.jail.update(this.get());
            }

            self.update_sprite();
        }
    }

    pub fn commit_action(&mut self) {
        let this = self.this();
        unsafe {
            let ix = self.ix as i32;
            let iy = self.iy as i32;
            if PlayerAction::Bobomb == self.action {
                self.bobomb = false;
                objectcontainer[2].add(Ptr::new_box(MO_Explosion::new(
                    Ptr::from_mut(&mut rm.spr_explosion),
                    Vec2s::new((ix + HALFPW - 96) as i16, (iy + HALFPH - 64) as i16),
                    2,
                    4,
                    self.globalID,
                    self.teamID,
                    KillStyle::Bobomb,
                )));
                if_sound_on_play(&mut rm.sfx_bobombsound);
            } else if PlayerAction::Fireball == self.action {
                objectcontainer[0].add(Ptr::new_box(MO_Fireball::new(
                    Ptr::from_mut(&mut rm.spr_fireball),
                    Vec2s::new((ix + 6) as i16, iy as i16),
                    4,
                    self.is_facing_right(),
                    5,
                    self.globalID,
                    self.teamID,
                    self.colorID,
                )));
                if_sound_on_play(&mut rm.sfx_fireball);

                self.projectiles += 1;

                if game_values.fireballlimit > 0 {
                    self.decrease_projectile_limit();
                }
            } else if PlayerAction::Hammer == self.action {
                let vx = if game_values.reversewalk { -self.velx } else { self.velx };
                if self.is_facing_right() {
                    objectcontainer[2].add(Ptr::new_box(MO_Hammer::new(
                        Ptr::from_mut(&mut rm.spr_hammer),
                        Vec2s::new((ix + 8) as i16, iy as i16),
                        6,
                        Vec2f::new(vx + 2.0f32, -HAMMERTHROW),
                        5,
                        self.globalID,
                        self.teamID,
                        self.colorID,
                        false,
                    )));
                } else {
                    objectcontainer[2].add(Ptr::new_box(MO_Hammer::new(
                        Ptr::from_mut(&mut rm.spr_hammer),
                        Vec2s::new((ix - 14) as i16, iy as i16),
                        6,
                        Vec2f::new(vx - 2.0f32, -HAMMERTHROW),
                        5,
                        self.globalID,
                        self.teamID,
                        self.colorID,
                        false,
                    )));
                }
                self.projectiles += 1;

                self.hammertimer = game_values.hammerdelay;
                if_sound_on_play(&mut rm.sfx_fireball);

                if game_values.hammerlimit > 0 {
                    self.decrease_projectile_limit();
                }
            } else if PlayerAction::Boomerang == self.action {
                let args = [self.globalID as i32, ix, iy, self.is_facing_right() as i32];
                if game_values.boomerangstyle != BoomerangStyle::Random || !net_random::event(Ev::Boomerang, &args) {
                    self.net_throw_boomerang(ix, iy, self.is_facing_right());
                }
            } else if PlayerAction::Iceblast == self.action {
                if self.is_facing_right() {
                    objectcontainer[2].add(Ptr::new_box(MO_IceBlast::new(
                        Ptr::from_mut(&mut rm.spr_iceblast),
                        Vec2s::new((ix + HALFPW - 2) as i16, (iy + HALFPH - 16) as i16),
                        5.0,
                        self.globalID,
                        self.teamID,
                        self.colorID,
                    )));
                } else {
                    objectcontainer[2].add(Ptr::new_box(MO_IceBlast::new(
                        Ptr::from_mut(&mut rm.spr_iceblast),
                        Vec2s::new((ix + HALFPW - 30) as i16, (iy + HALFPH - 16) as i16),
                        -5.0,
                        self.globalID,
                        self.teamID,
                        self.colorID,
                    )));
                }

                self.projectiles += 1;

                if_sound_on_play(&mut rm.sfx_wand);

                if game_values.wandlimit > 0 {
                    self.decrease_projectile_limit();
                }
            } else if PlayerAction::Bomb == self.action {
                if !net_random::event(Ev::Bomb, &[self.globalID as i32, ix, iy, self.is_facing_right() as i32]) {
                    self.net_throw_bomb(ix, iy, self.is_facing_right());
                }
            } else if PlayerAction::SpinCape == self.action {
                self.cape.spin(this.get());
            } else if PlayerAction::SpinTail == self.action {
                self.tail.spin(this.get());
            }

            if self.action == PlayerAction::Fireball {
                game_values.unlocksecret3part1[self.globalID as usize] += 1;
                check_secret(2);
            } else if self.action != PlayerAction::None {
                game_values.unlocksecret3part1[self.globalID as usize] = 0;
            }
        }

        self.action = PlayerAction::None;
    }

    pub fn update_sprite(&mut self) {
        unsafe {
            let iReverseSprite: i32 = if game_values.reversewalk { 1 } else { 0 };

            //Use correct sprite (and animate)
            if should_update_sprite() {
                //if player is warping from below, set them in the air
                if self.iswarping() {
                    if self.state == PlayerState::LeavingWarpDown || self.state == PlayerState::EnteringWarpUp {
                        self.inair = true;
                    } else {
                        self.inair = false;
                    }
                }

                //lockjump is true when we are in the air (even if we fell of an edge)
                if self.spin.is_spin_in_progress() {
                    self.sprite_state = self.spin.to_sprite_id();
                } else if self.state == PlayerState::Spawning {
                    if self.sprite_state & 0x1 == 0 {
                        self.sprite_state = (PGFX_JUMPING_R + iReverseSprite) as u8;
                    } else {
                        self.sprite_state = (PGFX_JUMPING_L - iReverseSprite) as u8;
                    }
                } else if self.inair {
                    self.frictionslidetimer = 0;
                    self.rainsteptimer = 0;

                    if self.is_facing_right() {
                        self.sprite_state = PGFX_JUMPING_R as u8;
                    } else {
                        self.sprite_state = PGFX_JUMPING_L as u8;
                    }
                } else if self.velx > 0.0 {
                    if self.keys().game_left().fDown && !self.keys().game_right().fDown && self.state == PlayerState::Ready {
                        self.sprite_state = (PGFX_STOPPING_R + iReverseSprite) as u8;

                        self.frictionslidetimer += 1;
                        if self.frictionslidetimer > 3 {
                            self.frictionslidetimer = 0;
                            eyecandy[1].emplace(EC_SingleAnimation::new_rect(
                                Ptr::from_mut(&mut rm.spr_frictionsmoke),
                                self.ix,
                                (self.iy as i32 + PH - 12) as i16,
                                4,
                                4,
                                0,
                                0,
                                16,
                                16,
                            ));
                        }
                    } else if self.onice && !self.keys().game_right().fDown && !self.keys().game_left().fDown {
                        self.sprite_state = (PGFX_STANDING_R + iReverseSprite) as u8;
                    } else {
                        self.sprswitch -= 1;
                        if self.sprswitch < 1 {
                            if game_values.reversewalk {
                                if self.sprite_state as i32 == PGFX_STANDING_L {
                                    self.sprite_state = PGFX_RUNNING_L as u8;
                                } else {
                                    self.sprite_state = PGFX_STANDING_L as u8;
                                }
                            } else if self.sprite_state as i32 == PGFX_STANDING_R {
                                self.sprite_state = PGFX_RUNNING_R as u8;
                            } else {
                                self.sprite_state = PGFX_STANDING_R as u8;
                            }

                            self.sprswitch = 4;
                        } else {
                            //If animation timer hasn't fired, make sure we're facing the correct direction
                            if game_values.reversewalk {
                                if self.sprite_state & 0x1 == 0 {
                                    self.sprite_state = PGFX_STANDING_L as u8;
                                }
                            } else if self.sprite_state & 0x1 != 0 {
                                self.sprite_state = PGFX_STANDING_R as u8;
                            }
                        }
                    }
                } else if self.velx < 0.0 {
                    if self.keys().game_right().fDown && !self.keys().game_left().fDown && self.state == PlayerState::Ready {
                        self.sprite_state = (PGFX_STOPPING_L - iReverseSprite) as u8;

                        self.frictionslidetimer += 1;
                        if self.frictionslidetimer > 3 {
                            self.frictionslidetimer = 0;
                            eyecandy[1].emplace(EC_SingleAnimation::new_rect(
                                Ptr::from_mut(&mut rm.spr_frictionsmoke),
                                (self.ix as i32 + PW - 16) as i16,
                                (self.iy as i32 + PH - 12) as i16,
                                4,
                                4,
                                0,
                                0,
                                16,
                                16,
                            ));
                        }
                    } else if self.onice && !self.keys().game_right().fDown && !self.keys().game_left().fDown {
                        self.sprite_state = (PGFX_STANDING_L - iReverseSprite) as u8;
                    } else {
                        self.sprswitch -= 1;
                        if self.sprswitch < 1 {
                            if game_values.reversewalk {
                                if self.sprite_state as i32 == PGFX_STANDING_R {
                                    self.sprite_state = PGFX_RUNNING_R as u8;
                                } else {
                                    self.sprite_state = PGFX_STANDING_R as u8;
                                }
                            } else if self.sprite_state as i32 == PGFX_STANDING_L {
                                self.sprite_state = PGFX_RUNNING_L as u8;
                            } else {
                                self.sprite_state = PGFX_STANDING_L as u8;
                            }

                            self.sprswitch = 4;
                        } else {
                            //If animation timer hasn't fired, make sure we're facing the correct direction
                            if game_values.reversewalk {
                                if self.sprite_state & 0x1 != 0 {
                                    self.sprite_state = PGFX_STANDING_R as u8;
                                }
                            } else if self.sprite_state & 0x1 == 0 {
                                self.sprite_state = PGFX_STANDING_L as u8;
                            }
                        }
                    }
                } else {
                    //standing
                    if self.keys().game_left().fDown {
                        self.sprite_state = (PGFX_STANDING_L - iReverseSprite) as u8;
                    } else if self.keys().game_right().fDown {
                        self.sprite_state = (PGFX_STANDING_R + iReverseSprite) as u8;
                    } else if self.sprite_state & 0x1 != 0 {
                        self.sprite_state = PGFX_STANDING_L as u8;
                    } else {
                        self.sprite_state = PGFX_STANDING_R as u8;
                    }
                }
            } else if game_values.flags.swapplayers {
                let mut iSpriteDirection: i32 = 0;

                if self.fNewSwapX < self.fOldSwapX {
                    iSpriteDirection = 1;
                }

                self.sprswitch -= 1;
                if self.sprswitch < 1 {
                    if self.sprite_state as i32 == PGFX_STANDING_R + iSpriteDirection {
                        self.sprite_state = (PGFX_RUNNING_R + iSpriteDirection) as u8;
                    } else {
                        self.sprite_state = (PGFX_STANDING_R + iSpriteDirection) as u8;
                    }

                    self.sprswitch = 4;
                }
            }
        }
    }

    pub fn jump(&mut self, iMove: i16, jumpModifier: f32, fKuriboBounce: bool) {
        unsafe {
            if fKuriboBounce {
                self.vely = -VELKURIBOBOUNCE;
            } else if (game_values.flags.slowdownon != -1 && game_values.flags.slowdownon != self.teamID) || self.jail.is_active() {
                self.vely = -VELSLOWJUMP * jumpModifier;
            } else if self.velx.abs() > VELMOVING && iMove != 0 && self.keys().game_turbo().fDown {
                self.vely = -VELTURBOJUMP * jumpModifier;
            } else {
                self.vely = -VELJUMP * jumpModifier;
            }
        }

        self.inair = true;

        //Need to help the player off the platform otherwise it will collide with them again
        if !self.platform.is_null() {
            self.platform = Ptr::null();
        }
    }

    pub fn cpu_think(&mut self) {
        let keys = self.playerKeys;
        self.pPlayerAI.as_mut().unwrap().think(keys);
    }

    pub fn die(&mut self, deathStyle: PlayerDeathStyle, fTeamRemoved: bool, fKillCarriedItem: bool) {
        if !fTeamRemoved {
            crate::smw::net_outcomes::note_death(self.globalID, deathStyle as i32, false);
        }
        unsafe {
            let ix = self.ix as i32;
            let iy = self.iy as i32;

            //Only show the death gfx if the player is alive when he died
            //If he is spawning or already dead, then don't show anything
            if self.state >= PlayerState::Dead {
                let iDeathSprite = if deathStyle == PlayerDeathStyle::Jump { PGFX_DEADFLYING } else { PGFX_DEAD } as usize;

                let mut corpseSprite: Ptr<gfxSprite> = Ptr::from_mut(&mut self.sprites.get()[iDeathSprite]);
                let isChicken = match gamemode_as::<CGM_Chicken>() {
                    Some(gmChicken) => gmChicken.chicken() == Ptr::from_mut(self),
                    None => false,
                };

                //If the player was a bobomb or chicken, make sure their death sprite matches
                if self.diedas == 1 || isChicken {
                    corpseSprite = Ptr::from_mut(&mut rm.spr_chocobo[self.colorID as usize][iDeathSprite]);
                } else if self.diedas == 2 || self.bobomb {
                    corpseSprite = Ptr::from_mut(&mut rm.spr_bobomb[self.colorID as usize][iDeathSprite]);
                } else if self.diedas == 3 || self.shyguy {
                    corpseSprite = Ptr::from_mut(&mut rm.spr_shyguy[self.colorID as usize][iDeathSprite]);
                }

                //Add eyecandy for the dead player
                if deathStyle == PlayerDeathStyle::Shatter || self.frozen {
                    let ice = Ptr::from_mut(&mut rm.spr_brokeniceblock);
                    eyecandy[2].emplace(EC_FallingObject::new(ice, (ix + HALFPW - 16) as i16, (iy + HALFPH - 16) as i16, -1.5, -7.0, 4, 2, 0, 0, 16, 16));
                    eyecandy[2].emplace(EC_FallingObject::new(ice, (ix + HALFPW) as i16, (iy + HALFPH - 16) as i16, 1.5, -7.0, 4, 2, 0, 0, 16, 16));
                    eyecandy[2].emplace(EC_FallingObject::new(ice, (ix + HALFPW - 16) as i16, (iy + HALFPH) as i16, -1.5, -4.0, 4, 2, 0, 0, 16, 16));
                    eyecandy[2].emplace(EC_FallingObject::new(ice, (ix + HALFPW) as i16, (iy + HALFPH) as i16, 1.5, -4.0, 4, 2, 0, 0, 16, 16));

                    game_values.unlocksecret2part2 += 1;
                } else if deathStyle == PlayerDeathStyle::Jump {
                    eyecandy[2].emplace(EC_FallingObject::new(
                        corpseSprite,
                        (ix + HALFPW - 16) as i16,
                        (iy + PH - 32) as i16,
                        0.0,
                        -VELTURBOJUMP,
                        1,
                        0,
                        self.iSrcOffsetX,
                        0,
                        32,
                        32,
                    ));
                } else if deathStyle == PlayerDeathStyle::Squish {
                    eyecandy[1].emplace(EC_Corpse::new(corpseSprite, (ix - PWOFFSET) as f32, (iy + PH - 32) as f32, self.iSrcOffsetX));
                }
            }

            //Drop any item the dead player was carrying
            if !self.carriedItem.is_null() {
                if fKillCarriedItem {
                    self.carriedItem.get().kill_object_map_hazard(-1);
                } else {
                    MO_CarriedObjectTrait::drop(self.carriedItem.get());
                }

                self.carriedItem = Ptr::null();
            }

            //Drop a shoe item if the player died in one
            if self.kuriboshoe.is_on() {
                let shoe = Ptr::new_box(CO_KuriboShoe::new(
                    Ptr::from_mut(&mut rm.spr_kuriboshoe),
                    Vec2s::new((ix - PWOFFSET) as i16, (iy - PHOFFSET) as i16),
                    self.kuriboshoe.get_type() == STICKY,
                ));
                io_moving_object_collision_detection_checksides(shoe.get());

                objectcontainer[1].add(shoe);
            }

            //If the game mode didn't say to remove the team from the game, then respawn the player
            if !fTeamRemoved {
                if game_values.screencrunch && !game_values.spinscreen {
                    y_shake = (y_shake as i32 + CRUNCHAMOUNT) as i16;
                }

                if y_shake as i32 > CRUNCHMAX {
                    y_shake = CRUNCHMAX as i16;
                }

                self.setup_new_player();
            }
        }
    }

    // Called by CO_KuriboShoe
    pub fn set_kuribo_shoe(&mut self, r#type: KuriboShoeType) {
        debug_assert!(r#type > 0);
        self.kuriboshoe.set_type(r#type);

        if !self.carriedItem.is_null() && !self.carriedItem.is_carried_by_kuribo_shoe() {
            MO_CarriedObjectTrait::drop(self.carriedItem.get());
            self.carriedItem = Ptr::null();
        }

        //Clear out powerup states that the player might be in the middle of
        self.clear_powerup_states();
    }

    pub fn setup_new_player(&mut self) {
        unsafe {
            self.pScoreboardSprite = self.sprites;
            self.action = PlayerAction::None;

            self.velx = 0.0;
            self.oldvelx = self.velx;

            if game_values.spawnstyle == SpawnStyle::Door {
                self.vely = 0.0;
            } else {
                self.vely = -(VELJUMP / 2.0); //make the player jump up on respawn
            }

            self.fOldX = self.fx;
            self.fOldY = self.fy;

            self.inair = true;
            self.onice = false;
            self.lockjump = true;
            self.superjumptimer = 0;
            self.superjumptype = 0;
            self.projectilelimit = 0;
            self.hammertimer = 0;

            self.frozen = false;
            self.frozentimer = 0;

            self.killsinrow = 0;
            self.killsinrowinair = 0;
            self.extrajumps = 0;

            self.strip_powerups();
            self.clear_powerup_states();

            self.sSpotlight = Ptr::null();

            self.cape.reset();

            self.frictionslidetimer = 0;
            self.bobombsmoketimer = 0;
            self.rainsteptimer = 0;

            self.spawntimer = 0;
            self.waittimer = 0;
            *self.respawncounter.get() = game_values.respawn;
            self.state = PlayerState::Waiting;

            self.fallthrough = false;
            self.diedas = 0;
            self.iSrcOffsetX = 0;

            self.outofarena.reset();

            self.platform = Ptr::null();
            self.iHorizontalPlatformCollision = -1;
            self.iVerticalPlatformCollision = -1;
            self.iPlatformCollisionPlayerId = -1;

            self.shield.reset();
            self.kuriboshoe.reset();
            self.secretcode.reset();
            self.cardcollection.reset();
            self.superstomp.reset();
            self.suicidetimer.reset();

            self.throw_star = 0;

            game_values.unlocksecret3part1[self.globalID as usize] = 0;
            game_values.unlocksecret3part2[self.globalID as usize] = 0;
        }
    }

    pub fn strip_powerups(&mut self) {
        self.bobomb = false;
        self.powerup = -1;

        self.tanookisuit.reset();
        self.invincibility.reset();
        self.powerupused = None;
    }

    pub fn find_spawn_point(&mut self) -> bool {
        unsafe {
            let fRet = g_map.findspawnpoint(self.teamID + 1, &mut self.ix, &mut self.iy, PW as i16, PH as i16, false);

            self.fx = self.ix as f32;
            self.fy = self.iy as f32;

            fRet
        }
    }

    pub fn spawn_text(&mut self, szText: &str) {
        self.spawntext += 1;
        if self.spawntext >= 20 {
            unsafe {
                eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut rm.game_font_large),
                    (self.ix as i32 + HALFPW) as i16,
                    self.iy,
                    szText.to_string(),
                    -VELJUMP,
                ));
            }
            self.spawntext = 0; //spawn text every 20 frames
        }
    }

    pub fn bouncejump(&mut self) -> bool {
        self.superstomp.reset();

        if self.keys().game_jump().fDown {
            self.lockjump = true;
            self.vely = -VELJUMP;
            true
        } else {
            self.vely = -VELJUMP / 2.0; //jump a little off of collision object
            false
        }
    }

    pub fn add_killer_award(&mut self, killed: Ptr<CPlayer>, style: KillStyle) {
        let this = self.this();
        self.awardeffects.add_killer_award(this.get(), killed, style);
    }

    pub fn death_awards(&mut self) {
        let this = self.this();
        self.awardeffects.add_death_award(this.get());
    }

    pub fn add_kills_in_row_in_air_award(&mut self) {
        let this = self.this();
        self.awardeffects.add_kills_in_row_in_air_award(this.get());
    }

    pub fn killed_player(&mut self, killed: Ptr<CPlayer>, deathstyle: PlayerDeathStyle, style: KillStyle, fForce: bool, fKillCarriedItem: bool) -> PlayerKillType {
        let args = [1, self.globalID as i32, killed.globalID as i32, deathstyle as i32, style as i32, fForce as i32, fKillCarriedItem as i32, -1];
        if let Some(result) = crate::smw::net_outcomes::kill(&args) {
            return result;
        }
        self.killed_player_now(killed, deathstyle, style, fForce, fKillCarriedItem)
    }

    pub fn killed_player_now(&mut self, mut killed: Ptr<CPlayer>, mut deathstyle: PlayerDeathStyle, style: KillStyle, fForce: bool, fKillCarriedItem: bool) -> PlayerKillType {
        let mut killer = self.this();

        unsafe {
            //If this player is already dead, then don't kill him again
            if killed.state != PlayerState::Ready {
                return PlayerKillType::None;
            }

            if let Some(gmChicken) = gamemode_as::<CGM_Chicken>() {
                if gmChicken.chicken() == killer && style != KillStyle::Pow {
                    if_sound_on_play(&mut rm.sfx_chicken);
                }
            }

            if killed.frozen {
                deathstyle = PlayerDeathStyle::Shatter;
            }

            if killer.teamID != killed.teamID {
                killer.add_killer_award(killed, style);
            }

            if game_values.awardstyle != AwardStyle::None {
                killed.death_awards();
            }

            //now kill the player (don't call this function earlier because we need the old position, etc.
            let iKillType = game_values.gamemode.get().playerkilledplayer(killer, killed, style);

            if PlayerKillType::NonKill != iKillType || fForce {
                if killed.bobomb {
                    killed.diedas = 2;
                    killer.set_powerup(0);
                }

                if deathstyle == PlayerDeathStyle::Jump {
                    if_sound_on_play(&mut rm.sfx_deathsound);
                } else if deathstyle == PlayerDeathStyle::Squish {
                    if_sound_on_play(&mut rm.sfx_mip);
                } else if deathstyle == PlayerDeathStyle::Shatter {
                    if_sound_on_play(&mut rm.sfx_breakblock);
                }
            }

            if PlayerKillType::Normal == iKillType || (fForce && PlayerKillType::NonKill == iKillType) {
                killed.die(deathstyle, false, fKillCarriedItem);
            }

            iKillType
        }
    }

    pub fn transfer_tag(&mut self, mut o2: Ptr<CPlayer>) {
        let mut o1 = self.this();

        unsafe {
            let gmTag = match gamemode_as::<CGM_Tag>() {
                Some(t) => t,
                None => return,
            };

            if !o1.isready() || !o2.isready() {
                return;
            }

            let mut tagged = Ptr::null();
            if gmTag.tagged() == o1 && !o2.is_shielded() && !o2.is_invincible() {
                gmTag.set_tagged(o2);
                o1.shield.turn_on();
                tagged = gmTag.tagged();
            } else if gmTag.tagged() == o2 && !o1.is_shielded() && !o1.is_invincible() {
                gmTag.set_tagged(o1);
                o2.shield.turn_on();
                tagged = gmTag.tagged();
            }

            if !tagged.is_null() {
                let t: Ptr<CPlayer> = tagged;
                eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut rm.game_font_large),
                    (t.ix as i32 + HALFPW) as i16,
                    (t.iy as i32 + PH) as i16,
                    "Tagged!".to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (t.ix as i32 + HALFPW - 16) as i16,
                    (t.iy as i32 + HALFPH - 16) as i16,
                    3,
                    8,
                ));
                if_sound_on_play(&mut rm.sfx_transform);
            }
        }
    }

    pub fn transfer_shy_guy(&mut self, o2: Ptr<CPlayer>) {
        let o1 = self.this();

        unsafe {
            //Don't shyguy tag if this isn't shyguy tag mode or if tag transfers is set to kills only
            if game_values.gamemode.gamemode != game_mode_shyguytag || game_values.gamemodesettings.shyguytag.tagtransfer == 1 {
                return;
            }

            if !o1.isready() || !o2.isready() {
                return;
            }

            let sgt = gamemode_as::<CGM_ShyGuyTag>().unwrap();

            if o1.shyguy && !o2.shyguy && !o2.is_shielded() && !o2.is_invincible() {
                sgt.set_shy_guy(o2.get_team_id());
            } else if o2.shyguy && !o1.shyguy && !o1.is_shielded() && !o1.is_invincible() {
                sgt.set_shy_guy(o1.get_team_id());
            }
        }
    }

    pub fn bounce_assist_player(&mut self, o2: Ptr<CPlayer>) {
        let mut o1 = self.this();

        if o1.state == PlayerState::Ready
            && o1.fOldY + PH as f32 <= o2.fOldY
            && o1.iy as i32 + PH >= o2.iy as i32
            && o1.keys().game_jump().fDown
        {
            o1.set_yi((o2.iy as i32 - PH) as i16); //set new position to top of other player
            o1.get().collisions.checktop(o1.get());
            o1.platform = Ptr::null();
            o1.vely = -VELSUPERJUMP;

            o1.superstomp.reset();

            unsafe {
                if_sound_on_play(&mut rm.sfx_superspring);
            }
        }
    }

    pub fn draw_spotlight(&mut self) {
        unsafe {
            if game_values.spotlights && self.state != PlayerState::Dead {
                if self.sSpotlight.is_null() {
                    self.sSpotlight = spotlightManager.add_spotlight((self.ix as i32 + HALFPW) as i16, (self.iy as i32 + HALFPH) as i16, 7);
                }

                if !self.sSpotlight.is_null() {
                    self.sSpotlight.get().update_position((self.ix as i32 + HALFPW) as i16, (self.iy as i32 + HALFPH) as i16);
                }
            }
        }
    }

    pub fn draw_powerup_ring(&mut self) {
        if let Some(used) = self.powerupused {
            let numeyecandy: i16 = 8;
            let addangle: f32 = TWO_PI / numeyecandy as f32;
            let mut displayangle: f32 = self.powerupangle;

            for _k in 0..numeyecandy {
                let powerupX = (self.ix as i32 + HALFPW - 8 + (self.powerupradius * displayangle.cos()) as i16 as i32) as i16;
                let powerupY = (self.iy as i32 + HALFPH - 8 + (self.powerupradius * displayangle.sin()) as i16 as i32) as i16;

                displayangle += addangle;

                let src = SDL_Rect { x: used as i32 * 16, y: 0, w: 16, h: 16 };
                unsafe {
                    if self.iswarping() {
                        rm.spr_storedpowerupsmall.draw_clip(powerupX as i32, powerupY as i32, &src, self.clip_edge(), self.get_warp_plane() as i32);
                    } else {
                        rm.spr_storedpowerupsmall.draw_src(powerupX as i32, powerupY as i32, &src);
                    }
                }
            }
        }
    }

    pub fn draw_winner_crown(&mut self) {
        unsafe {
            if game_values.showwinningcrown && g_iWinningPlayer == self.teamID {
                let x = self.ix as i32 + HALFPW - if self.is_facing_right() { 4 } else { 10 };
                let y = self.iy as i32 - 10 - if self.kuriboshoe.is_on() { 16 } else { 0 };
                if self.iswarping() {
                    rm.spr_crown.draw_clip(x, y, &SDL_Rect { x: 0, y: 0, w: 14, h: 14 }, self.clip_edge(), self.get_warp_plane() as i32);
                } else {
                    rm.spr_crown.draw(x, y);
                }
            }
        }
    }

    pub fn draw(&mut self) {
        let this = self.this();

        //Don't draw a player that is waiting to respawn
        if self.state == PlayerState::Waiting {
            return;
        }

        self.draw_spotlight();

        if self.state == PlayerState::Spawning {
            return;
        }

        unsafe {
            //Draw player
            self.pScoreboardSprite = self.sprites;

            let mut chicken: Ptr<CPlayer> = Ptr::null();
            if let Some(gmChicken) = gamemode_as::<CGM_Chicken>() {
                chicken = gmChicken.chicken();
            }

            let ix = self.ix as i32;
            let iy = self.iy as i32;
            let colorID = self.colorID as usize;

            if self.tanookisuit.is_statue() {
                //Make sure the scoreboard still accurately represents the player
                if self.bobomb {
                    self.pScoreboardSprite = Ptr::from_mut(&mut rm.spr_bobomb[colorID]);
                } else if chicken == this {
                    self.pScoreboardSprite = Ptr::from_mut(&mut rm.spr_chocobo[colorID]);
                } else if self.shyguy {
                    self.pScoreboardSprite = Ptr::from_mut(&mut rm.spr_shyguy[colorID]);
                }

                self.tanookisuit.draw_statue(this.get());

                return;
            } else if self.bobomb {
                //draw him as bob-omb
                self.pScoreboardSprite = Ptr::from_mut(&mut rm.spr_bobomb[colorID]);

                //Add smoke to the top of the bomb
                if {
                    self.bobombsmoketimer += 1;
                    self.bobombsmoketimer > 2
                } && (self.velx != 0.0 || self.vely != GRAVITATION)
                    && self.state == PlayerState::Ready
                {
                    self.bobombsmoketimer = 0;
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_bobombsmoke),
                        (ix + HALFPH - 8) as i16,
                        (iy - PHOFFSET - 8) as i16,
                        4,
                        4,
                    ));
                }
            } else if chicken == this {
                //draw him as chicken
                self.pScoreboardSprite = Ptr::from_mut(&mut rm.spr_chocobo[colorID]);
            } else if self.shyguy {
                //draw him as chicken
                self.pScoreboardSprite = Ptr::from_mut(&mut rm.spr_shyguy[colorID]);
                rm.spr_ownedtags.draw_src(ix - PWOFFSET - 8, iy - PHOFFSET - 8, &SDL_Rect { x: self.ownerColorOffsetX as i32, y: 0, w: 48, h: 48 });
            }

            if self.ownerPlayerID > -1 {
                let src = SDL_Rect { x: self.ownerColorOffsetX as i32, y: 0, w: 48, h: 48 };
                if self.iswarping() {
                    rm.spr_ownedtags.draw_clip(ix - PWOFFSET - 8, iy - PHOFFSET - 8, &src, self.clip_edge(), self.get_warp_plane() as i32);
                } else {
                    rm.spr_ownedtags.draw_src(ix - PWOFFSET - 8, iy - PHOFFSET - 8, &src);
                }
            }

            //Don't allow cape, tail, wings to be used with shoe
            if !self.kuriboshoe.is_on() {
                if self.powerup == 3 {
                    self.cape.draw(this.get());
                } else if self.powerup == 8 {
                    self.wings.draw(this.get());
                }
                //This has to come last otherwise chickens with glide option won't be able to use cape or wings
                else if self.powerup == 7 || (self.powerup == -1 && chicken == this && game_values.gamemodesettings.chicken.glide) {
                    self.tail.draw(this.get());
                }
            }

            //Draw the player sprite above the shoe
            let mut iPlayerKuriboOffsetY: i32 = 0;
            if self.kuriboshoe.is_on() {
                iPlayerKuriboOffsetY = 16;
            }

            //Don't draw the player if he is frozen in a shoe
            if !self.frozen || !self.kuriboshoe.is_on() {
                let spr = &self.pScoreboardSprite.get()[self.sprite_state as usize];
                let src = SDL_Rect { x: self.iSrcOffsetX as i32, y: 0, w: 32, h: 32 };
                if self.iswarping() {
                    spr.draw_clip(ix - PWOFFSET, iy - PHOFFSET - iPlayerKuriboOffsetY, &src, self.clip_edge(), self.get_warp_plane() as i32);
                } else {
                    spr.draw_src(ix - PWOFFSET, iy - PHOFFSET - iPlayerKuriboOffsetY, &src);
                }
            }

            //Draw Kuribo's Shoe
            self.kuriboshoe.draw(this.get());

            //Draw the crown on the player
            self.draw_winner_crown();

            if self.state < PlayerState::Ready {
                return;
            }

            if self.frozen {
                let src = SDL_Rect { x: 0, y: 0, w: 32, h: 32 };
                if self.iswarping() {
                    rm.spr_iceblock.draw_clip(ix - PWOFFSET, iy - PHOFFSET, &src, self.clip_edge(), self.get_warp_plane() as i32);
                } else {
                    rm.spr_iceblock.draw_src(ix - PWOFFSET, iy - PHOFFSET, &src);
                }
            }

            self.jail.draw(this.get());
            self.suicidetimer.draw(this.get());

            //Draw the Ring awards
            if game_values.awardstyle == AwardStyle::Halo && self.killsinrow as i32 >= MINAWARDSNEEDED {
                self.awardeffects.draw_ring_award(this.get());
            }

            //Draw the powerup ring when a powerup is being used
            self.draw_powerup_ring();
        }
    }

    pub fn draw_out_of_screen_indicators(&mut self) {
        let this = self.this();
        self.outofarena.draw(this.get());
    }

    pub fn updateswap(&mut self) {
        if self.state != PlayerState::Ready {
            return;
        }

        unsafe {
            if game_values.swapstyle == 1 {
                self.set_xf(if game_values.flags.swapplayersblink { self.fOldSwapX } else { self.fNewSwapX });
                self.set_yf(if game_values.flags.swapplayersblink { self.fOldSwapY } else { self.fNewSwapY });
            } else {
                self.set_xf(((self.fNewSwapX - self.fOldSwapX) * game_values.flags.swapplayersposition) + self.fOldSwapX);
                self.set_yf(((self.fNewSwapY - self.fOldSwapY) * game_values.flags.swapplayersposition) + self.fOldSwapY);
            }
        }

        if !self.carriedItem.is_null() {
            self.carriedItem.get().move_to_owner();
        }
    }

    pub fn drawswap(&mut self) {
        if self.state != PlayerState::Ready {
            self.draw();
            return;
        }

        if unsafe { game_values.swapstyle } != 1 {
            self.update_sprite();
        }

        self.draw();

        if !self.carriedItem.is_null() {
            self.carriedItem.get().draw();
        }
    }

    pub fn mapcolldet_handle_platform_velocity(&mut self, fPlatformVelX: &mut f32, fPlatformVelY: &mut f32) {
        if !self.platform.is_null() {
            *fPlatformVelX = self.platform.fVelX;
            self.set_xf(self.fx + *fPlatformVelX);
            self.flipsidesifneeded();

            *fPlatformVelY = self.platform.fVelY;

            if self.platform.fOldVelY < 0.0 {
                self.fy += self.platform.fOldVelY;
            }

            self.fPrecalculatedY += self.platform.fOldVelY;
        }
    }

    pub fn mapcolldet_handle_out_of_screen(&mut self) -> bool {
        if self.fPrecalculatedY + (PH as f32) < 0.0 {
            // on top outside of the screen
            self.set_yf(self.fPrecalculatedY);
            self.vely = cap_falling_velocity(GRAVITATION + self.vely);

            if self.platform.is_null() {
                self.inair = true;
                self.onice = false;
                self.superjumptimer = 0;
                self.fallthrough = false;
            }

            return true;
        } else if self.fPrecalculatedY + PH as f32 >= 480.0 {
            //on ground outside of the screen?
            self.set_yi(-PH as i16);
            self.fOldY = (-PH - 1) as f32;
            self.fallthrough = false;
            self.onice = false;

            //Clear the platform if the player wrapped to the top of the screen

            if !self.platform.is_null() {
                self.vely = self.platform.fVelY;
                self.platform = Ptr::null();
            }

            return true;
        }

        false
    }

    pub fn mapcolldet_move_horizontally(&mut self, direction: i16) {
        debug_assert!(direction == 1 || direction == 3);
        let counter_direction: i16 = if direction == 1 { 3 } else { 1 };
        let this = self.this();

        unsafe {
            //Could be optimized with bit shift >> 5
            let ty = (self.fy as i16 as i32 / TILESIZE) as i16;
            let ty2 = ((self.fy as i16 as i32 + PH) / TILESIZE) as i16;
            let mut tx: i16;

            let isMoveKeyDown = if direction == 1 { self.keys().game_left().fDown } else { self.keys().game_right().fDown };

            if direction == 1 {
                //moving left
                tx = (self.fx as i16 as i32 / TILESIZE) as i16;
            } else {
                //moving right
                if self.fx + PW as f32 >= App::screenWidth as f32 {
                    tx = ((self.fx + PW as f32 - App::screenWidth as f32) as i16 as i32 / TILESIZE) as i16;
                    self.fOldX -= App::screenWidth as f32;
                } else {
                    tx = ((self.fx as i16 as i32 + PW) / TILESIZE) as i16;
                }
            }

            //Just in case tx out of bounds and flipsidesifneeded wasn't called
            if tx < 0 {
                tx += 20;
            } else if tx > 19 {
                tx -= 20;
            }

            let topblock = g_map.block(tx, ty);
            let bottomblock = g_map.block(tx, ty2);

            let toptile = g_map.map(tx as i32, ty as i32);
            let bottomtile = g_map.map(tx as i32, ty2 as i32);

            let deathTileBehind: bool;
            let superDeathTileBehind: bool;
            if direction == 1 {
                deathTileBehind = (b(toptile & tile_flag_death_on_right) && b(bottomtile & tile_flag_death_on_right))
                    || (b(toptile & tile_flag_death_on_right) && !b(bottomtile & tile_flag_solid))
                    || (!b(toptile & tile_flag_solid) && b(bottomtile & tile_flag_death_on_right));

                superDeathTileBehind = (b(toptile & tile_flag_super_or_player_death_right) && b(bottomtile & tile_flag_super_or_player_death_right))
                    || (b(toptile & tile_flag_super_or_player_death_right) && !b(bottomtile & tile_flag_solid))
                    || (!b(toptile & tile_flag_solid) && b(bottomtile & tile_flag_super_or_player_death_right));
            } else {
                deathTileBehind = (b(toptile & tile_flag_death_on_left) && b(bottomtile & tile_flag_death_on_left))
                    || (b(toptile & tile_flag_death_on_left) && !b(bottomtile & tile_flag_solid))
                    || (!b(toptile & tile_flag_solid) && b(bottomtile & tile_flag_death_on_left));

                superDeathTileBehind = (b(toptile & tile_flag_super_or_player_death_left) && b(bottomtile & tile_flag_super_or_player_death_left))
                    || (b(toptile & tile_flag_super_or_player_death_left) && !b(bottomtile & tile_flag_solid))
                    || (!b(toptile & tile_flag_solid) && b(bottomtile & tile_flag_super_or_player_death_left));
            }

            let fTopBlockSolid = !topblock.is_null() && !topblock.get().is_transparent() && !topblock.get().is_hidden();
            let fBottomBlockSolid = !bottomblock.is_null() && !bottomblock.get().is_transparent() && !bottomblock.get().is_hidden();

            //first check to see if player hit a warp
            if isMoveKeyDown && !self.frozen && g_map.checkforwarp(tx, ty, ty2, direction) {
                if direction == 1 {
                    self.set_xf((((tx as i32) << 5) + TILESIZE) as f32 + 0.2f32); // move to the edge of the tile
                } else {
                    self.set_xf((((tx as i32) << 5) - PW) as f32 - 0.2f32); // move to the edge of the tile (tile on the right -> mind the player width)
                }

                let warp = Ptr::from_mut(g_map.warp(tx, ty2));
                self.warpstatus.enter_warp(this.get(), warp);

                if (self.iy as i32 - PHOFFSET) < ((ty as i32) << 5) {
                    self.set_yi((((ty as i32) << 5) + PHOFFSET) as i16);
                } else if self.iy as i32 + PH > ((ty2 as i32) << 5) + TILESIZE - 3 {
                    self.set_yi((((ty2 as i32) << 5) + TILESIZE - PH - 3) as i16);
                }
            } else if fTopBlockSolid || fBottomBlockSolid {
                if self.iHorizontalPlatformCollision == direction {
                    self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    return;
                }

                if netplay.active && netplay.theHostIsMe {
                    netplay.client.local_gamehost.prepare_map_collision_event(self);
                }

                let mut collisionresult = true;

                if fTopBlockSolid {
                    // collide with top block
                    collisionresult &= topblock.get().collide_player_dir(this, counter_direction, if netplay.active { netplay.theHostIsMe || netplay.allowMapCollisionEvent } else { true });
                    self.flipsidesifneeded();
                }

                if fBottomBlockSolid {
                    // then bottom
                    collisionresult &= bottomblock.get().collide_player_dir(this, counter_direction, if netplay.active { netplay.theHostIsMe || netplay.allowMapCollisionEvent } else { true });
                    self.flipsidesifneeded();
                }

                if netplay.active && netplay.theHostIsMe && collisionresult {
                    netplay.client.local_gamehost.send_map_collision_event();
                }
            } else if superDeathTileBehind || (deathTileBehind && !self.is_invincible() && !self.is_shielded() && !self.shyguy) {
                if PlayerKillType::NonKill != self.kill_player_map_hazard(superDeathTileBehind, KillStyle::Environment, false, -1) {
                    return;
                }
            }
            //collision on the side.
            else if b(toptile & tile_flag_solid) || b(bottomtile & tile_flag_solid) {
                //collide with solid, ice, and death and all sides death
                if self.iHorizontalPlatformCollision == direction {
                    self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    return;
                }

                if direction == 1 {
                    self.set_xf((((tx as i32) << 5) + TILESIZE) as f32 + 0.2f32); // move to the edge of the tile
                } else {
                    self.set_xf((((tx as i32) << 5) - PW) as f32 - 0.2f32); // move to the edge of the tile (tile on the right -> mind the player width)
                }

                self.fOldX = self.fx;

                if self.velx.abs() > 0.0 {
                    self.velx = 0.0;
                }

                if self.oldvelx.abs() > 0.0 {
                    self.oldvelx = 0.0;
                }

                self.flipsidesifneeded();
            }
        }
    }

    pub fn mapcolldet_move_upward(&mut self, txl: i16, txc: i16, txr: i16, alignedBlockX: i16, unAlignedBlockX: i16, unAlignedBlockFX: f32) {
        let this = self.this();

        unsafe {
            // moving up
            self.fallthrough = false;

            let ty = (self.fPrecalculatedY as i16 as i32 / TILESIZE) as i16;

            let leftblock = g_map.block(txl, ty);
            let centerblock = g_map.block(txc, ty);
            let rightblock = g_map.block(txr, ty);

            if self.keys().game_jump().fDown && !self.frozen && g_map.checkforwarp(alignedBlockX, unAlignedBlockX, ty, 2) {
                self.set_yf((((ty as i32) << 5) + TILESIZE) as f32 + 0.2f32);
                let warp = Ptr::from_mut(g_map.warp(unAlignedBlockX, ty));
                self.warpstatus.enter_warp(this.get(), warp);

                if (self.ix as i32 - PWOFFSET) < ((txl as i32) << 5) + 1 {
                    self.set_xi((((txl as i32) << 5) + PHOFFSET + 1) as i16);
                } else if self.ix as i32 + PW + PWOFFSET > ((txr as i32) << 5) + TILESIZE {
                    self.set_xi((((txr as i32) << 5) + TILESIZE - PW - PWOFFSET) as i16);
                }

                return;
            }

            if !centerblock.is_null() && !centerblock.get().is_transparent() {
                if netplay.active && netplay.theHostIsMe {
                    netplay.client.local_gamehost.prepare_map_collision_event(self);
                }

                if !centerblock.get().collide_player_dir(this, 0, if netplay.active { netplay.theHostIsMe || netplay.allowMapCollisionEvent } else { true }) {
                    if self.iVerticalPlatformCollision == 2 && !centerblock.get().is_hidden() {
                        self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    }

                    if netplay.active && netplay.theHostIsMe {
                        netplay.client.local_gamehost.send_map_collision_event();
                    }

                    return;
                }
            }

            //Player hit a solid, ice or death on top
            //or if the player is invincible and hits death or death on bottom

            //There is a known issue where where if a death on bottom tile and a super death on bottom tile
            //are next to each other and the player hits from below and aligns with the super death on bottom
            //the player will then shift over and fully hit the super death on bottom and die even if shielded
            //or invincible.

            let alignedTileType = g_map.map(alignedBlockX as i32, ty as i32);
            if b(alignedTileType & tile_flag_solid)
                && !b(alignedTileType & tile_flag_super_or_player_death_bottom)
                && (!b(alignedTileType & tile_flag_death_on_bottom) || self.is_invincible() || self.is_shielded() || self.shyguy)
            {
                self.set_yf((((ty as i32) << 5) + TILESIZE) as f32 + 0.2f32);
                self.fOldY = self.fy - 1.0;

                if self.vely < 0.0 {
                    self.vely = -self.vely * BOUNCESTRENGTH;
                }

                if self.iVerticalPlatformCollision == 2 {
                    self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                }

                return;
            }

            if !leftblock.is_null() && !leftblock.get().is_transparent() {
                //then left
                let useBehavior = alignedBlockX == txl || rightblock.is_null() || rightblock.get().is_transparent() || rightblock.get().is_hidden();

                if netplay.active && netplay.theHostIsMe {
                    netplay.client.local_gamehost.prepare_map_collision_event(self);
                }

                if !leftblock.get().collide_player_dir(this, 0, if netplay.active { (netplay.theHostIsMe || netplay.allowMapCollisionEvent) && useBehavior } else { useBehavior }) {
                    if self.iVerticalPlatformCollision == 2 && !leftblock.get().is_hidden() {
                        self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    }

                    if netplay.active && netplay.theHostIsMe {
                        netplay.client.local_gamehost.send_map_collision_event();
                    }

                    return;
                }
            }

            if !rightblock.is_null() && !rightblock.get().is_transparent() {
                //then right
                let useBehavior = alignedBlockX == txr || leftblock.is_null() || leftblock.get().is_transparent() || leftblock.get().is_hidden();

                if netplay.active && netplay.theHostIsMe {
                    netplay.client.local_gamehost.prepare_map_collision_event(self);
                }

                if !rightblock.get().collide_player_dir(this, 0, if netplay.active { (netplay.theHostIsMe || netplay.allowMapCollisionEvent) && useBehavior } else { useBehavior }) {
                    if self.iVerticalPlatformCollision == 2 && !rightblock.get().is_hidden() {
                        self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    }

                    if netplay.active && netplay.theHostIsMe {
                        netplay.client.local_gamehost.send_map_collision_event();
                    }

                    return;
                }
            }

            //Player squeezed around the block, ice or death on top
            //or if the player is invincible and hits death or death on bottom
            let unalignedTileType = g_map.map(unAlignedBlockX as i32, ty as i32);
            if b(unalignedTileType & tile_flag_solid)
                && !b(unalignedTileType & tile_flag_super_or_player_death_bottom)
                && (!b(unalignedTileType & tile_flag_death_on_bottom) || self.is_invincible() || self.is_shielded() || self.shyguy)
            {
                self.set_xf(unAlignedBlockFX);
                self.fOldX = self.fx;

                self.set_yf(self.fPrecalculatedY);
                self.vely += GRAVITATION;
            } else if b(alignedTileType & tile_flag_player_or_death_on_bottom) || b(unalignedTileType & tile_flag_player_or_death_on_bottom) {
                let fRespawnPlayer = (b(alignedTileType & tile_flag_super_or_player_death_bottom) && b(unalignedTileType & tile_flag_super_or_player_death_bottom))
                    || (b(alignedTileType & tile_flag_super_or_player_death_bottom) && !b(unalignedTileType & tile_flag_solid))
                    || (b(alignedTileType & tile_flag_solid) && !b(unalignedTileType & tile_flag_super_or_player_death_bottom));

                if PlayerKillType::NonKill != self.kill_player_map_hazard(fRespawnPlayer, KillStyle::Environment, false, -1) {
                    return;
                }
            } else {
                self.set_yf(self.fPrecalculatedY);
                self.vely += GRAVITATION;
            }

            if self.platform.is_null() {
                self.inair = true;
                self.onice = false;
            }
        }
    }

    pub fn mapcolldet_move_downward(&mut self, txl: i16, _txc: i16, txr: i16, alignedBlockX: i16, unAlignedBlockX: i16, _unAlignedBlockFX: f32) {
        let this = self.this();

        unsafe {
            // moving down / on ground
            let ty = ((self.fPrecalculatedY as i16 as i32 + PH) / TILESIZE) as i16;

            if self.keys().game_down().fDown && !self.frozen && g_map.checkforwarp(txl, txr, ty, 0) {
                self.set_yf((((ty as i32) << 5) - PH) as f32 - 0.2f32);
                let warp = Ptr::from_mut(g_map.warp(txr, ty));
                self.warpstatus.enter_warp(this.get(), warp);

                self.fallthrough = false;
                self.platform = Ptr::null();

                if (self.ix as i32 - PWOFFSET) < ((txl as i32) << 5) + 1 {
                    self.set_xi((((txl as i32) << 5) + PHOFFSET + 1) as i16);
                } else if self.ix as i32 + PW + PWOFFSET > ((txr as i32) << 5) + TILESIZE {
                    self.set_xi((((txr as i32) << 5) + TILESIZE - PW - PWOFFSET) as i16);
                }

                return;
            }

            let leftblock = g_map.block(txl, ty);
            let rightblock = g_map.block(txr, ty);

            let fLeftBlockSolid = !leftblock.is_null() && !leftblock.get().is_transparent() && !leftblock.get().is_hidden();
            let fRightBlockSolid = !rightblock.is_null() && !rightblock.get().is_transparent() && !rightblock.get().is_hidden();

            if fLeftBlockSolid || fRightBlockSolid {
                let mut collisionresult = true;
                if netplay.active && netplay.theHostIsMe {
                    netplay.client.local_gamehost.prepare_map_collision_event(self);
                }

                if fLeftBlockSolid {
                    //collide with left block
                    let useBehavior = alignedBlockX == txl || rightblock.is_null() || rightblock.get().is_transparent() || rightblock.get().is_hidden();
                    collisionresult &= leftblock.get().collide_player_dir(this, 2, if netplay.active { (netplay.theHostIsMe || netplay.allowMapCollisionEvent) && useBehavior } else { useBehavior });

                    if netplay.active && netplay.theHostIsMe && collisionresult {
                        netplay.client.local_gamehost.send_map_collision_event();
                    }

                    //If player was bumped and killed then return
                    if self.state != PlayerState::Ready {
                        return;
                    }
                }

                if fRightBlockSolid {
                    //then right
                    let useBehavior = alignedBlockX == txr || leftblock.is_null() || leftblock.get().is_transparent() || leftblock.get().is_hidden();
                    collisionresult &= rightblock.get().collide_player_dir(this, 2, if netplay.active { (netplay.theHostIsMe || netplay.allowMapCollisionEvent) && useBehavior } else { useBehavior });

                    if netplay.active && netplay.theHostIsMe && collisionresult {
                        netplay.client.local_gamehost.send_map_collision_event();
                    }

                    //If player was bumped and killed then return
                    if self.state != PlayerState::Ready {
                        return;
                    }
                }

                if !collisionresult {
                    self.platform = Ptr::null();
                    self.onice = false;

                    if self.iVerticalPlatformCollision == 0 {
                        self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    }

                    return;
                }
            }

            let lefttile = g_map.map(txl as i32, ty as i32);
            let righttile = g_map.map(txr as i32, ty as i32);

            let fGapSupport = (self.velx >= VELTURBOMOVING || self.velx <= -VELTURBOMOVING) && (lefttile == tile_flag_gap || righttile == tile_flag_gap);

            let fSolidTileUnderPlayer = b(lefttile & tile_flag_solid) || b(righttile & tile_flag_solid);

            if (b(lefttile & tile_flag_solid_on_top) || b(righttile & tile_flag_solid_on_top) || fGapSupport) && self.fOldY + PH as f32 <= ((ty as i32) << 5) as f32 {
                //on ground
                //Deal with player down jumping through solid on top tiles

                if self.platform.is_null() {
                    self.onice = false;
                }

                if self.fallthrough && !fSolidTileUnderPlayer {
                    self.set_yf((((ty as i32) << 5) - PH) as f32 + 0.2f32);

                    if self.platform.is_null() {
                        self.inair = true;
                    }
                } else {
                    //we were above the tile in the previous frame
                    self.set_yf((((ty as i32) << 5) - PH) as f32 - 0.2f32);
                    self.vely = GRAVITATION;

                    if self.platform.is_null() {
                        let alignedtile = g_map.map(alignedBlockX as i32, ty as i32);

                        if b(alignedtile & tile_flag_ice)
                            || ((alignedtile == tile_flag_nonsolid || alignedtile == tile_flag_gap) && b(g_map.map(unAlignedBlockX as i32, ty as i32) & tile_flag_ice))
                        {
                            self.onice = true;
                        } else {
                            self.onice = false;
                        }

                        self.inair = false;
                        self.extrajumps = 0;
                        self.killsinrowinair = 0;
                    }
                }

                self.fOldY = self.fy - GRAVITATION;

                if self.platform.is_null() {
                    self.fallthrough = false;
                }

                self.platform = Ptr::null();

                if self.iVerticalPlatformCollision == 0 {
                    self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                }

                return;
            }

            let fDeathTileUnderPlayer = (b(lefttile & tile_flag_death_on_top) && b(righttile & tile_flag_death_on_top))
                || (b(lefttile & tile_flag_death_on_top) && !b(righttile & tile_flag_solid))
                || (!b(lefttile & tile_flag_solid) && b(righttile & tile_flag_death_on_top));

            let fSuperDeathTileUnderPlayer = (b(lefttile & tile_flag_super_or_player_death_top) && b(righttile & tile_flag_super_or_player_death_top))
                || (b(lefttile & tile_flag_super_or_player_death_top) && !b(righttile & tile_flag_solid))
                || (!b(lefttile & tile_flag_solid) && b(righttile & tile_flag_super_or_player_death_top));

            if fSolidTileUnderPlayer && !fSuperDeathTileUnderPlayer && (!fDeathTileUnderPlayer || self.is_invincible_on_bottom() || self.shyguy) {
                //on ground

                self.set_yf((((ty as i32) << 5) - PH) as f32 - 0.2f32);
                self.vely = GRAVITATION; //1 so we test against the ground again int the next frame (0 would test against the ground in the next+1 frame)

                if self.platform.is_null() {
                    let alignedtile = g_map.map(alignedBlockX as i32, ty as i32);

                    if b(alignedtile & tile_flag_ice)
                        || ((alignedtile == tile_flag_nonsolid || alignedtile == tile_flag_gap) && b(g_map.map(unAlignedBlockX as i32, ty as i32) & tile_flag_ice))
                    {
                        self.onice = true;
                    } else {
                        self.onice = false;
                    }

                    self.inair = false;
                    self.extrajumps = 0;
                    self.killsinrowinair = 0;
                }

                self.platform = Ptr::null();

                if self.iVerticalPlatformCollision == 0 {
                    self.kill_player_map_hazard(true, KillStyle::Environment, true, self.iPlatformCollisionPlayerId);
                    return;
                }
            } else if fDeathTileUnderPlayer || fSuperDeathTileUnderPlayer {
                if PlayerKillType::NonKill != self.kill_player_map_hazard(fSuperDeathTileUnderPlayer, KillStyle::Environment, false, -1) {
                    return;
                }
            } else {
                //falling (in air)
                self.set_yf(self.fPrecalculatedY);
                self.vely = cap_falling_velocity(GRAVITATION + self.vely);

                if self.platform.is_null() {
                    //If we're not hopping with kuribo's shoe, then zero out our super jump timer
                    if self.superjumptype != 3 {
                        self.superjumptimer = 0;
                    }

                    self.inair = true;
                }
            }
        }
    }

    pub fn collision_detection_map(&mut self) {
        let this = self.this();

        self.set_xf(self.fx + self.velx);
        self.flipsidesifneeded();

        self.fPrecalculatedY = self.fy + self.vely; //Fixes weird float rounding error.  Must be computed here before casting to int.  Otherwise, this will miss the bottom collision, but then hit the side collision and the player can slide out of 1x1 spaces.

        let mut fPlatformVelX: f32 = 0.0;
        let mut fPlatformVelY: f32 = 0.0;

        let fTempY = self.fy;

        self.mapcolldet_handle_platform_velocity(&mut fPlatformVelX, &mut fPlatformVelY);

        self.iHorizontalPlatformCollision = -1;
        self.iVerticalPlatformCollision = -1;
        self.iPlatformCollisionPlayerId = -1;

        unsafe {
            g_map.moving_platform_collision_player(this);
        }

        if self.state != PlayerState::Ready {
            return;
        }

        self.fy = fTempY;

        if self.mapcolldet_handle_out_of_screen() {
            return;
        }

        //-----------------------------------------------------------------
        //  x axis (--)
        //-----------------------------------------------------------------
        if self.fy + PH as f32 >= 0.0 {
            if self.velx + fPlatformVelX > 0.01 || self.iHorizontalPlatformCollision == 3 {
                self.mapcolldet_move_horizontally(3);
            } else if self.velx + fPlatformVelX < -0.01 || self.iHorizontalPlatformCollision == 1 {
                self.mapcolldet_move_horizontally(1);
            }

            if self.isdead() {
                return;
            }
        }

        //-----------------------------------------------------------------
        //  then y axis (|)
        //-----------------------------------------------------------------

        let mut iPlayerL: i16 = self.ix;
        let mut iPlayerC: i16 = (self.ix as i32 + HALFPW) as i16;
        let mut iPlayerR: i16 = (self.ix as i32 + PW) as i16;

        if (iPlayerL as i32) < 0 {
            iPlayerL = (iPlayerL as i32 + App::screenWidth) as i16;
        } else if iPlayerL as i32 >= App::screenWidth {
            iPlayerL = (iPlayerL as i32 - App::screenWidth) as i16;
        }

        if iPlayerC as i32 >= App::screenWidth {
            iPlayerC = (iPlayerC as i32 - App::screenWidth) as i16;
        }

        if iPlayerR as i32 >= App::screenWidth {
            iPlayerR = (iPlayerR as i32 - App::screenWidth) as i16;
        }

        let txl = (iPlayerL as i32 / TILESIZE) as i16;
        let txc = (iPlayerC as i32 / TILESIZE) as i16;
        let txr = (iPlayerR as i32 / TILESIZE) as i16;

        //What block is the player aligned to (this will be the block that has the action on it)
        let alignedBlockX: i16;
        let unAlignedBlockX: i16;
        let unAlignedBlockFX: f32;

        let overlaptxl = (((txl as i32) << 5) + TILESIZE + 1) as i16;

        if (self.ix as i32 + HALFPW) < overlaptxl as i32 {
            alignedBlockX = txl;
            unAlignedBlockX = txr;
            unAlignedBlockFX = (((txr as i32) << 5) - PW) as f32 - 0.2f32;
        } else {
            alignedBlockX = txr;
            unAlignedBlockX = txl;
            unAlignedBlockFX = (((txl as i32) << 5) + TILESIZE) as f32 + 0.2f32;
        }

        let mut fMovingUp = self.vely;
        if !self.platform.is_null() {
            fMovingUp = self.vely + fPlatformVelY - GRAVITATION;
        }

        if fMovingUp < -0.01 {
            //moving up
            self.mapcolldet_move_upward(txl, txc, txr, alignedBlockX, unAlignedBlockX, unAlignedBlockFX);
        } else {
            //moving down / on ground
            self.mapcolldet_move_downward(txl, txc, txr, alignedBlockX, unAlignedBlockX, unAlignedBlockFX);
        }

        if self.platform.is_null() {
            self.fallthrough = false;

            if self.inair {
                self.onice = false;
            }
        }
    }

    //iPlayerIdCredit is passed in if this platform was triggered by another player and crushed this player (e.g. donut block)
    pub fn kill_player_map_hazard(&mut self, fForce: bool, style: KillStyle, fKillCarriedItem: bool, iPlayerIdCredit: i16) -> PlayerKillType {
        let args = [2, self.globalID as i32, -1, 0, style as i32, fForce as i32, fKillCarriedItem as i32, iPlayerIdCredit as i32];
        if let Some(result) = crate::smw::net_outcomes::kill(&args) {
            return result;
        }
        self.kill_player_map_hazard_now(fForce, style, fKillCarriedItem, iPlayerIdCredit)
    }

    pub fn kill_player_map_hazard_now(&mut self, fForce: bool, style: KillStyle, fKillCarriedItem: bool, iPlayerIdCredit: i16) -> PlayerKillType {
        let this = self.this();
        if iPlayerIdCredit >= 0 || self.iSuicideCreditPlayerID >= 0 {
            player_killed_player(
                if iPlayerIdCredit >= 0 { iPlayerIdCredit } else { self.iSuicideCreditPlayerID },
                this,
                PlayerDeathStyle::Jump,
                KillStyle::Push,
                fForce,
                fKillCarriedItem,
            )
        } else {
            self.death_awards();

            unsafe {
                let iKillType = game_values.gamemode.get().playerkilledself(this, style);
                if PlayerKillType::Normal == iKillType || (PlayerKillType::NonKill == iKillType && fForce) {
                    self.die(PlayerDeathStyle::Jump, false, fKillCarriedItem);
                }

                if_sound_on_play(&mut rm.sfx_deathsound);

                iKillType
            }
        }
    }

    /// `collidesWith(CPlayer*)`
    pub fn collides_with_player(&mut self, other: Ptr<CPlayer>) {
        let this = self.this();
        self.collisions.handle_p2p(this, other);
    }

    /// `collidesWith(CObject*)`
    pub fn collides_with_object(&mut self, object: Ptr<dyn CObjectTrait>) -> bool {
        let this = self.this();
        self.collisions.handle_p2o(this, object)
    }

    pub fn flipsidesifneeded(&mut self) {
        //Use ix here to avoid rounding issues (can crash if tile_x_right evals to over the right side of screen)
        if self.ix < 0 || self.fx < 0.0 {
            //This avoids rounding errors
            self.set_xf(self.fx + 640.0);
            self.fOldX += 640.0;
        } else if self.ix >= 640 || self.fx >= 640.0 {
            self.set_xf(self.fx - 640.0);
            self.fOldX -= 640.0;
        }
    }

    pub fn makeinvincible(&mut self) {
        let this = self.this();
        self.invincibility.turn_on(this.get());
    }

    pub fn makefrozen(&mut self, iTime: i16) {
        if !self.frozen {
            self.frozen = true;
            self.frozentimer = iTime;

            unsafe {
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (self.ix as i32 + HALFPW - 16) as i16,
                    (self.iy as i32 + HALFPH - 16) as i16,
                    3,
                    8,
                ));
            }
        }
    }

    pub fn turnslowdownon(&mut self) {
        unsafe {
            game_values.flags.slowdownon = self.teamID;
            game_values.flags.slowdowncounter = 0;
        }
    }

    //Returns true if player facing right, false if left
    pub fn is_facing_right(&self) -> bool {
        unsafe {
            let fLeft = game_values.reversewalk;
            let fRight = !fLeft;

            if game_values.flags.swapplayers && game_values.swapstyle == 0 {
                if self.fNewSwapX < self.fOldSwapX {
                    return fLeft;
                } else {
                    return fRight;
                }
            }

            if self.state == PlayerState::Ready {
                if self.keys().game_left().fDown && self.keys().game_right().fDown && self.velx != 0.0 {
                    if self.velx > 0.0 {
                        return fRight;
                    } else {
                        return fLeft;
                    }
                } else if self.keys().game_left().fDown {
                    return fLeft;
                } else if self.keys().game_right().fDown {
                    return fRight;
                }
            }

            if self.sprite_state as i32 == PGFX_STOPPING_R {
                false
            } else if self.sprite_state as i32 == PGFX_STOPPING_L {
                true
            } else {
                self.sprite_state & 0x1 == 0
            }
        }
    }

    pub fn accept_item(&mut self, mut item: Ptr<dyn MO_CarriedObjectTrait>) -> bool {
        if self.fAcceptingItem && self.tanookisuit.not_statue() && (!self.kuriboshoe.is_on() || item.is_carried_by_kuribo_shoe()) {
            self.action = PlayerAction::None;

            self.carriedItem = item;
            item.owner = Ptr::from_mut(self);
            self.carriedItem.velx = 0.0;
            self.carriedItem.vely = 0.0;

            self.fAcceptingItem = false;
            return true;
        }

        false
    }

    pub fn set_powerup(&mut self, iPowerup: i16) {
        unsafe {
            if self.shyguy {
                if_sound_on_play(&mut rm.sfx_stun);
                return;
            }

            if iPowerup == 0 {
                if self.bobomb {
                    self.set_stored_powerup_type(PowerupType::Bobomb);
                } else {
                    if_sound_on_play(&mut rm.sfx_transform);
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_poof),
                        (self.ix as i32 + HALFPW - 24) as i16,
                        (self.iy as i32 + HALFPH - 24) as i16,
                        4,
                        5,
                    ));
                    self.bobomb = true;
                }
            } else if iPowerup == 9 {
                if self.tanookisuit.is_on() {
                    self.set_stored_powerup_type(PowerupType::Tanooki);
                } else {
                    self.tanookisuit.on_pickup();
                }
            } else if iPowerup >= 10 {
                if iPowerup == 10 {
                    self.set_stored_powerup_type(PowerupType::Pow);
                } else if iPowerup == 11 {
                    self.set_stored_powerup_type(PowerupType::Mod);
                } else if iPowerup == 12 {
                    self.set_stored_powerup_type(PowerupType::BulletBill);
                } else if iPowerup == 13 {
                    self.set_stored_powerup_type(PowerupType::Podobo);
                } else if iPowerup >= 14 {
                    self.set_stored_powerup(iPowerup - 2); //Storing shells
                }
            } else {
                if iPowerup == 3 || iPowerup == 7 || iPowerup == 8 {
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 + HALFPW - 16) as i16,
                        (self.iy as i32 + HALFPH - 16) as i16,
                        3,
                        8,
                    ));
                }

                if self.powerup != iPowerup {
                    if iPowerup == 3 {
                        if_sound_on_play(&mut rm.sfx_collectfeather);
                    } else {
                        if_sound_on_play(&mut rm.sfx_collectpowerup);
                    }
                }

                self.clear_powerup_states();

                if self.powerup == 1 {
                    self.set_stored_powerup_type(PowerupType::Fire);
                } else if self.powerup == 2 {
                    self.set_stored_powerup_type(PowerupType::Hammer);
                } else if self.powerup == 3 {
                    self.set_stored_powerup_type(PowerupType::Feather);
                } else if self.powerup == 4 {
                    self.set_stored_powerup_type(PowerupType::Boomerang);
                } else if self.powerup == 5 {
                    self.set_stored_powerup_type(PowerupType::IceWand);
                } else if self.powerup == 6 {
                    self.set_stored_powerup_type(PowerupType::Bomb);
                } else if self.powerup == 7 {
                    self.set_stored_powerup_type(PowerupType::Leaf);
                } else if self.powerup == 8 {
                    self.set_stored_powerup_type(PowerupType::PWings);
                }

                self.powerup = iPowerup;
                self.projectilelimit = 0;

                self.flying = false;

                if self.powerup == 1 {
                    if game_values.fireballlimit > 0 {
                        self.projectilelimit = game_values.fireballlimit;
                    }
                } else if self.powerup == 2 {
                    if game_values.hammerlimit > 0 {
                        self.projectilelimit = game_values.hammerlimit;
                    }
                } else if self.powerup == 3 {
                    if game_values.featherlimit > 0 {
                        self.projectilelimit = game_values.featherlimit;
                    }
                } else if self.powerup == 4 {
                    if game_values.boomeranglimit > 0 {
                        self.projectilelimit = game_values.boomeranglimit;
                    }
                } else if self.powerup == 5 {
                    if game_values.wandlimit > 0 {
                        self.projectilelimit = game_values.wandlimit;
                    }
                } else if self.powerup == 6 {
                    if game_values.bombslimit > 0 {
                        self.projectilelimit = game_values.bombslimit;
                    }
                } else if self.powerup == 7 {
                    if game_values.leaflimit > 0 {
                        self.projectilelimit = game_values.leaflimit;
                    }
                } else if self.powerup == 8 {
                    if game_values.pwingslimit > 0 {
                        self.projectilelimit = game_values.pwingslimit;
                    }
                }
            }

            //Minor fix for becoming caped to draw animation correctly
            if iPowerup == 3 {
                self.cape.restart_animation();
            }
        }
    }

    pub fn set_stored_powerup(&mut self, iPowerup: i16) {
        unsafe {
            //If the player as the poison mushroom or the game is over, don't store the powerup
            if game_values.gamepowerups[self.globalID as usize] == PowerupType::PoisonMushroom as i16 || game_values.gamemode.gameover {
                if_sound_on_play(&mut rm.sfx_stun);
                return;
            }

            game_values.gamepowerups[self.globalID as usize] = iPowerup;
            if_sound_on_play(&mut rm.sfx_storepowerup);
        }
    }

    pub fn clear_powerup_states(&mut self) {
        self.tail.reset();
        self.spin.reset();
        self.wings.reset();

        self.flying = false;
        self.flyingtimer = 0;
    }

    pub fn decrease_projectile_limit(&mut self) {
        self.projectilelimit -= 1;
        if self.projectilelimit <= 0 {
            self.projectilelimit = 0;
            self.powerup = -1;
            unsafe {
                if_sound_on_play(&mut rm.sfx_powerdown);
            }
        }
    }
}
