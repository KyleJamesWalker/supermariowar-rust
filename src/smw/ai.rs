//! Port of src/smw/ai.cpp

use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::global_constants::*;
use crate::common::input::COutputControl;
use crate::common::moving_object_types::*;
use crate::common::object_base::*;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType, RANDOM_BOOL};
use crate::common::tile_types::*;
use crate::globals::*;
use crate::smw::gamemodes::chicken::CGM_Chicken;
use crate::smw::gamemodes::race::CGM_Race;
use crate::smw::gamemodes::star::CGM_Star;
use crate::smw::gamemodes::tag::CGM_Tag;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::objectgame::PowerupType;
use crate::smw::objects::carriable::co_bomb::CO_Bomb;
use crate::smw::objects::carriable::co_egg::CO_Egg;
use crate::smw::objects::carriable::co_flag::CO_Flag;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_star::CO_Star;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_bonus_house_chest::MO_BonusHouseChest;
use crate::smw::objects::moving::mo_boomerang::MO_Boomerang;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::moving::mo_collection_card::MO_CollectionCard;
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::mo_fireball::MO_Fireball;
use crate::smw::objects::moving::mo_flag_base::MO_FlagBase;
use crate::smw::objects::moving::mo_hammer::MO_Hammer;
use crate::smw::objects::moving::mo_ice_blast::MO_IceBlast;
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::moving::mo_yoshi::MO_Yoshi;
use crate::smw::objects::overmap::wo_area::OMO_Area;
use crate::smw::objects::overmap::wo_phanto::OMO_Phanto;
use crate::smw::objects::overmap::wo_pipe_bonus::OMO_PipeBonus;
use crate::smw::objects::overmap::wo_pipe_coin::OMO_PipeCoin;
use crate::smw::objects::overmap::wo_race_goal::OMO_RaceGoal;
use crate::smw::player::{CPlayer, PlayerState};
use std::any::Any;
use std::collections::BTreeMap;

pub struct NearestObjects {
    pub player: Ptr<CPlayer>,
    pub teammate: Ptr<CPlayer>,

    pub goal: Ptr<dyn CObjectTrait>,
    pub stomp: Ptr<dyn CObjectTrait>,
    pub threat: Ptr<dyn CObjectTrait>,

    pub playerdistance: i32,
    pub teammatedistance: i32,
    pub goaldistance: i32,
    pub stompdistance: i32,
    pub threatdistance: i32,

    pub playerwrap: bool,
    pub teammatewrap: bool,
    pub goalwrap: bool,
    pub stompwrap: bool,
    pub threatwrap: bool,
}

impl NearestObjects {
    pub fn new() -> Self {
        let mut n = NearestObjects {
            player: Ptr::null(),
            teammate: Ptr::null(),
            goal: Ptr::null(),
            stomp: Ptr::null(),
            threat: Ptr::null(),
            playerdistance: 0,
            teammatedistance: 0,
            goaldistance: 0,
            stompdistance: 0,
            threatdistance: 0,
            playerwrap: false,
            teammatewrap: false,
            goalwrap: false,
            stompwrap: false,
            threatwrap: false,
        };
        n.reset();
        n
    }

    pub fn reset(&mut self) {
        self.player = Ptr::null();
        self.goal = Ptr::null();
        self.stomp = Ptr::null();
        self.teammate = Ptr::null();
        self.threat = Ptr::null();

        self.playerdistance = App::screenWidth * 1000;
        self.goaldistance = App::screenWidth * 1000;
        self.stompdistance = App::screenWidth * 1000;
        self.teammatedistance = App::screenWidth * 1000;
        self.threatdistance = App::screenWidth * 1000;

        self.playerwrap = false;
        self.goalwrap = false;
        self.stompwrap = false;
        self.teammatewrap = false;
        self.threatwrap = false;
    }
}

impl Default for NearestObjects {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct AttentionObject {
    pub iID: i32,    //Global ID of this object
    pub iType: i16,  //Ignore it, high priority, etc.
    pub iTimer: i16, //When it the attention expires, 0 for never
}

/// Virtual interface of `CPlayerAI` (`Init`, `Think`).
pub trait CPlayerAITrait {
    fn ai(&mut self) -> &mut CPlayerAI;

    fn set_player(&mut self, player: Ptr<CPlayer>) {
        self.ai().pPlayer = player;
    }
    fn init(&mut self) {
        cplayer_ai_init(self.ai())
    }
    fn think(&mut self, playerKeys: Ptr<COutputControl>) {
        cplayer_ai_think(self.ai(), playerKeys)
    }
}

pub struct CPlayerAI {
    pub pPlayer: Ptr<CPlayer>,

    pub iFallDanger: i16,
    pub nearestObjects: NearestObjects,

    pub attentionObjects: BTreeMap<i32, Box<AttentionObject>>,
    pub currentAttentionObject: AttentionObject,

    pub _alias: Aliased,
}

impl Default for CPlayerAI {
    fn default() -> Self {
        Self::new()
    }
}

impl CPlayerAITrait for CPlayerAI {
    fn ai(&mut self) -> &mut CPlayerAI {
        self
    }
}

// --- cast helpers -------------------------------------------------------------

/// `dynamic_cast<T*>(obj)`
fn dynamic_cast<T: Any>(obj: Ptr<dyn CObjectTrait>) -> Ptr<T> {
    if obj.is_null() {
        return Ptr::null();
    }
    match obj.get().as_any().downcast_mut::<T>() {
        Some(t) => Ptr::from_mut(t),
        None => Ptr::null(),
    }
}

/// `(T*)obj` where C++ already knows the type from `getObjectType()` / `getMovingObjectType()`.
fn static_cast<T: Any>(obj: Ptr<dyn CObjectTrait>) -> Ptr<T> {
    let p = dynamic_cast::<T>(obj);
    if p.is_null() {
        panic!("ai: bad static_cast to {}", std::any::type_name::<T>());
    }
    p
}

/// `dynamic_cast<T*>(game_values.gamemode)`
fn dynamic_cast_gamemode<T: Any>() -> Ptr<T> {
    unsafe {
        let gm = game_values.gamemode;
        if gm.is_null() {
            return Ptr::null();
        }
        match gm.get().as_any().downcast_mut::<T>() {
            Some(t) => Ptr::from_mut(t),
            None => Ptr::null(),
        }
    }
}

/// `(T*)game_values.gamemode`
fn static_cast_gamemode<T: Any>() -> Ptr<T> {
    let p = dynamic_cast_gamemode::<T>();
    if p.is_null() {
        panic!("ai: bad static_cast of gamemode to {}", std::any::type_name::<T>());
    }
    p
}

// After a wrap pushback ix can be ~-632 or ~650, so the C++ indexes g_map with x ~-19 or 20 and reads
// neighbouring CMap members. These offsets are the clang arm64 CMap layout (offsetof); padding reads as 0.
const CMAP_OFF_MAPDATA: i32 = 38;
const CMAP_OFF_MAPDATATOP: i32 = 7238;
const CMAP_OFF_OBJECTDATA: i32 = 7538;
const CMAP_OFF_BLOCKDATA: i32 = 24344;
const CMAP_OFF_NOSPAWN: i32 = 26744;
const CMAP_OFF_WARPDATA: i32 = 36368;
const CMAP_OFF_NUMWARPEXITS: i32 = 38768;
const CMAP_OFF_WARPEXITS: i32 = 38772;
const CMAP_OFF_WARPLOCKTIMER: i32 = 39540;
const CMAP_OFF_WARPLOCKED: i32 = 39560;
const CMAP_OFF_MAXCONNECTION: i32 = 39570;
const CMAP_SIZEOF_MAPBLOCK: i32 = 56;
const CMAP_SIZEOF_WARPEXIT: i32 = 24;

fn in_map(x: i32, y: i32) -> bool {
    (0..MAPWIDTH).contains(&x) && (0..MAPHEIGHT).contains(&y)
}

/// One byte of the C++ CMap object at `off`, for the members between mapdata and blockdata.
unsafe fn cmap_byte(off: i32) -> u8 {
    let cells = MAPWIDTH * MAPHEIGHT;
    if (CMAP_OFF_MAPDATA..CMAP_OFF_MAPDATA + cells * MAPLAYERS * 6).contains(&off) {
        let rel = off - CMAP_OFF_MAPDATA;
        let e = rel / 6;
        let t = &g_map.mapdata[(e / (MAPHEIGHT * MAPLAYERS)) as usize][((e / MAPLAYERS) % MAPHEIGHT) as usize][(e % MAPLAYERS) as usize];
        let v = [t.iID, t.iCol, t.iRow][((rel % 6) / 2) as usize];
        return v.to_le_bytes()[(rel % 2) as usize];
    }
    if (CMAP_OFF_MAPDATATOP..CMAP_OFF_MAPDATATOP + cells).contains(&off) {
        let e = off - CMAP_OFF_MAPDATATOP;
        return g_map.mapdatatop[(e / MAPHEIGHT) as usize][(e % MAPHEIGHT) as usize].0;
    }
    if (CMAP_OFF_OBJECTDATA..CMAP_OFF_OBJECTDATA + cells * CMAP_SIZEOF_MAPBLOCK).contains(&off) {
        let rel = off - CMAP_OFF_OBJECTDATA;
        let e = rel / CMAP_SIZEOF_MAPBLOCK;
        let b = &g_map.objectdata[(e / MAPHEIGHT) as usize][(e % MAPHEIGHT) as usize];
        let r = rel % CMAP_SIZEOF_MAPBLOCK;
        return match r {
            0..=1 => b.iType.to_le_bytes()[r as usize],
            2..=53 => b.iSettings[((r - 2) / 2) as usize].to_le_bytes()[(r % 2) as usize],
            54 => b.fHidden as u8,
            _ => 0,
        };
    }
    if (CMAP_OFF_NOSPAWN..CMAP_OFF_NOSPAWN + NUMSPAWNAREATYPES * cells).contains(&off) {
        let rel = off - CMAP_OFF_NOSPAWN;
        return g_map.nospawn[(rel / cells) as usize][((rel % cells) / MAPHEIGHT) as usize][(rel % MAPHEIGHT) as usize] as u8;
    }
    if (CMAP_OFF_WARPDATA..CMAP_OFF_NUMWARPEXITS).contains(&off) {
        let rel = off - CMAP_OFF_WARPDATA;
        let e = rel / 8;
        let w = &g_map.warpdata[(e / MAPHEIGHT) as usize][(e % MAPHEIGHT) as usize];
        let r = (rel % 8) as usize;
        return match r {
            0..=3 => (w.direction as i32).to_le_bytes()[r],
            4..=5 => w.connection.to_le_bytes()[r - 4],
            _ => w.id.to_le_bytes()[r - 6],
        };
    }
    if (CMAP_OFF_NUMWARPEXITS..CMAP_OFF_WARPEXITS).contains(&off) {
        let r = (off - CMAP_OFF_NUMWARPEXITS) as usize;
        return if r < 2 { g_map.numwarpexits.to_le_bytes()[r] } else { 0 };
    }
    if (CMAP_OFF_WARPEXITS..CMAP_OFF_WARPLOCKTIMER).contains(&off) {
        let rel = off - CMAP_OFF_WARPEXITS;
        let x = &g_map.warpexits[(rel / CMAP_SIZEOF_WARPEXIT) as usize];
        let r = (rel % CMAP_SIZEOF_WARPEXIT) as usize;
        if r < 4 {
            return (x.direction as i32).to_le_bytes()[r];
        }
        let v = [x.connection, x.id, x.x, x.y, x.lockx, x.locky, x.warpx, x.warpy, x.numblocks, x.locktimer][(r - 4) / 2];
        return v.to_le_bytes()[r % 2];
    }
    if (CMAP_OFF_WARPLOCKTIMER..CMAP_OFF_WARPLOCKED).contains(&off) {
        let rel = (off - CMAP_OFF_WARPLOCKTIMER) as usize;
        return g_map.warplocktimer[rel / 2].to_le_bytes()[rel % 2];
    }
    if (CMAP_OFF_WARPLOCKED..CMAP_OFF_MAXCONNECTION).contains(&off) {
        return g_map.warplocked[(off - CMAP_OFF_WARPLOCKED) as usize] as u8;
    }
    panic!("ai: C++ out-of-bounds CMap read at offset {} is not modelled", off);
}

unsafe fn cmap_i16(off: i32) -> i16 {
    i16::from_le_bytes([cmap_byte(off), cmap_byte(off + 1)])
}

unsafe fn cmap_i32(off: i32) -> i32 {
    i32::from_le_bytes([cmap_byte(off), cmap_byte(off + 1), cmap_byte(off + 2), cmap_byte(off + 3)])
}

/// `g_map->map(x, y)`, including the C++ out-of-bounds reads.
unsafe fn ai_map(x: i32, y: i32) -> i32 {
    if in_map(x, y) {
        return g_map.map(x, y);
    }
    tile_to_flags(TileType(cmap_byte(CMAP_OFF_MAPDATATOP + x * MAPHEIGHT + y))) as i32
}

/// `g_map->block(x, y) != NULL`, including the C++ out-of-bounds reads.
unsafe fn ai_block_nonnull(x: i16, y: i16) -> bool {
    if in_map(x as i32, y as i32) {
        return !g_map.block(x, y).is_null();
    }
    let off = CMAP_OFF_BLOCKDATA + (x as i32 * MAPHEIGHT + y as i32) * 8;
    (0..8).any(|i| cmap_byte(off + i) != 0)
}

/// `&warpdata[x][y]` as (direction, connection, id), including the C++ out-of-bounds reads.
unsafe fn ai_warp(x: i16, y: i16) -> (i32, i16, i16) {
    if in_map(x as i32, y as i32) {
        let w = &g_map.warpdata[x as usize][y as usize];
        return (w.direction as i32, w.connection, w.id);
    }
    let off = CMAP_OFF_WARPDATA + (x as i32 * MAPHEIGHT + y as i32) * 8;
    (cmap_i32(off), cmap_i16(off + 4), cmap_i16(off + 6))
}

/// `g_map->checkforwarp(x1, x2, y, 2)`, including the C++ out-of-bounds reads.
unsafe fn ai_checkforwarp_up(x1: i16, x2: i16, y: i16) -> bool {
    if in_map(x1 as i32, y as i32) && in_map(x2 as i32, y as i32) {
        return g_map.checkforwarp(x1, x2, y, 2);
    }
    let w1 = ai_warp(x1, y);
    let w2 = ai_warp(x2, y);
    if !(w1.0 == w2.0 && w1.2 == w2.2 && w1.0 == 2) {
        return false;
    }
    let locked = if (0..10).contains(&w1.1) { g_map.warplocked[w1.1 as usize] as u8 } else { cmap_byte(CMAP_OFF_WARPLOCKED + w1.1 as i32) };
    if locked != 0 {
        return false;
    }
    let locktimer = if (0..32).contains(&w1.2) {
        g_map.warpexits[w1.2 as usize].locktimer
    } else {
        cmap_i16(CMAP_OFF_WARPEXITS + w1.2 as i32 * CMAP_SIZEOF_WARPEXIT + 22)
    };
    locktimer <= 0
}

/// `carriedItem` seen as a `CObject*`.
fn carried_as_object(carriedItem: Ptr<dyn MO_CarriedObjectTrait>) -> Ptr<dyn CObjectTrait> {
    if carriedItem.is_null() {
        return Ptr::null();
    }
    carriedItem.get().as_object_ptr()
}

/// `movingobject->getMovingObjectType()` for an object whose `getObjectType()` is `object_moving`.
fn moving_object_type(mut obj: Ptr<dyn CObjectTrait>) -> MovingObjectType {
    obj.as_io_moving_object().expect("ai: object_moving is not an IO_MovingObject").get_moving_object_type()
}

const POWERUP_TYPES: [PowerupType; 27] = [
    PowerupType::PoisonMushroom,
    PowerupType::ExtraLife1,
    PowerupType::ExtraLife2,
    PowerupType::ExtraLife3,
    PowerupType::ExtraLife5,
    PowerupType::Fire,
    PowerupType::Star,
    PowerupType::Clock,
    PowerupType::Bobomb,
    PowerupType::Pow,
    PowerupType::BulletBill,
    PowerupType::Hammer,
    PowerupType::ShellGreen,
    PowerupType::ShellRed,
    PowerupType::ShellSpiny,
    PowerupType::ShellBuzzy,
    PowerupType::Mod,
    PowerupType::Feather,
    PowerupType::MysteryMushroom,
    PowerupType::Boomerang,
    PowerupType::Tanooki,
    PowerupType::IceWand,
    PowerupType::Podobo,
    PowerupType::Bomb,
    PowerupType::Leaf,
    PowerupType::PWings,
    PowerupType::JailKey,
];

/// `DistanceToObject(obj, &nearestObjects.<kind>, &nearestObjects.<kind>distance, &nearestObjects.<kind>wrap)`
macro_rules! nearest {
    ($self:ident . $method:ident ($obj:expr, $target:ident, $dist:ident, $wrap:ident)) => {{
        let no: *mut NearestObjects = &mut $self.nearestObjects;
        unsafe { $self.$method($obj, &mut (*no).$target, &mut (*no).$dist, &mut (*no).$wrap) }
    }};
}

impl CPlayerAI {
    pub fn new() -> Self {
        CPlayerAI {
            pPlayer: Ptr::null(),
            // Uninitialized in the C++ constructor; heap memory from `new` reads as 0 in practice.
            iFallDanger: 0,
            nearestObjects: NearestObjects::new(),
            attentionObjects: BTreeMap::new(),
            currentAttentionObject: AttentionObject { iID: -1, iType: 0, iTimer: 0 },
            _alias: Aliased::new(),
        }
    }

    pub fn set_player(&mut self, player: Ptr<CPlayer>) {
        self.pPlayer = player;
    }

    pub fn get_nearest_objects(&mut self) {
        unsafe {
            self.nearestObjects.reset();

            let pPlayer = self.pPlayer;
            let carriedItem = pPlayer.carriedItem;
            let carriedObj = carried_as_object(carriedItem);
            let fInvincible = pPlayer.is_invincible() || pPlayer.is_shielded() || pPlayer.shyguy;
            let iTeamID: i16 = pPlayer.teamID;

            let n = objectcontainer[1].list().len();
            for i in 0..n {
                let mut obj = objectcontainer[1].list()[i];
                if self.attentionObjects.contains_key(&obj.network_id()) {
                    //DistanceToObject(object, &nearestObjects.threat, &nearestObjects.threatdistance, &nearestObjects.threatwrap);
                    continue;
                }

                let r#type: ObjectType = obj.get_object_type();

                match r#type {
                    object_moving => {
                        let movingtype = moving_object_type(obj);

                        if carriedObj == obj {
                            continue;
                        }

                        if movingobject_shell == movingtype {
                            let shell = static_cast::<CO_Shell>(obj);

                            if shell.is_threat() {
                                if fInvincible {
                                    continue;
                                }

                                nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                            } else if !carriedItem.is_null() {
                                continue;
                            } else {
                                nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                            }
                        } else if movingobject_throwblock == movingtype
                            || (movingobject_throwbox == movingtype && static_cast::<CO_ThrowBox>(obj).has_kill_velocity())
                        {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_pirhanaplant == movingtype {
                            // MO_PirhanaPlant has no own `state`; this is CObject::state.
                            if obj.state > 0 {
                                nearest!(self.distance_to_object_center(obj, threat, threatdistance, threatwrap));
                            }
                        } else if movingobject_flag == movingtype {
                            let mut flag = static_cast::<CO_Flag>(obj);

                            if flag.get_in_base() && flag.get_team_id() == iTeamID {
                                continue;
                            }

                            if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_flag {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if movingobject_yoshi == movingtype {
                            let mut yoshi = static_cast::<MO_Yoshi>(obj);

                            if carriedItem.is_null() || carriedItem.get_moving_object_type() != movingobject_egg {
                                continue;
                            }

                            let mut egg = static_cast::<CO_Egg>(carriedObj);

                            if yoshi.get_color() != egg.get_color() {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if movingobject_egg == movingtype {
                            if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_egg {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if movingobject_throwbox == movingtype {
                            if !carriedItem.is_null() {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if movingobject_star == movingtype {
                            if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_star {
                                continue;
                            }

                            let mut starmode = static_cast_gamemode::<CGM_Star>();
                            let mut star = static_cast::<CO_Star>(obj);

                            if star.get_type() == 1 || starmode.isplayerstar(pPlayer) {
                                nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                            } else {
                                nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                            }
                        } else if movingobject_coin == movingtype || movingobject_phantokey == movingtype {
                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if movingobject_collectioncard == movingtype {
                            let iNumHeldCards: i32 = pPlayer.score.subscore[0] as i32;
                            let iHeldCards: i32 = pPlayer.score.subscore[1] as i32;

                            // C++ `break` out of the switch ends this object's iteration: `continue` here.
                            if iNumHeldCards == 3 {
                                let iThreeHeldCards = iHeldCards & 63;

                                //If bot is holding all of the same type of card, then ignore all other cards
                                if iThreeHeldCards == 42 || iThreeHeldCards == 21 || iThreeHeldCards == 0 {
                                    nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                                    continue;
                                }

                                let iTwoHeldCards = iHeldCards & 15;
                                if iTwoHeldCards == 10 || iTwoHeldCards == 5 || iTwoHeldCards == 0 {
                                    nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                                    continue;
                                }
                            } else {
                                let mut card = static_cast::<MO_CollectionCard>(obj);

                                if iNumHeldCards == 2 {
                                    let iTwoHeldCards = iHeldCards & 15;

                                    if iTwoHeldCards == 10 || iTwoHeldCards == 5 || iTwoHeldCards == 0 {
                                        let iCardValue: i32 = card.get_value() as i32;
                                        if card.get_type() == 1
                                            && ((iTwoHeldCards == 10 && iCardValue != 2)
                                                || (iTwoHeldCards == 5 && iCardValue != 1)
                                                || (iTwoHeldCards == 0 && iCardValue != 0))
                                        {
                                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                                            continue;
                                        }
                                    }
                                } else if iNumHeldCards == 1 {
                                    let iHeldCard = iHeldCards & 3;
                                    let iCardValue: i32 = card.get_value() as i32;
                                    if card.get_type() == 1 || iHeldCard != iCardValue {
                                        nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                                        continue;
                                    }
                                }
                            }

                            if iNumHeldCards == 3 {
                                nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                            } else {
                                nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                            }
                        }
                    }
                    object_frenzycard => {
                        nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                    }

                    object_pipe_coin => {
                        let mut coin = static_cast::<OMO_PipeCoin>(obj);

                        if coin.get_color() != 0 {
                            if coin.get_team() == -1 || coin.get_team() == pPlayer.teamID {
                                nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                            }
                        } else {
                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        }
                    }

                    object_pipe_bonus => {
                        let mut bonus = static_cast::<OMO_PipeBonus>(obj);

                        if bonus.get_type() != 5 {
                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else {
                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        }
                    }

                    object_pathhazard | object_orbithazard => {
                        nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                    }

                    object_phanto => {
                        let mut phanto = static_cast::<OMO_Phanto>(obj);

                        if phanto.get_type() == 2
                            || (!carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_phantokey)
                        {
                            nearest!(self.distance_to_object_center(obj, threat, threatdistance, threatwrap));
                        }
                    }

                    object_flamecannon => {
                        // IO_FlameCannon has no own `state`; this is CObject::state.
                        if obj.state > 0 {
                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        }
                    }

                    _ => {
                        nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                    }
                }
            }

            let n = objectcontainer[0].list().len();
            for i in 0..n {
                let mut obj = objectcontainer[0].list()[i];
                if self.attentionObjects.contains_key(&obj.network_id()) {
                    //DistanceToObject(object, &nearestObjects.threat, &nearestObjects.threatdistance, &nearestObjects.threatwrap);
                    continue;
                }

                let r#type: ObjectType = obj.get_object_type();

                match r#type {
                    object_moving => {
                        let movingtype = moving_object_type(obj);

                        if carriedObj == obj {
                            continue;
                        }

                        if movingobject_powerup == movingtype {
                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if (movingobject_fireball == movingtype && static_cast::<MO_Fireball>(obj).iTeamID != iTeamID)
                            || movingobject_poisonpowerup == movingtype
                        {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if (movingobject_goomba == movingtype || movingobject_koopa == movingtype) && obj.get_state() == 1 {
                            nearest!(self.distance_to_object(obj, stomp, stompdistance, stompwrap));
                        } else if movingobject_sledgebrother == movingtype {
                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_treasurechest == movingtype {
                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else if movingobject_flagbase == movingtype {
                            let mut flagbase = static_cast::<MO_FlagBase>(obj);

                            if carriedItem.is_null()
                                || carriedItem.get_moving_object_type() != movingobject_flag
                                || flagbase.get_team_id() != iTeamID
                            {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                        } else {
                            continue;
                        }
                    }

                    object_area => {
                        if static_cast::<OMO_Area>(obj).get_color_id() == pPlayer.colorID {
                            continue;
                        }

                        nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                    }

                    object_kingofthehill_area => {
                        nearest!(self.distance_to_object_center(obj, goal, goaldistance, goalwrap));
                    }

                    _ => {
                        nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                    }
                }
            }

            let n = objectcontainer[2].list().len();
            for i in 0..n {
                let mut obj = objectcontainer[2].list()[i];
                if self.attentionObjects.contains_key(&obj.network_id()) {
                    //DistanceToObject(object, &nearestObjects.threat, &nearestObjects.threatdistance, &nearestObjects.threatwrap);
                    continue;
                }

                let r#type: ObjectType = obj.get_object_type();

                match r#type {
                    object_moving => {
                        let movingtype = moving_object_type(obj);

                        if carriedObj == obj {
                            continue;
                        }

                        if movingobject_cheepcheep == movingtype {
                            nearest!(self.distance_to_object(obj, stomp, stompdistance, stompwrap));
                        } else if movingobject_bulletbill == movingtype {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_hammer == movingtype && static_cast::<MO_Hammer>(obj).iTeamID != iTeamID {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_iceblast == movingtype && static_cast::<MO_IceBlast>(obj).iTeamID != iTeamID {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_boomerang == movingtype && static_cast::<MO_Boomerang>(obj).iTeamID != iTeamID {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_bomb == movingtype && static_cast::<CO_Bomb>(obj).iTeamID != iTeamID {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_podobo == movingtype && static_cast::<MO_Podobo>(obj).iTeamID != iTeamID {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object(obj, threat, threatdistance, threatwrap));
                        } else if movingobject_explosion == movingtype && static_cast::<MO_Explosion>(obj).iTeamID != iTeamID {
                            if fInvincible {
                                continue;
                            }

                            nearest!(self.distance_to_object_center(obj, threat, threatdistance, threatwrap));
                        }
                    }
                    object_thwomp | object_bowserfire => {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object_center(obj, threat, threatdistance, threatwrap));
                    }
                    object_race_goal => {
                        if game_values.gamemode.gamemode != game_mode_race {
                            continue;
                        }

                        let mut racegoal = static_cast::<OMO_RaceGoal>(obj);

                        if racegoal.get_goal_id() != static_cast_gamemode::<CGM_Race>().get_next_goal(iTeamID) {
                            continue;
                        }

                        nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                    }

                    _ => {
                        nearest!(self.distance_to_object(obj, goal, goaldistance, goalwrap));
                    }
                }
            }

            let gmChicken = dynamic_cast_gamemode::<CGM_Chicken>();

            //Figure out where the other players are
            for iPlayer in 0..players.len() {
                if iPlayer == pPlayer.localID as usize || players[iPlayer].state != PlayerState::Ready {
                    continue;
                }

                //Find players in jail on own team to tag
                if game_values.gamemode.gamemode == game_mode_jail {
                    if players[iPlayer].jail.is_active() && players[iPlayer].teamID == iTeamID {
                        nearest!(self.distance_to_player(players[iPlayer], teammate, teammatedistance, teammatewrap));
                    }
                }

                if players[iPlayer].teamID == iTeamID || players[iPlayer].state != PlayerState::Ready {
                    continue;
                }

                //If there is a chicken, only focus on stomping him
                if !gmChicken.is_null() && !gmChicken.chicken().is_null() {
                    if gmChicken.chicken().teamID != iTeamID && gmChicken.chicken() != players[iPlayer] {
                        continue;
                    }
                }

                nearest!(self.distance_to_player(players[iPlayer], player, playerdistance, playerwrap));
            }
        }
    }

    pub fn distance_to_object(&self, object: Ptr<dyn CObjectTrait>, target: &mut Ptr<dyn CObjectTrait>, nearest: &mut i32, wrap: &mut bool) {
        let pPlayer = self.pPlayer;

        //Calculate normal screen
        let mut tx: i16 = (object.x() - pPlayer.ix as i32) as i16;
        let ty: i16 = (object.y() - pPlayer.iy as i32) as i16;
        let mut fScreenWrap = false;

        //See if it is a shorter distance wrapping around the screen
        if tx as i32 > App::screenWidth / 2 {
            tx = (App::screenWidth - tx as i32) as i16;
            fScreenWrap = true;
        } else if (tx as i32) < -App::screenWidth / 2 {
            tx = (App::screenWidth + tx as i32) as i16;
            fScreenWrap = true;
        }

        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32; // a^2 = b^2 + c^2 :)

        if distance_player_pow2 < *nearest {
            *target = object;
            *nearest = distance_player_pow2;
            *wrap = fScreenWrap;
        }
    }

    pub fn distance_to_object_center(&self, object: Ptr<dyn CObjectTrait>, target: &mut Ptr<dyn CObjectTrait>, nearest: &mut i32, wrap: &mut bool) {
        let pPlayer = self.pPlayer;

        //Calculate normal screen
        let mut tx: i16 = (object.x() + (object.collision_rect_w() as i32 / 2) - pPlayer.ix as i32 - HALFPW) as i16;
        let ty: i16 = (object.y() + (object.collision_rect_h() as i32 / 2) - pPlayer.iy as i32 - HALFPH) as i16;
        let mut fScreenWrap = false;

        if tx as i32 > App::screenWidth / 2 {
            tx = (App::screenWidth - tx as i32) as i16;
            fScreenWrap = true;
        } else if (tx as i32) < -App::screenWidth / 2 {
            tx = (App::screenWidth + tx as i32) as i16;
            fScreenWrap = true;
        }

        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32;

        if distance_player_pow2 < *nearest {
            *target = object;
            *nearest = distance_player_pow2;
            *wrap = fScreenWrap;
        }
    }

    pub fn distance_to_player(&self, player: Ptr<CPlayer>, target: &mut Ptr<CPlayer>, nearest: &mut i32, wrap: &mut bool) {
        let pPlayer = self.pPlayer;

        //Calculate normal screen
        let mut tx: i16 = (player.ix as i32 - pPlayer.ix as i32) as i16;
        let ty: i16 = (player.iy as i32 - pPlayer.iy as i32) as i16;
        let mut fScreenWrap = false;

        if tx as i32 > App::screenWidth / 2 {
            tx = (App::screenWidth - tx as i32) as i16;
            fScreenWrap = true;
        } else if (tx as i32) < -App::screenWidth / 2 {
            tx = (App::screenWidth + tx as i32) as i16;
            fScreenWrap = true;
        }

        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32;

        if distance_player_pow2 < *nearest {
            *target = player;
            *nearest = distance_player_pow2;
            *wrap = fScreenWrap;
        }
    }
}

//Setup AI so that it can ignore or pay attention to some objects
pub fn cplayer_ai_init(this: &mut CPlayerAI) {
    unsafe {
        //Scan yoshi's egg mode objects to make sure that we ignore eggs without matching yoshis
        if game_values.gamemode.gamemode == game_mode_eggs {
            let mut fYoshi: [bool; 4] = [false, false, false, false];

            //Scan Yoshis to see which ones are present
            let n = objectcontainer[1].list().len();
            for i in 0..n {
                let obj = objectcontainer[1].list()[i];
                let mut yoshi = dynamic_cast::<MO_Yoshi>(obj);
                if !yoshi.is_null() {
                    fYoshi[yoshi.get_color() as usize] = true;
                }
            }

            //Now scan eggs and ignore any egg that doesn't have a yoshi
            let n = objectcontainer[1].list().len();
            for i in 0..n {
                let obj = objectcontainer[1].list()[i];
                let mut egg = dynamic_cast::<CO_Egg>(obj);
                if !egg.is_null() {
                    if !fYoshi[egg.get_color() as usize] {
                        let ao = Box::new(AttentionObject {
                            iID: egg.network_id(),
                            iType: 1,  //Ignore this object
                            iTimer: 0, //Ignore it forever
                        });

                        this.attentionObjects.insert(ao.iID, ao);
                    }
                }
            }
        }
    }
}

pub fn cplayer_ai_think(this: &mut CPlayerAI, playerKeys: Ptr<COutputControl>) {
    let mut playerKeys = playerKeys;
    let mut pPlayer = this.pPlayer;

    unsafe {
        let iDecisionPercentage: [i16; 5] = [25, 35, 50, 75, 100];

        //Clear out the old input settings
        playerKeys.game_left_mut().fDown = false;
        playerKeys.game_right_mut().fDown = false;

        //A percentage of the time the cpu will do nothing based on the difficulty level
        if RandomNumberGenerator::generator().get_boolean_threshold(100, iDecisionPercentage[game_values.cpudifficulty as usize] as i32)
            && pPlayer.isready()
        {
            return;
        }

        playerKeys.game_jump_mut().fDown = false;
        playerKeys.game_down_mut().fDown = false;
        playerKeys.game_turbo_mut().fDown = false;
        playerKeys.game_powerup_mut().fDown = false;

        if pPlayer.isdead() || pPlayer.isspawning() {
            return;
        }

        let mut gmChicken = dynamic_cast_gamemode::<CGM_Chicken>();
        let mut gmTag = dynamic_cast_gamemode::<CGM_Tag>();

        /***************************************************
         * 1. Figure out what objects are nearest to us
         ***************************************************/
        this.get_nearest_objects();

        let iTenSeconds: i16 = (15 * iDecisionPercentage[game_values.cpudifficulty as usize] as i32 / 10) as i16;

        //If there is a goal, then make sure we aren't paying attention to it for too long
        if !this.nearestObjects.goal.is_null() {
            if this.currentAttentionObject.iID == this.nearestObjects.goal.network_id() {
                //If we have been paying attention to this goal for too long, then start ignoring it
                this.currentAttentionObject.iTimer = this.currentAttentionObject.iTimer.wrapping_add(1);
                if this.currentAttentionObject.iTimer > iTenSeconds {
                    let ao = Box::new(AttentionObject {
                        iID: this.currentAttentionObject.iID,
                        iType: 1, //Ignore this object
                        iTimer: iTenSeconds,
                    });

                    this.attentionObjects.insert(ao.iID, ao);
                }
            } else {
                this.currentAttentionObject.iID = this.nearestObjects.goal.network_id();
                this.currentAttentionObject.iTimer = 0;
            }
        } else {
            this.currentAttentionObject.iID = -1;
        }

        //Expire attention objects
        let mut toDelete: Vec<i32> = Vec::new();
        for (key, ao) in this.attentionObjects.iter_mut() {
            if ao.iTimer > 0 {
                ao.iTimer -= 1;
                if ao.iTimer == 0 {
                    toDelete.push(*key);
                }
            }
        }

        for td in toDelete.iter() {
            // perform the actual disposal and removal
            this.attentionObjects.remove(td);
        }

        let iStoredPowerup: i16 = game_values.gamepowerups[pPlayer.globalID as usize];
        let mut carriedItem = pPlayer.carriedItem;
        let carriedObj = carried_as_object(carriedItem);
        let ix: i16 = pPlayer.ix;
        let iy: i16 = pPlayer.iy;
        let iTeamID: i16 = pPlayer.teamID;

        let ix32 = ix as i32;
        let iy32 = iy as i32;

        /***************************************************
         * 2. Figure out priority of actions
         ***************************************************/

        let actionType: i16;
        let no = &this.nearestObjects;
        if !no.threat.is_null() && no.threatdistance < 14400 && !pPlayer.is_invincible() {
            actionType = 2;
        } else if !no.stomp.is_null() && no.stompdistance < 14400 {
            actionType = 4;
        } else if !no.goal.is_null() && !no.teammate.is_null() {
            if no.goaldistance < no.teammatedistance {
                actionType = 1;
            } else {
                actionType = 3;
            }
        } else if !no.teammate.is_null() {
            actionType = 3;
        } else if !no.goal.is_null() {
            if no.playerdistance < 8100 {
                actionType = 0;
            } else {
                actionType = 1;
            }
        } else {
            actionType = 0;
        }

        /***************************************************
         * 3. Deal with closest item
         ***************************************************/
        //return if no players, goals, or threats available
        if ((actionType != 0) || !no.player.is_null())
            && (actionType != 1 || !no.goal.is_null())
            && ((actionType != 2) || !no.threat.is_null())
            && ((actionType != 3) || !no.teammate.is_null())
            && ((actionType != 4) || !no.stomp.is_null())
            && (this.iFallDanger == 0)
        {
            //Use turbo
            if pPlayer.bobomb {
                if no.playerdistance <= 4096 || no.stompdistance <= 4096 || no.threatdistance <= 4096 {
                    playerKeys.game_turbo_mut().fDown = true;
                }
            } else {
                if pPlayer.powerup == -1 || !RandomNumberGenerator::generator().get_boolean_scale(20) {
                    playerKeys.game_turbo_mut().fDown = true;
                }

                if !carriedItem.is_null() {
                    //Hold important mode goal objects (we'll drop the star later if it is a bad star)
                    let carriedobjecttype = carriedItem.get_moving_object_type();
                    if (carriedobjecttype == movingobject_egg)
                        || (carriedobjecttype == movingobject_flag)
                        || (carriedobjecttype == movingobject_star)
                        || (carriedobjecttype == movingobject_phantokey)
                    {
                        playerKeys.game_turbo_mut().fDown = true;
                    }
                }
            }

            if actionType == 0 {
                //Kill nearest player
                let mut player = no.player;
                let moveToward: *mut bool;
                let moveAway: *mut bool;

                if (player.ix > ix && no.playerwrap) || (player.ix < ix && !no.playerwrap) {
                    moveToward = &mut playerKeys.game_left_mut().fDown;
                    moveAway = &mut playerKeys.game_right_mut().fDown;
                } else {
                    moveToward = &mut playerKeys.game_right_mut().fDown;
                    moveAway = &mut playerKeys.game_left_mut().fDown;
                }

                let pix = player.ix as i32;
                let piy = player.iy as i32;

                //Move player relative to opponent when we are playing star mode and we are holding a star
                if game_values.gamemode.gamemode == game_mode_star {
                    if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_star {
                        if static_cast::<CO_Star>(carriedObj).get_type() == 1 {
                            *moveAway = true;
                        } else {
                            *moveToward = true;

                            //If player is close
                            if piy <= iy32 && piy > iy32 - 60 && pix - ix32 < 90 && pix - ix32 > -90 {
                                //And we are facing toward that player, throw the star
                                if (pix > ix32 && pPlayer.is_facing_right()) || ((pix < ix32) && !pPlayer.is_facing_right()) {
                                    pPlayer.throw_star = 30;
                                    playerKeys.game_turbo_mut().fDown = false;
                                }
                            }
                        }
                    }
                }
                //Move player relative to opponent when we are carrying a weapon like a shell or throwblock
                else if !carriedItem.is_null() {
                    if carriedItem.get_moving_object_type() == movingobject_shell
                        || carriedItem.get_moving_object_type() == movingobject_throwblock
                    {
                        *moveToward = true;

                        //If player is close
                        if piy > iy32 - 10 && piy < iy32 + 30 && (pix - ix32).abs() < 150 {
                            //And we are facing toward that player, throw the projectile
                            if (pix > ix32 && pPlayer.is_facing_right()) || ((pix < ix32) && !pPlayer.is_facing_right()) {
                                playerKeys.game_turbo_mut().fDown = false;
                            }
                        }
                    }
                } else if (!gmTag.is_null() && gmTag.tagged() == player)
                    || player.is_invincible()
                    || player.shyguy
                    || (!gmChicken.is_null() && gmChicken.chicken() == pPlayer)
                {
                    *moveAway = true;
                } else if pPlayer.is_invincible() || pPlayer.shyguy || pPlayer.bobomb || (!gmTag.is_null() && gmTag.tagged() == pPlayer) {
                    *moveToward = true;
                } else if piy >= iy32 && !player.is_invincible() && !player.bobomb {
                    *moveToward = true;
                } else {
                    if no.playerdistance < 8100 {
                        //else if player is near but higher, run away (left)
                        *moveAway = true;
                    } else {
                        *moveToward = true; //Don't just stand and do nothing
                    }
                }

                if piy <= iy32 &&					//jump if player is higher or at the same level and
                    pix - ix32 < 45 &&				//player is very near
                    pix - ix32 > -45
                {
                    //or if player is high
                    playerKeys.game_jump_mut().fDown = true;
                } else if piy > iy32 &&			//try to down jump if player is below us
                    pix - ix32 < 45 &&
                    pix - ix32 > -45 && RANDOM_BOOL()
                {
                    //or if player is high
                    if !pPlayer.inair {
                        playerKeys.game_down_mut().fDown = true;
                    }

                    if !pPlayer.superstomp.is_in_super_stomp_state() {
                        //If the player has the tanooki or shoe, then try to super stomp on them
                        if pPlayer.tanookisuit.is_on() || pPlayer.kuriboshoe.is_on() {
                            playerKeys.game_down_mut().fDown = true;
                            playerKeys.game_jump_mut().fPressed = true;
                            if pPlayer.tanookisuit.is_on() {
                                playerKeys.game_turbo_mut().fPressed = true;
                                pPlayer.lockfire = false;
                            }
                        }
                    }
                }
            } else if actionType == 1 {
                //Go for goal
                let mut goal = no.goal;

                if (goal.x() > ix32 && no.goalwrap) || (goal.x() < ix32 && !no.goalwrap) {
                    playerKeys.game_left_mut().fDown = true;
                } else {
                    playerKeys.game_right_mut().fDown = true;
                }

                if goal.y() <= iy32 && goal.x() - ix32 < 45 && goal.x() - ix32 > -45 {
                    playerKeys.game_jump_mut().fDown = true;
                } else if goal.y() > iy32 && goal.x() - ix32 < 45 && goal.x() - ix32 > -45 {
                    if !pPlayer.inair {
                        playerKeys.game_down_mut().fDown = true;
                    }
                }

                if !dynamic_cast::<CO_Egg>(goal).is_null() {
                    playerKeys.game_turbo_mut().fDown = true;
                } else if !dynamic_cast::<CO_Star>(goal).is_null() && pPlayer.throw_star == 0 {
                    playerKeys.game_turbo_mut().fDown = true;
                } else if !dynamic_cast::<CO_Flag>(goal).is_null() {
                    playerKeys.game_turbo_mut().fDown = true;
                }

                //Drop current item if we're going after another carried item
                if !carriedItem.is_null() {
                    if let Some(movingobject) = goal.as_io_moving_object() {
                        let goalobjecttype = movingobject.get_moving_object_type();
                        if goalobjecttype == movingobject_egg
                            || goalobjecttype == movingobject_flag
                            || goalobjecttype == movingobject_star
                            || goalobjecttype == movingobject_phantokey
                        {
                            let carriedobjecttype = carriedItem.get_moving_object_type();

                            //If we are holding something that isn't a mode goal object, then drop it
                            if carriedobjecttype != movingobject_egg
                                && carriedobjecttype != movingobject_flag
                                && carriedobjecttype != movingobject_star
                                && carriedobjecttype != movingobject_phantokey
                            {
                                playerKeys.game_turbo_mut().fDown = false;
                            }
                        }
                    }
                }

                //Open treasure chests in bonus houses in world mode
                if !dynamic_cast::<MO_BonusHouseChest>(goal).is_null() {
                    playerKeys.game_turbo_mut().fPressed = true;
                }
            } else if actionType == 2 {
                //Evade Threat
                let threat = no.threat;

                //If threat, always use turbo
                playerKeys.game_turbo_mut().fDown = true;

                if (threat.x() > ix32 && no.threatwrap) || (threat.x() < ix32 && !no.threatwrap) {
                    playerKeys.game_right_mut().fDown = true;
                } else {
                    playerKeys.game_left_mut().fDown = true;
                }

                if threat.y() <= iy32 && threat.x() - ix32 < 60 && threat.x() - ix32 > -60 {
                    if !pPlayer.inair {
                        playerKeys.game_down_mut().fDown = true;
                    }
                } else if threat.y() > iy32 && threat.x() - ix32 < 60 && threat.x() - ix32 > -60 {
                    playerKeys.game_jump_mut().fDown = true;
                }
            } else if actionType == 3 {
                //Tag teammate
                let teammate = no.teammate;
                let tix = teammate.ix as i32;
                let tiy = teammate.iy as i32;

                if (tix > ix32 && no.teammatewrap) || (tix < ix32 && !no.teammatewrap) {
                    playerKeys.game_left_mut().fDown = true;
                } else {
                    playerKeys.game_right_mut().fDown = true;
                }

                if tiy <= iy32 && tix - ix32 < 45 && tix - ix32 > -45 {
                    playerKeys.game_jump_mut().fDown = true;
                } else if tiy > iy32 && tix - ix32 < 45 && tix - ix32 > -45 {
                    if !pPlayer.inair {
                        playerKeys.game_down_mut().fDown = true;
                    }
                }
            } else if actionType == 4 {
                //Stomp something (goomba, koopa, cheepcheep)
                let stomp = no.stomp;
                let moveToward: *mut bool;
                let moveAway: *mut bool;

                if (stomp.x() > ix32 && no.stompwrap) || (stomp.x() < ix32 && !no.stompwrap) {
                    moveToward = &mut playerKeys.game_left_mut().fDown;
                    moveAway = &mut playerKeys.game_right_mut().fDown;
                } else {
                    moveToward = &mut playerKeys.game_right_mut().fDown;
                    moveAway = &mut playerKeys.game_left_mut().fDown;
                }

                if stomp.y() > iy32 + PH
                    || pPlayer.shyguy
                    || pPlayer.is_invincible()
                    || (!carriedItem.is_null()
                        && (carriedItem.get_moving_object_type() == movingobject_shell
                            || carriedItem.get_moving_object_type() == movingobject_throwblock))
                {
                    //if true stomp target is lower or at the same level, run toward
                    *moveToward = true;
                } else {
                    if no.stompdistance < 8100 {
                        *moveAway = true;
                    } else {
                        *moveToward = true;
                    }
                }

                if stomp.y() <= iy32 + PH && no.stompdistance < 2025 {
                    playerKeys.game_jump_mut().fDown = true;
                } else if stomp.y() > iy32 + PH && no.stompdistance < 2025 {
                    if !pPlayer.inair {
                        playerKeys.game_down_mut().fDown = true;
                    }

                    if !pPlayer.superstomp.is_in_super_stomp_state() {
                        //If the player has the tanooki or shoe, then try to super stomp on them
                        if pPlayer.tanookisuit.is_on() || pPlayer.kuriboshoe.is_on() {
                            playerKeys.game_down_mut().fDown = true;
                            playerKeys.game_jump_mut().fPressed = true;
                            if pPlayer.tanookisuit.is_on() {
                                playerKeys.game_turbo_mut().fPressed = true;
                                pPlayer.lockfire = false;
                            }
                        }
                    }
                }
            }
        }

        //Jump if trying to move left/right and x velocity is zero
        if (playerKeys.game_left().fDown || playerKeys.game_right().fDown) && pPlayer.velx == 0.0f32 {
            playerKeys.game_jump_mut().fDown = true;
        }

        //Pick up throwable blocks from below
        if playerKeys.game_turbo().fDown && carriedItem.is_null() && !pPlayer.inair {
            playerKeys.game_turbo_mut().fPressed = true;
        }

        //Stay inside tanooki statue
        if pPlayer.tanookisuit.is_on() && pPlayer.tanookisuit.is_statue() {
            playerKeys.game_down_mut().fDown = true;
        }

        //"Star Mode" specific stuff
        //Drop the star if we're not it
        if game_values.gamemode.gamemode == game_mode_star {
            let mut starmode = static_cast_gamemode::<CGM_Star>();

            if !carriedItem.is_null()
                && carriedItem.get_moving_object_type() == movingobject_star
                && static_cast::<CO_Star>(carriedObj).get_type() == 0
                && !starmode.isplayerstar(pPlayer)
            {
                playerKeys.game_turbo_mut().fDown = false;
            } else if starmode.isplayerstar(pPlayer) && pPlayer.throw_star == 0 {
                playerKeys.game_turbo_mut().fDown = true;
            }
        }

        if !carriedItem.is_null() {
            let carriedobjecttype = carriedItem.get_moving_object_type();
            let no = &this.nearestObjects;

            //"Phanto Mode" specific stuff
            if game_values.gamemode.gamemode == game_mode_chase {
                //Ignore the key if a phanto is really close
                if !no.threat.is_null()
                    && no.threat.get().get_object_type() == object_phanto
                    && no.threatdistance < 4096
                    && !pPlayer.is_invincible()
                {
                    playerKeys.game_turbo_mut().fDown = false;

                    //Ignore the key for a little while
                    let carriedID = carriedItem.network_id();
                    if let Some(ao) = this.attentionObjects.get_mut(&carriedID) {
                        ao.iType = 1;
                        ao.iTimer = iDecisionPercentage[game_values.cpudifficulty as usize] / 3;
                    } else {
                        let ao = Box::new(AttentionObject {
                            iID: carriedID,
                            iType: 1,
                            iTimer: iDecisionPercentage[game_values.cpudifficulty as usize] / 3,
                        });
                        this.attentionObjects.insert(ao.iID, ao);
                    }
                }
            }
            //If we are holding something that we are ignoring, drop it
            else if this.attentionObjects.contains_key(&carriedItem.network_id()) {
                playerKeys.game_turbo_mut().fDown = false;
            }

            //Drop live bombs if they are not yours
            let bomb = dynamic_cast::<CO_Bomb>(carriedObj);
            if !bomb.is_null() {
                if bomb.iTeamID != pPlayer.teamID {
                    playerKeys.game_turbo_mut().fDown = false;
                }
            } else if carriedobjecttype == movingobject_carried || carriedobjecttype == movingobject_throwbox {
                //Drop springs,spikes,shoes
                playerKeys.game_turbo_mut().fDown = false;
            }
        }

        /***************************************************
         * 4. Deal with falling onto spikes (this needs to be improved)
         ***************************************************/

        //Make sure we don't jump over something that could kill us
        let mut iDeathY: i16 = (iy32 / TILESIZE) as i16;
        let iDeathX1: i16 = (ix32 / TILESIZE) as i16;
        let iDeathX2: i16;

        if ix32 + PW >= App::screenWidth {
            iDeathX2 = ((ix32 + PW - App::screenWidth) / TILESIZE) as i16;
        } else {
            iDeathX2 = ((ix32 + PW) / TILESIZE) as i16;
        }

        if iDeathY < 0 {
            iDeathY = 0;
        }

        //short depth = -1;
        // `break 'ExitDeathCheck` is the C++ `goto ExitDeathCheck`.
        'ExitDeathCheck: while (iDeathY as i32) < MAPHEIGHT {
            let ttLeftTile: i32 = ai_map(iDeathX1 as i32, iDeathY as i32);
            let ttRightTile: i32 = ai_map(iDeathX2 as i32, iDeathY as i32);

            let fDeathTileUnderPlayer1 = ((ttLeftTile & tile_flag_death_on_top) != 0 && (ttRightTile & tile_flag_death_on_top) != 0)
                || ((ttLeftTile & tile_flag_death_on_top) != 0 && (ttRightTile & tile_flag_solid) == 0)
                || ((ttLeftTile & tile_flag_solid) == 0 && (ttRightTile & tile_flag_death_on_top) != 0);

            if fDeathTileUnderPlayer1 {
                if this.iFallDanger == 0 {
                    this.iFallDanger = if playerKeys.game_right().fDown { -1 } else { 1 };
                }

                break 'ExitDeathCheck;
            } else if (ttLeftTile & tile_flag_solid) != 0
                || (ttLeftTile & tile_flag_solid_on_top) != 0
                || ai_block_nonnull(iDeathX1, iDeathY)
                || (ttRightTile & tile_flag_solid) != 0
                || (ttRightTile & tile_flag_solid_on_top) != 0
                || ai_block_nonnull(iDeathX2, iDeathY)
            {
                this.iFallDanger = 0;
                break 'ExitDeathCheck;
            }

            //Look through all platforms and see if we are hitting solid or death tiles in them
            let mut iPlatform: i16 = 0;
            while (iPlatform as usize) < g_map.platforms.len() {
                let lefttile: i32 = g_map.platforms[iPlatform as usize].get_tile_type_from_coord(ix, ((iDeathY as i32) << 5) as i16);
                let righttile: i32 =
                    g_map.platforms[iPlatform as usize].get_tile_type_from_coord((ix32 + PW) as i16, ((iDeathY as i32) << 5) as i16);

                let fDeathTileUnderPlayer2 = ((lefttile & tile_flag_death_on_top) != 0 && (righttile & tile_flag_death_on_top) != 0)
                    || ((lefttile & tile_flag_death_on_top) != 0 && (righttile & tile_flag_solid) == 0)
                    || ((lefttile & tile_flag_solid) == 0 && (righttile & tile_flag_death_on_top) != 0);

                if fDeathTileUnderPlayer2 {
                    if this.iFallDanger == 0 {
                        this.iFallDanger = if playerKeys.game_right().fDown { -1 } else { 1 };
                    }

                    break 'ExitDeathCheck;
                } else if (lefttile & tile_flag_solid) != 0
                    || (lefttile & tile_flag_solid_on_top) != 0
                    || (righttile & tile_flag_solid) != 0
                    || (righttile & tile_flag_solid_on_top) != 0
                {
                    this.iFallDanger = 0;
                    break 'ExitDeathCheck;
                }

                iPlatform += 1;
            }

            iDeathY += 1;
        }

        //If we are done checking for death under the player, come here
        //ExitDeathCheck:

        //There is a death tile below us so move to the side that is safest
        if this.iFallDanger < 0 {
            playerKeys.game_right_mut().fDown = true;
            playerKeys.game_left_mut().fDown = false;
            playerKeys.game_jump_mut().fDown = true;
            playerKeys.game_turbo_mut().fDown = true;
            playerKeys.game_down_mut().fDown = false;
        } else if this.iFallDanger > 0 {
            playerKeys.game_right_mut().fDown = false;
            playerKeys.game_left_mut().fDown = true;
            playerKeys.game_jump_mut().fDown = true;
            playerKeys.game_turbo_mut().fDown = true;
            playerKeys.game_down_mut().fDown = false;
        }

        //Make sure we don't jump up into something that can kill us
        iDeathY = (iy32 / TILESIZE) as i16;

        if iDeathY < 0 {
            iDeathY = 0;
        }

        let mut heightlimit: i16 = 3;
        while iDeathY >= 0 && heightlimit > 0 {
            let ttLeftTile: i32 = ai_map(iDeathX1 as i32, iDeathY as i32);
            let ttRightTile: i32 = ai_map(iDeathX2 as i32, iDeathY as i32);

            if heightlimit == 2
                && ((ttLeftTile & tile_flag_solid) != 0 || (ttRightTile & tile_flag_solid) != 0)
                && !ai_checkforwarp_up(iDeathX1, iDeathX2, iDeathY)
            {
                //Avoid jumping wildly in 1 tile high gaps
                playerKeys.game_jump_mut().fDown = false;
            }

            if ((ttLeftTile & tile_flag_solid) != 0 && (ttLeftTile & tile_flag_death_on_bottom) == 0)
                || ((ttRightTile & tile_flag_solid) != 0 && (ttRightTile & tile_flag_death_on_bottom) == 0)
                || ai_block_nonnull(iDeathX1, iDeathY)
                || ai_block_nonnull(iDeathX2, iDeathY)
            {
                break;
            } else if (ttLeftTile & tile_flag_death_on_bottom) != 0 || (ttRightTile & tile_flag_death_on_bottom) != 0 {
                playerKeys.game_jump_mut().fDown = false;
                break;
            }

            iDeathY -= 1;
            heightlimit -= 1;
        }

        /***************************************************
         * 5. Use stored powerups
         ***************************************************/

        if iStoredPowerup > 0 {
            let canUse = |powerup: PowerupType| -> bool {
                match powerup {
                    PowerupType::PoisonMushroom => return false,
                    PowerupType::ExtraLife1
                    | PowerupType::ExtraLife2
                    | PowerupType::ExtraLife3
                    | PowerupType::ExtraLife5
                    | PowerupType::Clock
                    | PowerupType::Pow
                    | PowerupType::BulletBill
                    | PowerupType::Mod
                    | PowerupType::Podobo => return true, // use 1-5up, clock, pow, bulletbill, mod, podobo, right away
                    PowerupType::Fire
                    | PowerupType::Hammer
                    | PowerupType::Feather
                    | PowerupType::Boomerang
                    | PowerupType::IceWand
                    | PowerupType::Bomb
                    | PowerupType::Leaf
                    | PowerupType::PWings => return pPlayer.powerup == -1,
                    PowerupType::Star => return !pPlayer.is_invincible(),
                    PowerupType::Bobomb => return !pPlayer.bobomb,
                    PowerupType::ShellGreen | PowerupType::ShellRed | PowerupType::ShellSpiny | PowerupType::ShellBuzzy => {
                        return carriedItem.is_null()
                    }
                    PowerupType::Tanooki => return !pPlayer.tanookisuit.is_on(),
                    PowerupType::JailKey => return pPlayer.jail.is_active(),
                    PowerupType::MysteryMushroom => {
                        //See if another player has a powerup
                        for iPlayer in 0..players.len() {
                            if iPlayer == pPlayer.localID as usize || players[iPlayer].teamID == iTeamID {
                                continue;
                            }
                            if game_values.gamepowerups[players[iPlayer].globalID as usize] > 0 {
                                return true;
                            }
                        }
                    }
                }
                false
            };
            // `static_cast<PowerupType>(iStoredPowerup)`: values past the last enumerator match no case.
            if (iStoredPowerup as usize) < POWERUP_TYPES.len() && canUse(POWERUP_TYPES[iStoredPowerup as usize]) {
                playerKeys.game_powerup_mut().fDown = true;
            }
        }
    }
}

/**************************************************
 * CSimpleAI class
 ***************************************************/
pub struct CSimpleAI {
    pub cplayer_ai: CPlayerAI,
}
crate::impl_base!(CSimpleAI => cplayer_ai: CPlayerAI);

impl CSimpleAI {
    pub fn new() -> Self {
        CSimpleAI { cplayer_ai: CPlayerAI::new() }
    }
}

impl Default for CSimpleAI {
    fn default() -> Self {
        Self::new()
    }
}

impl CPlayerAITrait for CSimpleAI {
    fn ai(&mut self) -> &mut CPlayerAI {
        &mut self.cplayer_ai
    }

    //This simple ai makes the player jump every so often
    //The jump is to the maximum jump height, then the game_jump.fDown is released
    fn think(&mut self, playerKeys: Ptr<COutputControl>) {
        let mut playerKeys = playerKeys;
        let pPlayer = self.pPlayer;

        playerKeys.game_left_mut().fDown = false;
        playerKeys.game_right_mut().fDown = false;

        playerKeys.game_down_mut().fDown = false;
        playerKeys.game_turbo_mut().fDown = false;
        playerKeys.game_powerup_mut().fDown = false;

        if pPlayer.isdead() || pPlayer.isspawning() {
            return;
        }

        //Hold down jump until player starts moving down again, then release jump button
        if pPlayer.inair {
            if pPlayer.vely > 0.0f32 {
                playerKeys.game_jump_mut().fDown = false;
            } else {
                playerKeys.game_jump_mut().fDown = true;
            }
        } else {
            //Try to jump 1 out of 50 chances when on ground
            if RandomNumberGenerator::generator().get_boolean_scale(50) {
                playerKeys.game_jump_mut().fDown = true;
            } else {
                playerKeys.game_jump_mut().fDown = false;
            }
        }
    }
}
