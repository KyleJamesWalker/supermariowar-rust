//! Port of src/smw/ai.cpp

use crate::smw::object_container::ObjectId;
use crate::common::map::{WARP_DOWN, WARP_LEFT, WARP_RIGHT};
use crate::smw::ai_nav::{Move, NavGraph, Route};
use crate::smw::objects::blocks::powerup_block::B_PowerupBlock;
use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::global_constants::*;
use crate::common::input::COutputControl;
use crate::common::moving_object_types::*;
use crate::common::object_base::*;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType, RANDOM_BOOL, RANDOM_INT};
use crate::common::tile_types::*;
use crate::globals::*;
use crate::smw::gamemodes::chicken::CGM_Chicken;
use crate::smw::gamemodes::race::CGM_Race;
use crate::smw::gamemodes::star::CGM_Star;
use crate::smw::gamemodes::tag::CGM_Tag;
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

#[derive(Clone)]

pub struct NearestObjects {
    pub player: PlayerId,
    pub teammate: PlayerId,

    pub goal: ObjectId,
    pub stomp: ObjectId,
    pub threat: ObjectId,

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
            player: PlayerId::NONE,
            teammate: PlayerId::NONE,
            goal: ObjectId::NONE,
            stomp: ObjectId::NONE,
            threat: ObjectId::NONE,
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
        self.player = PlayerId::NONE;
        self.goal = ObjectId::NONE;
        self.stomp = ObjectId::NONE;
        self.teammate = PlayerId::NONE;
        self.threat = ObjectId::NONE;

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
pub trait ClonePlayerAI {
    fn clone_box(&self) -> Box<dyn CPlayerAITrait>;
}

impl<T: CPlayerAITrait + Clone + 'static> ClonePlayerAI for T {
    fn clone_box(&self) -> Box<dyn CPlayerAITrait> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn CPlayerAITrait> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait CPlayerAITrait: ClonePlayerAI {
    fn ai(&mut self) -> &mut CPlayerAI;

    fn set_player(&mut self, player: PlayerId) {
        self.ai().pPlayer = player;
    }
    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        cplayer_ai_init(gp.get(), self.ai())
    }
    fn think(&mut self, g: &mut Game, playerKeys: Ptr<COutputControl>) {
        let mut gp = g.ptr();
        cplayer_ai_think(gp.get(), self.ai(), playerKeys)
    }
}

#[derive(Clone)]

pub struct CPlayerAI {
    pub pPlayer: PlayerId,

    pub iFallDanger: i16,
    pub nearestObjects: NearestObjects,

    pub attentionObjects: BTreeMap<i32, Box<AttentionObject>>,
    pub currentAttentionObject: AttentionObject,
    /// Frames in a row the AI has jumped at a wall it is walking into.
    pub wall_frames: i16,
    /// An opponent the AI gave up on behind a wall, and for how many more frames.
    pub ignored_player: (PlayerId, i16),
    pub nav: Option<NavGraph>,
    /// The tile column and move the AI left the ground for, followed while in the air.
    pub nav_step: Option<(i32, Move)>,
    pub style: Style,
    /// Thinks since `nav` was built, on a map whose moving platforms make it go stale.
    pub nav_age: u8,

    pub _alias: Aliased,
}

/// A CPU's play style (docs/CPU_AI.md, step 9), drawn at match start.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Style {
    #[default]
    Balanced,
    /// Leaves the objective for opponents farther away and backs off from fewer of them.
    Aggressive,
    /// Leaves the objective only for opponents right next to it.
    Objective,
    /// Keeps more distance from threats and from opponents above it.
    Cautious,
}

/// Squared distances that pull a CPU of `style` off its goal: a stomp target and the nearest player in objective modes,
/// the same in other modes, a threat to run from, and an opponent above it to back away from.
struct StyleRanges {
    objective: (i32, i32),
    open: (i32, i32),
    threat: i32,
    back_off: i32,
}

fn style_ranges(style: Style) -> StyleRanges {
    match style {
        Style::Balanced => StyleRanges { objective: (3600, 2025), open: (14400, 8100), threat: 14400, back_off: 8100 },
        Style::Aggressive => StyleRanges { objective: (4900, 3600), open: (22500, 14400), threat: 8100, back_off: 3600 },
        Style::Objective => StyleRanges { objective: (1600, 900), open: (14400, 8100), threat: 14400, back_off: 8100 },
        Style::Cautious => StyleRanges { objective: (3600, 2025), open: (10000, 4900), threat: 22500, back_off: 14400 },
    }
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
fn dynamic_cast_gamemode<T: Any>(g: &mut Game) -> Ptr<T> {
    let gm = g.mode();
    if gm.is_null() {
        return Ptr::null();
    }
    match gm.get().as_any().downcast_mut::<T>() {
        Some(t) => Ptr::from_mut(t),
        None => Ptr::null(),
    }
}

/// `(T*)game_values.gamemode`
fn static_cast_gamemode<T: Any>(g: &mut Game) -> Ptr<T> {
    let p = dynamic_cast_gamemode::<T>(g);
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
const CMAP_OFF_SPAWNAREAS: i32 = 28664;
const CMAP_OFF_NUMSPAWNAREAS: i32 = 36344;
const CMAP_OFF_TOTALSPAWNSIZE: i32 = 36356;
const CMAP_OFF_WARPDATA: i32 = 36368;
const CMAP_OFF_NUMWARPEXITS: i32 = 38768;
const CMAP_OFF_WARPEXITS: i32 = 38772;
const CMAP_OFF_WARPLOCKTIMER: i32 = 39540;
const CMAP_OFF_WARPLOCKED: i32 = 39560;
const CMAP_OFF_MAXCONNECTION: i32 = 39570;
const CMAP_SIZEOF_MAPBLOCK: i32 = 56;
const CMAP_SIZEOF_WARPEXIT: i32 = 24;
const CMAP_SIZEOF_SPAWNAREA: i32 = 10;

fn in_map(x: i32, y: i32) -> bool {
    (0..MAPWIDTH).contains(&x) && (0..MAPHEIGHT).contains(&y)
}

/// One byte of the C++ CMap object at `off`, for the members between mapdata and blockdata.
fn cmap_byte(g: &mut Game, off: i32) -> u8 {
    let cells = MAPWIDTH * MAPHEIGHT;
    if (CMAP_OFF_MAPDATA..CMAP_OFF_MAPDATA + cells * MAPLAYERS * 6).contains(&off) {
        let rel = off - CMAP_OFF_MAPDATA;
        let e = rel / 6;
        let t = &g.g_map.mapdata[(e / (MAPHEIGHT * MAPLAYERS)) as usize][((e / MAPLAYERS) % MAPHEIGHT) as usize][(e % MAPLAYERS) as usize];
        let v = [t.iID, t.iCol, t.iRow][((rel % 6) / 2) as usize];
        return v.to_le_bytes()[(rel % 2) as usize];
    }
    if (CMAP_OFF_MAPDATATOP..CMAP_OFF_MAPDATATOP + cells).contains(&off) {
        let e = off - CMAP_OFF_MAPDATATOP;
        return g.g_map.mapdatatop[(e / MAPHEIGHT) as usize][(e % MAPHEIGHT) as usize].0;
    }
    if (CMAP_OFF_OBJECTDATA..CMAP_OFF_OBJECTDATA + cells * CMAP_SIZEOF_MAPBLOCK).contains(&off) {
        let rel = off - CMAP_OFF_OBJECTDATA;
        let e = rel / CMAP_SIZEOF_MAPBLOCK;
        let b = &g.g_map.objectdata[(e / MAPHEIGHT) as usize][(e % MAPHEIGHT) as usize];
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
        return g.g_map.nospawn[(rel / cells) as usize][((rel % cells) / MAPHEIGHT) as usize][(rel % MAPHEIGHT) as usize] as u8;
    }
    if (CMAP_OFF_SPAWNAREAS..CMAP_OFF_NUMSPAWNAREAS).contains(&off) {
        let rel = off - CMAP_OFF_SPAWNAREAS;
        let e = rel / CMAP_SIZEOF_SPAWNAREA;
        let a = &g.g_map.spawnareas[(e / MAXSPAWNAREAS) as usize][(e % MAXSPAWNAREAS) as usize];
        let r = (rel % CMAP_SIZEOF_SPAWNAREA) as usize;
        return [a.left, a.top, a.width, a.height, a.size][r / 2].to_le_bytes()[r % 2];
    }
    if (CMAP_OFF_NUMSPAWNAREAS..CMAP_OFF_TOTALSPAWNSIZE).contains(&off) {
        let rel = (off - CMAP_OFF_NUMSPAWNAREAS) as usize;
        return g.g_map.numspawnareas[rel / 2].to_le_bytes()[rel % 2];
    }
    if (CMAP_OFF_TOTALSPAWNSIZE..CMAP_OFF_WARPDATA).contains(&off) {
        let rel = (off - CMAP_OFF_TOTALSPAWNSIZE) as usize;
        return g.g_map.totalspawnsize[rel / 2].to_le_bytes()[rel % 2];
    }
    if (CMAP_OFF_WARPDATA..CMAP_OFF_NUMWARPEXITS).contains(&off) {
        let rel = off - CMAP_OFF_WARPDATA;
        let e = rel / 8;
        let w = &g.g_map.warpdata[(e / MAPHEIGHT) as usize][(e % MAPHEIGHT) as usize];
        let r = (rel % 8) as usize;
        return match r {
            0..=3 => (w.direction as i32).to_le_bytes()[r],
            4..=5 => w.connection.to_le_bytes()[r - 4],
            _ => w.id.to_le_bytes()[r - 6],
        };
    }
    if (CMAP_OFF_NUMWARPEXITS..CMAP_OFF_WARPEXITS).contains(&off) {
        let r = (off - CMAP_OFF_NUMWARPEXITS) as usize;
        return if r < 2 { g.g_map.numwarpexits.to_le_bytes()[r] } else { 0 };
    }
    if (CMAP_OFF_WARPEXITS..CMAP_OFF_WARPLOCKTIMER).contains(&off) {
        let rel = off - CMAP_OFF_WARPEXITS;
        let x = &g.g_map.warpexits[(rel / CMAP_SIZEOF_WARPEXIT) as usize];
        let r = (rel % CMAP_SIZEOF_WARPEXIT) as usize;
        if r < 4 {
            return (x.direction as i32).to_le_bytes()[r];
        }
        let v = [x.connection, x.id, x.x, x.y, x.lockx, x.locky, x.warpx, x.warpy, x.numblocks, x.locktimer][(r - 4) / 2];
        return v.to_le_bytes()[r % 2];
    }
    if (CMAP_OFF_WARPLOCKTIMER..CMAP_OFF_WARPLOCKED).contains(&off) {
        let rel = (off - CMAP_OFF_WARPLOCKTIMER) as usize;
        return g.g_map.warplocktimer[rel / 2].to_le_bytes()[rel % 2];
    }
    if (CMAP_OFF_WARPLOCKED..CMAP_OFF_MAXCONNECTION).contains(&off) {
        return g.g_map.warplocked[(off - CMAP_OFF_WARPLOCKED) as usize] as u8;
    }
    if (CMAP_OFF_OBJECTDATA + MAPWIDTH * MAPHEIGHT * CMAP_SIZEOF_MAPBLOCK..CMAP_OFF_BLOCKDATA).contains(&off) {
        return 0;
    }
    panic!("ai: C++ out-of-bounds CMap read at offset {} is not modelled", off);
}

unsafe fn cmap_i16(g: &mut Game, off: i32) -> i16 {
    i16::from_le_bytes([cmap_byte(g, off), cmap_byte(g, off + 1)])
}

unsafe fn cmap_i32(g: &mut Game, off: i32) -> i32 {
    i32::from_le_bytes([cmap_byte(g, off), cmap_byte(g, off + 1), cmap_byte(g, off + 2), cmap_byte(g, off + 3)])
}

/// `g_map->map(x, y)`, including the C++ out-of-bounds reads.
unsafe fn ai_map(g: &mut Game, x: i32, y: i32) -> i32 {
    if in_map(x, y) {
        return g.g_map.map(x, y);
    }
    tile_to_flags(TileType(cmap_byte(g, CMAP_OFF_MAPDATATOP + x * MAPHEIGHT + y))) as i32
}

/// `g_map->block(x, y) != NULL`, including the C++ out-of-bounds reads.
unsafe fn ai_block_nonnull(g: &mut Game, x: i16, y: i16) -> bool {
    if in_map(x as i32, y as i32) {
        return !g.map_block(x, y).is_null();
    }
    let off = CMAP_OFF_BLOCKDATA + (x as i32 * MAPHEIGHT + y as i32) * 8;
    (0..8).any(|i| cmap_byte(g, off + i) != 0)
}

/// `&warpdata[x][y]` as (direction, connection, id), including the C++ out-of-bounds reads.
unsafe fn ai_warp(g: &mut Game, x: i16, y: i16) -> (i32, i16, i16) {
    if in_map(x as i32, y as i32) {
        let w = &g.g_map.warpdata[x as usize][y as usize];
        return (w.direction as i32, w.connection, w.id);
    }
    let off = CMAP_OFF_WARPDATA + (x as i32 * MAPHEIGHT + y as i32) * 8;
    (cmap_i32(g, off), cmap_i16(g, off + 4), cmap_i16(g, off + 6))
}

/// `g_map->checkforwarp(x1, x2, y, 2)`, including the C++ out-of-bounds reads.
unsafe fn ai_checkforwarp_up(g: &mut Game, x1: i16, x2: i16, y: i16) -> bool {
    if in_map(x1 as i32, y as i32) && in_map(x2 as i32, y as i32) {
        return g.g_map.checkforwarp(x1, x2, y, 2);
    }
    let w1 = ai_warp(g, x1, y);
    let w2 = ai_warp(g, x2, y);
    if !(w1.0 == w2.0 && w1.2 == w2.2 && w1.0 == 2) {
        return false;
    }
    let locked = if (0..10).contains(&w1.1) { g.g_map.warplocked[w1.1 as usize] as u8 } else { cmap_byte(g, CMAP_OFF_WARPLOCKED + w1.1 as i32) };
    if locked != 0 {
        return false;
    }
    let locktimer = if (0..32).contains(&w1.2) {
        g.g_map.warpexits[w1.2 as usize].locktimer
    } else {
        cmap_i16(g, CMAP_OFF_WARPEXITS + w1.2 as i32 * CMAP_SIZEOF_WARPEXIT + 22)
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
    ($self:ident . $method:ident ($g:ident, $obj:expr, $target:ident, $dist:ident, $wrap:ident)) => {{
        let no: *mut NearestObjects = &mut $self.nearestObjects;
        unsafe { $self.$method($g, $obj, &mut (*no).$target, &mut (*no).$dist, &mut (*no).$wrap) }
    }};
}

impl CPlayerAI {
    pub fn new() -> Self {
        CPlayerAI {
            pPlayer: PlayerId::NONE,
            // Uninitialized in the C++ constructor; heap memory from `new` reads as 0 in practice.
            iFallDanger: 0,
            nearestObjects: NearestObjects::new(),
            attentionObjects: BTreeMap::new(),
            currentAttentionObject: AttentionObject { iID: -1, iType: 0, iTimer: 0 },
            wall_frames: 0,
            ignored_player: (PlayerId::NONE, 0),
            nav: None,
            nav_step: None,
            style: Style::Balanced,
            nav_age: 0,
            _alias: Aliased::new(),
        }
    }

    pub fn set_player(&mut self, player: PlayerId) {
        self.pPlayer = player;
    }

    pub fn get_nearest_objects(&mut self, g: &mut Game) {
        self.nearestObjects.reset();

        let pPlayer = self.pPlayer;
        let carriedItem = g.carried_item(pPlayer);
        let carriedObj = carried_as_object(carriedItem);
        let fInvincible = g.player(pPlayer).is_invincible() || g.player(pPlayer).is_shielded() || g.player(pPlayer).shyguy;
        let iTeamID: i16 = g.player(pPlayer).teamID;

        let n = g.objectcontainer[1].list().len();
        for i in 0..n {
            let mut obj = g.objectcontainer[1].list()[i];
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

                            nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                        } else if !carriedItem.is_null() {
                            continue;
                        } else {
                            nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                        }
                    } else if movingobject_throwblock == movingtype
                        || (movingobject_throwbox == movingtype && static_cast::<CO_ThrowBox>(obj).has_kill_velocity())
                    {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_pirhanaplant == movingtype {
                        // MO_PirhanaPlant has no own `state`; this is CObject::state.
                        if obj.state > 0 {
                            nearest!(self.distance_to_object_center(g, obj, threat, threatdistance, threatwrap));
                        }
                    } else if movingobject_flag == movingtype {
                        let mut flag = static_cast::<CO_Flag>(obj);

                        if flag.get_in_base() && flag.get_team_id() == iTeamID {
                            continue;
                        }

                        if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_flag {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if movingobject_yoshi == movingtype {
                        let mut yoshi = static_cast::<MO_Yoshi>(obj);

                        if carriedItem.is_null() || carriedItem.get_moving_object_type() != movingobject_egg {
                            continue;
                        }

                        let mut egg = static_cast::<CO_Egg>(carriedObj);

                        if yoshi.get_color() != egg.get_color() {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if movingobject_egg == movingtype {
                        if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_egg {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if movingobject_throwbox == movingtype {
                        if !carriedItem.is_null() {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if movingobject_star == movingtype {
                        if !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_star {
                            continue;
                        }

                        let mut starmode = static_cast_gamemode::<CGM_Star>(g);
                        let mut star = static_cast::<CO_Star>(obj);

                        if star.get_type() == 1 || starmode.isplayerstar(g, pPlayer) {
                            nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                        } else {
                            nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                        }
                    } else if movingobject_coin == movingtype || movingobject_phantokey == movingtype {
                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if movingobject_collectioncard == movingtype {
                        let iNumHeldCards: i32 = g.player(pPlayer).score(g).subscore[0] as i32;
                        let iHeldCards: i32 = g.player(pPlayer).score(g).subscore[1] as i32;

                        // C++ `break` out of the switch ends this object's iteration: `continue` here.
                        if iNumHeldCards == 3 {
                            let iThreeHeldCards = iHeldCards & 63;

                            //If bot is holding all of the same type of card, then ignore all other cards
                            if iThreeHeldCards == 42 || iThreeHeldCards == 21 || iThreeHeldCards == 0 {
                                nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                                continue;
                            }

                            let iTwoHeldCards = iHeldCards & 15;
                            if iTwoHeldCards == 10 || iTwoHeldCards == 5 || iTwoHeldCards == 0 {
                                nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
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
                                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                                        continue;
                                    }
                                }
                            } else if iNumHeldCards == 1 {
                                let iHeldCard = iHeldCards & 3;
                                let iCardValue: i32 = card.get_value() as i32;
                                if card.get_type() == 1 || iHeldCard != iCardValue {
                                    nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                                    continue;
                                }
                            }
                        }

                        if iNumHeldCards == 3 {
                            nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                        } else {
                            nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                        }
                    }
                }
                object_frenzycard => {
                    nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                }

                object_pipe_coin => {
                    let mut coin = static_cast::<OMO_PipeCoin>(obj);

                    if coin.get_color() != 0 {
                        if coin.get_team() == -1 || coin.get_team() == g.player(pPlayer).teamID {
                            nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                        }
                    } else {
                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    }
                }

                object_pipe_bonus => {
                    let mut bonus = static_cast::<OMO_PipeBonus>(obj);

                    if bonus.get_type() != 5 {
                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else {
                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    }
                }

                object_pathhazard | object_orbithazard => {
                    nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                }

                object_phanto => {
                    let mut phanto = static_cast::<OMO_Phanto>(obj);

                    if phanto.get_type() == 2
                        || (!carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_phantokey)
                    {
                        nearest!(self.distance_to_object_center(g, obj, threat, threatdistance, threatwrap));
                    }
                }

                object_flamecannon => {
                    // IO_FlameCannon has no own `state`; this is CObject::state.
                    if obj.state > 0 {
                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    }
                }

                _ => {
                    nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                }
            }
        }

        let n = g.objectcontainer[0].list().len();
        for i in 0..n {
            let mut obj = g.objectcontainer[0].list()[i];
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
                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if (movingobject_fireball == movingtype && static_cast::<MO_Fireball>(obj).iTeamID != iTeamID)
                        || movingobject_poisonpowerup == movingtype
                    {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if (movingobject_goomba == movingtype || movingobject_koopa == movingtype) && obj.get_state() == 1 {
                        nearest!(self.distance_to_object(g, obj, stomp, stompdistance, stompwrap));
                    } else if movingobject_sledgebrother == movingtype {
                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_treasurechest == movingtype {
                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else if movingobject_flagbase == movingtype {
                        let mut flagbase = static_cast::<MO_FlagBase>(obj);

                        if carriedItem.is_null()
                            || carriedItem.get_moving_object_type() != movingobject_flag
                            || flagbase.get_team_id() != iTeamID
                        {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                    } else {
                        continue;
                    }
                }

                object_area => {
                    if static_cast::<OMO_Area>(obj).get_color_id() == g.player(pPlayer).colorID {
                        continue;
                    }

                    nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                }

                object_kingofthehill_area => {
                    nearest!(self.distance_to_object_center(g, obj, goal, goaldistance, goalwrap));
                }

                _ => {
                    nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                }
            }
        }

        let n = g.objectcontainer[2].list().len();
        for i in 0..n {
            let mut obj = g.objectcontainer[2].list()[i];
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
                        nearest!(self.distance_to_object(g, obj, stomp, stompdistance, stompwrap));
                    } else if movingobject_bulletbill == movingtype {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_hammer == movingtype && static_cast::<MO_Hammer>(obj).iTeamID != iTeamID {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_iceblast == movingtype && static_cast::<MO_IceBlast>(obj).iTeamID != iTeamID {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_boomerang == movingtype && static_cast::<MO_Boomerang>(obj).iTeamID != iTeamID {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_bomb == movingtype && static_cast::<CO_Bomb>(obj).iTeamID != iTeamID {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_podobo == movingtype && static_cast::<MO_Podobo>(obj).iTeamID != iTeamID {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_column_threat(g, obj, threat, threatdistance, threatwrap));
                    } else if movingobject_explosion == movingtype && static_cast::<MO_Explosion>(obj).iTeamID != iTeamID {
                        if fInvincible {
                            continue;
                        }

                        nearest!(self.distance_to_object_center(g, obj, threat, threatdistance, threatwrap));
                    }
                }
                object_thwomp => {
                    if fInvincible {
                        continue;
                    }

                    nearest!(self.distance_to_column_threat(g, obj, threat, threatdistance, threatwrap));
                }
                object_bowserfire => {
                    if fInvincible {
                        continue;
                    }

                    nearest!(self.distance_to_object_center(g, obj, threat, threatdistance, threatwrap));
                }
                object_race_goal => {
                    if g.mode().gamemode != game_mode_race {
                        continue;
                    }

                    let mut racegoal = static_cast::<OMO_RaceGoal>(obj);

                    if racegoal.get_goal_id() != static_cast_gamemode::<CGM_Race>(g).get_next_goal(iTeamID) {
                        continue;
                    }

                    nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                }

                _ => {
                    nearest!(self.distance_to_object(g, obj, goal, goaldistance, goalwrap));
                }
            }
        }

        // Without a powerup, a CPU goes for a nearby block that still holds one in the modes won by stomping.
        let stomp_mode = matches!(g.mode().gamemode, game_mode_classic | game_mode_frag | game_mode_timelimit);
        if stomp_mode && g.player(pPlayer).powerup == -1 && carriedItem.is_null() && self.nearestObjects.goal.is_none() {
            let (mut block, mut distance, mut wrap) = (ObjectId::NONE, 160 * 160, false);
            for i in 0..g.noncolcontainer.list().len() {
                let obj = g.noncolcontainer.list()[i];
                let pb = dynamic_cast::<B_PowerupBlock>(obj);
                if pb.is_null() || pb.hidden || obj.get_state() != 0 || self.attentionObjects.contains_key(&obj.network_id()) {
                    continue;
                }
                self.distance_to_object(g, obj, &mut block, &mut distance, &mut wrap);
            }
            if !block.is_none() {
                self.nearestObjects.goal = block;
                self.nearestObjects.goaldistance = distance;
                self.nearestObjects.goalwrap = wrap;
            }
        }

        let gmChicken = dynamic_cast_gamemode::<CGM_Chicken>(g);
        let mut best_weighted = i32::MAX;
        let mut fallback = PlayerId::NONE;

        //Figure out where the other players are
        for iPlayer in 0..g.players.len() {
            if iPlayer == g.player(pPlayer).localID as usize || g.player(g.players[iPlayer]).state != PlayerState::Ready {
                continue;
            }

            //Find players in jail on own team to tag
            if g.mode().gamemode == game_mode_jail {
                if g.player(g.players[iPlayer]).jail.is_active() && g.player(g.players[iPlayer]).teamID == iTeamID {
                    nearest!(self.distance_to_player(g, g.players[iPlayer], teammate, teammatedistance, teammatewrap));
                }
            }

            if g.player(g.players[iPlayer]).teamID == iTeamID || g.player(g.players[iPlayer]).state != PlayerState::Ready {
                continue;
            }

            //If there is a chicken, only focus on stomping him
            if !gmChicken.is_null() && !gmChicken.chicken().is_none() {
                if g.player(gmChicken.chicken()).teamID != iTeamID && gmChicken.chicken() != g.players[iPlayer] {
                    continue;
                }
            }

            let opponent = g.players[iPlayer];
            if opponent == self.ignored_player.0 && self.ignored_player.1 > 0 {
                fallback = opponent;
                continue;
            }
            let (mut target, mut distance, mut wrap) = (PlayerId::default(), i32::MAX, false);
            self.distance_to_player(g, opponent, &mut target, &mut distance, &mut wrap);
            let (num, den) = spread_weight(g, pPlayer, opponent);
            let weighted = (distance as i64 * num / den).min(i32::MAX as i64) as i32;
            if weighted < best_weighted {
                best_weighted = weighted;
                self.nearestObjects.player = opponent;
                self.nearestObjects.playerdistance = distance;
                self.nearestObjects.playerwrap = wrap;
            }
        }

        // The opponent given up on behind a wall is still better than standing with no target.
        if self.nearestObjects.player.is_none() && !fallback.is_none() {
            nearest!(self.distance_to_player(g, fallback, player, playerdistance, playerwrap));
        }
    }

    pub fn distance_to_object(&self, g: &Game, object: Ptr<dyn CObjectTrait>, target: &mut ObjectId, nearest: &mut i32, wrap: &mut bool) {
        let pPlayer = self.pPlayer;

        //Calculate normal screen
        let mut tx: i16 = (object.x() - g.player(pPlayer).ix as i32) as i16;
        let ty: i16 = (object.y() - g.player(pPlayer).iy as i32) as i16;
        let mut fScreenWrap = false;

        //See if it is a shorter distance wrapping around the screen
        if wrap_open(g, g.player(pPlayer).iy as i32) || wrap_open(g, object.y()) {
            if tx as i32 > App::screenWidth / 2 {
                tx = (App::screenWidth - tx as i32) as i16;
                fScreenWrap = true;
            } else if (tx as i32) < -App::screenWidth / 2 {
                tx = (App::screenWidth + tx as i32) as i16;
                fScreenWrap = true;
            }
        }

        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32; // a^2 = b^2 + c^2 :)

        if distance_player_pow2 < *nearest {
            *target = object.id;
            *nearest = distance_player_pow2;
            *wrap = fScreenWrap;
        }
    }

    pub fn distance_to_object_center(&self, g: &Game, object: Ptr<dyn CObjectTrait>, target: &mut ObjectId, nearest: &mut i32, wrap: &mut bool) {
        let pPlayer = self.pPlayer;

        //Calculate normal screen
        let mut tx: i16 = (object.x() + (object.collision_rect_w() as i32 / 2) - g.player(pPlayer).ix as i32 - HALFPW) as i16;
        let ty: i16 = (object.y() + (object.collision_rect_h() as i32 / 2) - g.player(pPlayer).iy as i32 - HALFPH) as i16;
        let mut fScreenWrap = false;

        if wrap_open(g, g.player(pPlayer).iy as i32) || wrap_open(g, object.y()) {
            if tx as i32 > App::screenWidth / 2 {
                tx = (App::screenWidth - tx as i32) as i16;
                fScreenWrap = true;
            } else if (tx as i32) < -App::screenWidth / 2 {
                tx = (App::screenWidth + tx as i32) as i16;
                fScreenWrap = true;
            }
        }

        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32;

        if distance_player_pow2 < *nearest {
            *target = object.id;
            *nearest = distance_player_pow2;
            *wrap = fScreenWrap;
        }
    }

    /// `distance_to_object_center` with the vertical distance counted at a quarter, for threats that fall or rise
    /// through a whole column (thwomps, podobos).
    pub fn distance_to_column_threat(&self, g: &Game, object: Ptr<dyn CObjectTrait>, target: &mut ObjectId, nearest: &mut i32, wrap: &mut bool) {
        let mut distance = i32::MAX;
        let mut found = ObjectId::NONE;
        let mut found_wrap = false;
        self.distance_to_object_center(g, object, &mut found, &mut distance, &mut found_wrap);
        let ty = object.y() + object.collision_rect_h() as i32 / 2 - g.player(self.pPlayer).iy as i32 - HALFPH;
        let squeezed = distance - ty * ty + (ty / 4) * (ty / 4);
        if squeezed < *nearest {
            *target = object.id;
            *nearest = squeezed;
            *wrap = found_wrap;
        }
    }

    pub fn distance_to_player(&self, g: &Game, player: PlayerId, target: &mut PlayerId, nearest: &mut i32, wrap: &mut bool) {
        let pPlayer = self.pPlayer;

        //Calculate normal screen
        let mut tx: i16 = (g.player(player).ix as i32 - g.player(pPlayer).ix as i32) as i16;
        let ty: i16 = (g.player(player).iy as i32 - g.player(pPlayer).iy as i32) as i16;
        let mut fScreenWrap = false;

        if wrap_open(g, g.player(pPlayer).iy as i32) || wrap_open(g, g.player(player).iy as i32) {
            if tx as i32 > App::screenWidth / 2 {
                tx = (App::screenWidth - tx as i32) as i16;
                fScreenWrap = true;
            } else if (tx as i32) < -App::screenWidth / 2 {
                tx = (App::screenWidth + tx as i32) as i16;
                fScreenWrap = true;
            }
        }

        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32;

        if distance_player_pow2 < *nearest {
            *target = player;
            *nearest = distance_player_pow2;
            *wrap = fScreenWrap;
        }
    }
}

/// Whether a player whose top is at `y` can walk off one side of the screen: no edge column is solid in its rows.
fn wrap_open(g: &Game, y: i32) -> bool {
    let first = (y / TILESIZE).max(0);
    let last = ((y + PH - 1) / TILESIZE).min(MAPHEIGHT - 1);
    (first..=last).all(|row| (g.g_map.map(0, row) | g.g_map.map(MAPWIDTH - 1, row)) & tile_flag_solid as i32 == 0)
}

/// How much farther `opponent` counts when `me` picks whom to chase: each other CPU already chasing it doubles,
/// triples and so on its distance, the score leader counts at three quarters, and a flag carrier or the holder of a
/// star worth keeping at half.
fn spread_weight(g: &mut Game, me: PlayerId, opponent: PlayerId) -> (i64, i64) {
    let mut chasers = 0;
    let mut leader = true;
    let score = g.player(opponent).score(g).score;
    for i in 0..g.players.len() {
        let other = g.players[i];
        if other == opponent {
            continue;
        }
        if g.player(other).score(g).score >= score {
            leader = false;
        }
        if other == me {
            continue;
        }
        if let Some(ai) = g.player(other).pPlayerAI.as_mut() {
            if ai.ai().nearestObjects.player == opponent {
                chasers += 1;
            }
        }
    }
    let carried = g.carried_item(opponent);
    let carrier = !carried.is_null()
        && (carried.get_moving_object_type() == movingobject_flag || (carried.get_moving_object_type() == movingobject_star && star_type(carried) == 1));
    (if leader { 3 } else { 4 } * (1 + chasers), if carrier { 8 } else { 4 })
}

/// Modes won by collecting or holding a goal, where a nearby player should not pull the AI off it.
fn objective_mode(mode: GameModeType) -> bool {
    matches!(
        mode,
        game_mode_coins | game_mode_eggs | game_mode_ctf | game_mode_domination | game_mode_koth | game_mode_race | game_mode_collection
    )
}

/// Where the AI is headed for `actionType`, as the top left of a player standing there.
fn path_target(g: &mut Game, this: &CPlayerAI, actionType: i16) -> Option<(i32, i32)> {
    let no = &this.nearestObjects;
    let player = match actionType {
        0 => no.player,
        3 => no.teammate,
        1 if !no.goal.is_none() => {
            let goal = ai_target(g, no.goal);
            return Some((goal.x() + goal.collision_rect_w() as i32 / 2 - HALFPW, goal.y() + goal.collision_rect_h() as i32 - PH));
        }
        _ => return None,
    };
    (!player.is_none()).then(|| (g.player(player).ix as i32, g.player(player).iy as i32))
}

/// Whether going from tile column `from` to `to` heads right, the short way where the move crosses the screen edge.
fn heads_right(from: i32, to: i32) -> bool {
    if (to - from).abs() > MAPWIDTH / 2 {
        to < from
    } else {
        to > from
    }
}

/// Steers along the navigation graph when the straight line the action chose is not the path, and gives up on a
/// target the graph cannot reach.
fn follow_path(g: &mut Game, this: &mut CPlayerAI, actionType: i16, mut keys: Ptr<COutputControl>) {
    let planned = match g.game_values.cpudifficulty {
        0 => false,
        1 => actionType == 1,
        _ => true,
    };
    let Some(target) = path_target(g, this, actionType).filter(|_| planned) else {
        this.nav_step = None;
        return;
    };
    this.nav_age = this.nav_age.saturating_add(1);
    if this.nav.is_none() || (!g.g_map.platforms.is_empty() && this.nav_age >= 8) {
        this.nav = Some(NavGraph::build(g));
        this.nav_age = 0;
    }
    let pPlayer = this.pPlayer;
    let (ix, iy) = (g.player(pPlayer).ix as i32, g.player(pPlayer).iy as i32);
    let mut fleeing = false;
    if actionType == 0 {
        let toward_left = (target.0 > ix && this.nearestObjects.playerwrap) || (target.0 < ix && !this.nearestObjects.playerwrap);
        let (left, right) = (keys.game_left().fDown, keys.game_right().fDown);
        fleeing = (toward_left && right && !left) || (!toward_left && left && !right);
    }
    let nav = this.nav.as_ref().unwrap();
    let Some(me) = nav.cell_of(ix, iy) else { return };
    if g.player(pPlayer).inair {
        // A note block keeps a CPU in the air, so a step whose column it has reached ends here.
        if this.nav_step.is_some_and(|(x, _)| x == me.0) {
            this.nav_step = None;
        }
        if let Some((x, how)) = this.nav_step {
            let right = heads_right(me.0, x);
            keys.game_left_mut().fDown = me.0 != x && !right;
            keys.game_right_mut().fDown = me.0 != x && right;
            if how == Move::Jump && g.player(pPlayer).vely < 0.0 {
                keys.game_jump_mut().fDown = true;
            }
        }
        return;
    }
    this.nav_step = None;
    let Some(to) = nav.cell_of(target.0, target.1) else { return };
    if fleeing {
        if let Some((x, _, how)) = nav.flee(me, to) {
            let right = heads_right(me.0, x);
            keys.game_left_mut().fDown = !right;
            keys.game_right_mut().fDown = right;
            if how == Move::Jump {
                keys.game_jump_mut().fDown = true;
            }
            this.nav_step = Some((x, how));
        } else {
            // Cornered: jump over the chaser rather than wait against the wall.
            let right = heads_right(me.0, to.0);
            keys.game_left_mut().fDown = !right;
            keys.game_right_mut().fDown = right;
            keys.game_jump_mut().fDown = true;
            this.nav_step = Some((to.0, Move::Jump));
        }
        return;
    }
    if (to.0 - me.0).abs() <= 1 && (to.1 - me.1).abs() <= 1 {
        return;
    }
    // Chasing a player, or a goal a player carries, the straight line wins fights; the path only takes over when
    // that line is blocked.
    let carried = actionType == 1 && {
        let goal = g.carried(this.nearestObjects.goal);
        !goal.is_null() && !goal.owner.is_none()
    };
    if (actionType != 1 || carried) && this.wall_frames == 0 && (to.1 - me.1).abs() < 3 {
        return;
    }
    match nav.route(me, to) {
        Route::Here => {}
        // The graph has no moving platforms or springs, so an unreachable target keeps the straight-line steering.
        Route::Unreachable => {}
        Route::Step { how: Move::Warp { dir, x }, .. } => {
            let centre = ix + HALFPW;
            let aligned = (centre - x).abs() <= 4;
            let toward_right = x > centre;
            keys.game_left_mut().fDown = false;
            keys.game_right_mut().fDown = false;
            keys.game_jump_mut().fDown = false;
            keys.game_down_mut().fDown = false;
            match dir {
                WARP_LEFT => keys.game_left_mut().fDown = true,
                WARP_RIGHT => keys.game_right_mut().fDown = true,
                _ if !aligned => {
                    keys.game_left_mut().fDown = !toward_right;
                    keys.game_right_mut().fDown = toward_right;
                }
                WARP_DOWN => keys.game_down_mut().fDown = true,
                _ => keys.game_jump_mut().fDown = true,
            }
            this.nav_step = Some((me.0, Move::Warp { dir, x }));
        }
        Route::Step { x, y, how } => {
            let right = heads_right(me.0, x);
            let chosen = if right { keys.game_right().fDown } else { keys.game_left().fDown };
            if how == Move::Walk && y == me.1 && chosen {
                return;
            }
            keys.game_left_mut().fDown = !right;
            keys.game_right_mut().fDown = right;
            match how {
                Move::Jump => keys.game_jump_mut().fDown = true,
                Move::Drop => {
                    keys.game_left_mut().fDown = false;
                    keys.game_right_mut().fDown = false;
                    keys.game_jump_mut().fDown = false;
                    keys.game_down_mut().fDown = true;
                }
                Move::Walk | Move::Fall | Move::Warp { .. } => {}
            }
            this.nav_step = Some((x, how));
        }
    }
}

/// How long the AI ignores a goal or an opponent behind a wall it cannot jump.
const UNREACHABLE_FRAMES: i16 = ticks(90) as i16;
/// Jumping at a wall this long without getting past it means the wall cannot be jumped.
const WALL_GIVE_UP_FRAMES: i16 = ticks(45) as i16;

/// Whether jumping now brings `me` down onto `target`'s head: `me` stands on the ground, `target` can be stomped and
/// is within a jump's height, and the horizontal gap equals the distance `me` closes on it while the jump carries its
/// feet up to `target`'s head and back down onto it.
fn stomp_lead_jump(g: &mut Game, me: PlayerId, target: PlayerId, wrap: bool) -> bool {
    let (p, t) = (g.player(me), g.player(target));
    if p.inair || t.is_invincible() || t.bobomb || p.is_invincible() {
        return false;
    }
    let mut dx = t.ix as i32 - p.ix as i32;
    if wrap {
        dx -= dx.signum() * App::screenWidth;
    }
    let closing = (p.velx - t.velx) * dx.signum() as f32;
    if closing <= 0.0 {
        return false;
    }
    let jump = if p.velx.abs() > VELMOVING { VELTURBOJUMP } else { VELJUMP };
    let rise = (p.iy as i32 + PH - t.iy as i32) as f32;
    let discriminant = jump * jump - 2.0 * GRAVITATION * rise;
    if rise < -(TILESIZE as f32) || discriminant < 0.0 {
        return false;
    }
    let airtime = (jump + discriminant.sqrt()) / GRAVITATION;
    let lead = closing * airtime;
    // The AI thinks every fourth tick, so the window spans what it closes in that time.
    let window = closing * 4.0;
    (dx.abs() as f32 - lead).abs() <= window.max(PW as f32 / 2.0)
}

/// The way to step (true for right) to get out from under an opponent falling onto `me`, if one is.
fn dodge_stomp(g: &mut Game, me: PlayerId) -> Option<bool> {
    let p = g.player(me);
    if p.is_invincible() || p.shyguy || p.bobomb {
        return None;
    }
    let (mx, my, team) = (p.ix as i32, p.iy as i32, p.teamID);
    for i in 0..g.players.len() {
        let o = g.player(g.players[i]);
        if g.players[i] == me || o.teamID == team || !o.isready() || o.vely <= 0.0 {
            continue;
        }
        let mut dx = o.ix as i32 - mx;
        if dx.abs() > App::screenWidth / 2 {
            dx -= dx.signum() * App::screenWidth;
        }
        let gap = my - (o.iy as i32 + PH);
        if dx.abs() < PW + 8 && (0..96).contains(&gap) {
            return Some(dx < 0 || (dx == 0 && p.is_facing_right(g)));
        }
    }
    None
}

fn star_type(carried: Ptr<dyn MO_CarriedObjectTrait>) -> i16 {
    static_cast::<CO_Star>(carried_as_object(carried)).get_type()
}

/// Whether `player` carries a star of `kind` (0 the one to pass on, 1 the one to keep).
fn holds_star(g: &mut Game, player: PlayerId, kind: i16) -> bool {
    let carried = g.carried_item(player);
    !carried.is_null() && carried.get_moving_object_type() == movingobject_star && star_type(carried) == kind
}

/// Whether an opponent stands where `me`'s weapon would reach it: in front within two rows for a fireball,
/// boomerang or ice blast, in front and not far below for a thrown hammer or bomb, and within reach of a spin.
fn weapon_lined_up(g: &mut Game, me: PlayerId) -> bool {
    let p = g.player(me);
    if !(1..=7).contains(&p.powerup) {
        return false;
    }
    let facing_right = p.is_facing_right(g);
    (0..g.players.len()).any(|i| {
        let t = g.player(g.players[i]);
        if g.players[i] == me || t.teamID == p.teamID || t.is_invincible() || !t.isready() {
            return false;
        }
        let mut dx = t.ix as i32 - p.ix as i32;
        if dx.abs() > App::screenWidth / 2 {
            dx -= dx.signum() * App::screenWidth;
        }
        let dy = t.iy as i32 - p.iy as i32;
        let ahead = if facing_right { dx } else { -dx };
        match p.powerup {
            1 | 4 | 5 => ahead > 0 && ahead < 240 && dy.abs() < 2 * TILESIZE,
            2 | 6 => ahead > 0 && ahead < 200 && dy > -3 * TILESIZE && dy < TILESIZE,
            3 | 7 => dx.abs() < PW * 2 && dy.abs() < PH,
            _ => false,
        }
    })
}

fn solid_at(g: &mut Game, tx: i32, ty: i32) -> bool {
    g.g_map.map(tx, ty) & tile_flag_solid as i32 != 0 || !g.map_block(tx as i16, ty as i16).is_null()
}

/// The tile column of a solid tile or block touching the side of the player at (`x`, `y`) that it is walking toward.
fn wall_ahead(g: &mut Game, x: i32, y: i32, left: bool, right: bool) -> Option<i32> {
    if left == right {
        return None;
    }
    let mut column = if right { x + PW } else { x - 1 };
    if column < 0 || column >= App::screenWidth {
        if !wrap_open(g, y) {
            return None;
        }
        column = column.rem_euclid(App::screenWidth);
    }
    let tx = column / TILESIZE;
    let mut rows = (y / TILESIZE).max(0)..=((y + PH - 1) / TILESIZE).min(MAPHEIGHT - 1);
    rows.any(|ty| solid_at(g, tx, ty)).then_some(tx)
}

/// Whether the wall in column `tx` rises four tiles above the head of a player whose top is at `y`, beyond a jump.
/// A wall that reaches the top row counts as going on above the map.
fn wall_too_tall(g: &mut Game, tx: i32, y: i32) -> bool {
    let top = y.div_euclid(TILESIZE);
    top <= 0 || ((top - 4).max(0)..top).all(|ty| solid_at(g, tx, ty))
}

//Setup AI so that it can ignore or pay attention to some objects
pub fn cplayer_ai_init(g: &mut Game, this: &mut CPlayerAI) {
    this.style = match RANDOM_INT(g, 4) {
        1 => Style::Aggressive,
        2 => Style::Objective,
        3 => Style::Cautious,
        _ => Style::Balanced,
    };
    //Scan yoshi's egg mode objects to make sure that we ignore eggs without matching yoshis
    if g.mode().gamemode == game_mode_eggs {
        let mut fYoshi: [bool; 4] = [false, false, false, false];

        //Scan Yoshis to see which ones are present
        let n = g.objectcontainer[1].list().len();
        for i in 0..n {
            let obj = g.objectcontainer[1].list()[i];
            let mut yoshi = dynamic_cast::<MO_Yoshi>(obj);
            if !yoshi.is_null() {
                fYoshi[yoshi.get_color() as usize] = true;
            }
        }

        //Now scan eggs and ignore any egg that doesn't have a yoshi
        let n = g.objectcontainer[1].list().len();
        for i in 0..n {
            let obj = g.objectcontainer[1].list()[i];
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

pub fn cplayer_ai_think(g: &mut Game, this: &mut CPlayerAI, playerKeys: Ptr<COutputControl>) {
    let mut gp = g.ptr();
    let mut playerKeys = playerKeys;
    let mut pPlayer = this.pPlayer;

    unsafe {
        let iDecisionPercentage: [i16; 5] = [25, 35, 50, 75, 100];

        //Clear out the old input settings
        playerKeys.game_left_mut().fDown = false;
        playerKeys.game_right_mut().fDown = false;

        // A frozen player breaks out sooner for each press of left.
        if g.player(pPlayer).frozen && g.game_values.cpudifficulty >= 1 {
            playerKeys.game_left_mut().fPressed = true;
            return;
        }

        //A percentage of the time the cpu will do nothing based on the difficulty level
        if RandomNumberGenerator::generator(gp.get()).get_boolean_threshold(100, iDecisionPercentage[g.game_values.cpudifficulty as usize] as i32)
            && g.player(pPlayer).isready()
        {
            return;
        }

        playerKeys.game_jump_mut().fDown = false;
        playerKeys.game_down_mut().fDown = false;
        playerKeys.game_turbo_mut().fDown = false;
        playerKeys.game_powerup_mut().fDown = false;

        if g.player(pPlayer).isdead() || g.player(pPlayer).isspawning() {
            return;
        }

        let mut gmChicken = dynamic_cast_gamemode::<CGM_Chicken>(g);
        let mut gmTag = dynamic_cast_gamemode::<CGM_Tag>(g);

        /***************************************************
         * 1. Figure out what objects are nearest to us
         ***************************************************/
        this.get_nearest_objects(g);

        let iTenSeconds: i16 = ticks(15 * iDecisionPercentage[g.game_values.cpudifficulty as usize] as i32 / 10) as i16;

        //If there is a goal, then make sure we aren't paying attention to it for too long
        if !this.nearestObjects.goal.is_none() {
            if this.currentAttentionObject.iID == ai_target(g, this.nearestObjects.goal).network_id() {
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
                this.currentAttentionObject.iID = ai_target(g, this.nearestObjects.goal).network_id();
                this.currentAttentionObject.iTimer = 0;
            }
        } else {
            this.currentAttentionObject.iID = -1;
        }

        if this.ignored_player.1 > 0 {
            this.ignored_player.1 -= 1;
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

        let iStoredPowerup: i16 = g.game_values.gamepowerups[g.player(pPlayer).globalID as usize];
        let mut carriedItem = g.carried_item(pPlayer);
        let carriedObj = carried_as_object(carriedItem);
        let ix: i16 = g.player(pPlayer).ix;
        let iy: i16 = g.player(pPlayer).iy;
        let iTeamID: i16 = g.player(pPlayer).teamID;

        let ix32 = ix as i32;
        let iy32 = iy as i32;

        /***************************************************
         * 2. Figure out priority of actions
         ***************************************************/

        let actionType: i16;
        let no = &this.nearestObjects;
        let objective = !no.goal.is_none() && objective_mode(g.mode().gamemode);
        let carrying_flag = !g.carried_item(pPlayer).is_null() && g.carried_item(pPlayer).get_moving_object_type() == movingobject_flag;
        let ranges = style_ranges(this.style);
        let (stomp_range, attack_range) = if carrying_flag && !no.goal.is_none() {
            (0, 0)
        } else if objective {
            ranges.objective
        } else {
            ranges.open
        };
        if !no.threat.is_none() && no.threatdistance < ranges.threat && !g.player(pPlayer).is_invincible() {
            actionType = 2;
        } else if !no.stomp.is_none() && no.stompdistance < stomp_range {
            actionType = 4;
        } else if !no.goal.is_none() && !no.teammate.is_none() {
            if no.goaldistance < no.teammatedistance {
                actionType = 1;
            } else {
                actionType = 3;
            }
        } else if !no.teammate.is_none() {
            actionType = 3;
        } else if !no.goal.is_none() {
            if no.playerdistance < attack_range {
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
        if ((actionType != 0) || !no.player.is_none())
            && (actionType != 1 || !no.goal.is_none())
            && ((actionType != 2) || !no.threat.is_none())
            && ((actionType != 3) || !no.teammate.is_none())
            && ((actionType != 4) || !no.stomp.is_none())
            && (this.iFallDanger == 0)
        {
            //Use turbo
            if g.player(pPlayer).bobomb {
                if no.playerdistance <= 4096 || no.stompdistance <= 4096 || no.threatdistance <= 4096 {
                    playerKeys.game_turbo_mut().fDown = true;
                }
            } else {
                if g.player(pPlayer).powerup == -1 || !RandomNumberGenerator::generator(gp.get()).get_boolean_scale(20) {
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

                if (g.player(player).ix > ix && no.playerwrap) || (g.player(player).ix < ix && !no.playerwrap) {
                    moveToward = &mut playerKeys.game_left_mut().fDown;
                    moveAway = &mut playerKeys.game_right_mut().fDown;
                } else {
                    moveToward = &mut playerKeys.game_right_mut().fDown;
                    moveAway = &mut playerKeys.game_left_mut().fDown;
                }

                let pix = g.player(player).ix as i32;
                let piy = g.player(player).iy as i32;

                //Move player relative to opponent when we are playing star mode and we are holding a star
                if g.mode().gamemode == game_mode_star && !carriedItem.is_null() && carriedItem.get_moving_object_type() == movingobject_star {
                    if static_cast::<CO_Star>(carriedObj).get_type() == 1 {
                        *moveAway = true;
                    } else {
                        *moveToward = true;

                        //If player is close
                        if piy <= iy32 && piy > iy32 - 60 && pix - ix32 < 90 && pix - ix32 > -90 {
                            //And we are facing toward that player, throw the star
                            if (pix > ix32 && g.player(pPlayer).is_facing_right(g)) || ((pix < ix32) && !g.player(pPlayer).is_facing_right(g)) {
                                g.player(pPlayer).throw_star = ticks(30) as i16;
                                playerKeys.game_turbo_mut().fDown = false;
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
                            if (pix > ix32 && g.player(pPlayer).is_facing_right(g)) || ((pix < ix32) && !g.player(pPlayer).is_facing_right(g)) {
                                playerKeys.game_turbo_mut().fDown = false;
                            }
                        }
                    }
                } else if (!gmTag.is_null() && gmTag.tagged() == player)
                    || holds_star(g, player, 0)
                    || g.player(player).is_invincible()
                    || g.player(player).shyguy
                    || (!gmChicken.is_null() && gmChicken.chicken() == pPlayer)
                {
                    *moveAway = true;
                } else if g.player(pPlayer).is_invincible() || g.player(pPlayer).shyguy || g.player(pPlayer).bobomb || (!gmTag.is_null() && gmTag.tagged() == pPlayer) {
                    *moveToward = true;
                } else if piy >= iy32 && !g.player(player).is_invincible() && !g.player(player).bobomb {
                    *moveToward = true;
                } else {
                    if no.playerdistance < ranges.back_off {
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
                    pix - ix32 > -45 && RANDOM_BOOL(gp.get())
                {
                    //or if player is high
                    if !g.player(pPlayer).inair {
                        playerKeys.game_down_mut().fDown = true;
                    }

                    if g.player(pPlayer).inair && !g.player(pPlayer).superstomp.is_in_super_stomp_state() {
                        //If the player has the tanooki or shoe, then try to super stomp on them
                        if g.player(pPlayer).tanookisuit.is_on() || g.player(pPlayer).kuriboshoe.is_on() {
                            playerKeys.game_down_mut().fDown = true;
                            playerKeys.game_jump_mut().fPressed = true;
                            if g.player(pPlayer).tanookisuit.is_on() {
                                playerKeys.game_turbo_mut().fPressed = true;
                                g.player(pPlayer).lockfire = false;
                            }
                        }
                    }
                }
                if g.game_values.cpudifficulty >= 2 && !playerKeys.game_jump().fDown && stomp_lead_jump(g, pPlayer, player, no.playerwrap) {
                    playerKeys.game_jump_mut().fDown = true;
                }
            } else if actionType == 1 {
                //Go for goal
                let mut goal = ai_target(g, no.goal);

                if (goal.x() > ix32 && no.goalwrap) || (goal.x() < ix32 && !no.goalwrap) {
                    playerKeys.game_left_mut().fDown = true;
                } else {
                    playerKeys.game_right_mut().fDown = true;
                }

                if goal.y() <= iy32 && goal.x() - ix32 < 45 && goal.x() - ix32 > -45 {
                    playerKeys.game_jump_mut().fDown = true;
                } else if goal.y() > iy32 && goal.x() - ix32 < 45 && goal.x() - ix32 > -45 {
                    if !g.player(pPlayer).inair {
                        playerKeys.game_down_mut().fDown = true;
                    }
                }

                if !dynamic_cast::<CO_Egg>(goal).is_null() {
                    playerKeys.game_turbo_mut().fDown = true;
                } else if !dynamic_cast::<CO_Star>(goal).is_null() && g.player(pPlayer).throw_star == 0 {
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
                let mut threat = ai_target(g, no.threat);
                let column_threat = threat.get_object_type() == object_thwomp
                    || (threat.get_object_type() == object_moving && moving_object_type(threat) == movingobject_podobo);

                //If threat, always use turbo
                playerKeys.game_turbo_mut().fDown = true;

                let threat_x = if column_threat { threat.x() + threat.collision_rect_w() as i32 / 2 - HALFPW } else { threat.x() };
                if (threat_x > ix32 && no.threatwrap) || (threat_x < ix32 && !no.threatwrap) {
                    playerKeys.game_right_mut().fDown = true;
                } else {
                    playerKeys.game_left_mut().fDown = true;
                }

                if column_threat {
                    // Ducking under a thwomp or jumping into a podobo only keeps the player in its column.
                    if wall_ahead(g, ix32, iy32, playerKeys.game_left().fDown, playerKeys.game_right().fDown).is_some() {
                        let right = playerKeys.game_right().fDown;
                        playerKeys.game_left_mut().fDown = right;
                        playerKeys.game_right_mut().fDown = !right;
                    }
                } else if threat.y() <= iy32 && threat.x() - ix32 < 60 && threat.x() - ix32 > -60 {
                    if !g.player(pPlayer).inair {
                        playerKeys.game_down_mut().fDown = true;
                    }
                } else if threat.y() > iy32 && threat.x() - ix32 < 60 && threat.x() - ix32 > -60 {
                    playerKeys.game_jump_mut().fDown = true;
                }
            } else if actionType == 3 {
                //Tag teammate
                let teammate = no.teammate;
                let tix = g.player(teammate).ix as i32;
                let tiy = g.player(teammate).iy as i32;

                if (tix > ix32 && no.teammatewrap) || (tix < ix32 && !no.teammatewrap) {
                    playerKeys.game_left_mut().fDown = true;
                } else {
                    playerKeys.game_right_mut().fDown = true;
                }

                if tiy <= iy32 && tix - ix32 < 45 && tix - ix32 > -45 {
                    playerKeys.game_jump_mut().fDown = true;
                } else if tiy > iy32 && tix - ix32 < 45 && tix - ix32 > -45 {
                    if !g.player(pPlayer).inair {
                        playerKeys.game_down_mut().fDown = true;
                    }
                }
            } else if actionType == 4 {
                //Stomp something (goomba, koopa, cheepcheep)
                let stomp = ai_target(g, no.stomp);
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
                    || g.player(pPlayer).shyguy
                    || g.player(pPlayer).is_invincible()
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
                    if !g.player(pPlayer).inair {
                        playerKeys.game_down_mut().fDown = true;
                    }

                    if g.player(pPlayer).inair && !g.player(pPlayer).superstomp.is_in_super_stomp_state() {
                        //If the player has the tanooki or shoe, then try to super stomp on them
                        if g.player(pPlayer).tanookisuit.is_on() || g.player(pPlayer).kuriboshoe.is_on() {
                            playerKeys.game_down_mut().fDown = true;
                            playerKeys.game_jump_mut().fPressed = true;
                            if g.player(pPlayer).tanookisuit.is_on() {
                                playerKeys.game_turbo_mut().fPressed = true;
                                g.player(pPlayer).lockfire = false;
                            }
                        }
                    }
                }
            }
        }

        //Jump if trying to move left/right and x velocity is zero
        if (playerKeys.game_left().fDown || playerKeys.game_right().fDown) && g.player(pPlayer).velx == 0.0f32 {
            playerKeys.game_jump_mut().fDown = true;
        }

        //Pick up throwable blocks from below
        if playerKeys.game_turbo().fDown && carriedItem.is_null() && !g.player(pPlayer).inair {
            playerKeys.game_turbo_mut().fPressed = true;
        }

        //Stay inside tanooki statue
        if g.player(pPlayer).tanookisuit.is_on() && g.player(pPlayer).tanookisuit.is_statue() {
            playerKeys.game_down_mut().fDown = true;
        }

        //"Star Mode" specific stuff
        //Drop the star if we're not it
        if g.mode().gamemode == game_mode_star {
            let mut starmode = static_cast_gamemode::<CGM_Star>(g);

            if !carriedItem.is_null()
                && carriedItem.get_moving_object_type() == movingobject_star
                && static_cast::<CO_Star>(carriedObj).get_type() == 0
                && !starmode.isplayerstar(g, pPlayer)
            {
                playerKeys.game_turbo_mut().fDown = false;
            } else if starmode.isplayerstar(g, pPlayer) && g.player(pPlayer).throw_star == 0 {
                playerKeys.game_turbo_mut().fDown = true;
            }
        }

        if !carriedItem.is_null() {
            let carriedobjecttype = carriedItem.get_moving_object_type();
            let no = &this.nearestObjects;

            //"Phanto Mode" specific stuff
            if g.mode().gamemode == game_mode_chase {
                //Ignore the key if a phanto is really close
                if !no.threat.is_none()
                    && ai_target(g, no.threat).get_object_type() == object_phanto
                    && no.threatdistance < 4096
                    && !g.player(pPlayer).is_invincible()
                {
                    playerKeys.game_turbo_mut().fDown = false;

                    //Ignore the key for a little while
                    let carriedID = carriedItem.network_id();
                    if let Some(ao) = this.attentionObjects.get_mut(&carriedID) {
                        ao.iType = 1;
                        ao.iTimer = ticks(iDecisionPercentage[g.game_values.cpudifficulty as usize] as i32 / 3) as i16;
                    } else {
                        let ao = Box::new(AttentionObject {
                            iID: carriedID,
                            iType: 1,
                            iTimer: ticks(iDecisionPercentage[g.game_values.cpudifficulty as usize] as i32 / 3) as i16,
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
                if bomb.iTeamID != g.player(pPlayer).teamID {
                    playerKeys.game_turbo_mut().fDown = false;
                }
            } else if carriedobjecttype == movingobject_carried || carriedobjecttype == movingobject_throwbox {
                //Drop springs,spikes,shoes
                playerKeys.game_turbo_mut().fDown = false;
            }
        }

        follow_path(g, this, actionType, playerKeys);

        // Turbo fires once per press, so with a target lined up a CPU that already fired lets go to fire again.
        if g.game_values.cpudifficulty >= 1 && carriedItem.is_null() && weapon_lined_up(g, pPlayer) {
            playerKeys.game_turbo_mut().fDown = !g.player(pPlayer).lockfire;
        }

        if g.game_values.cpudifficulty >= 2 {
            if let Some(right) = dodge_stomp(g, pPlayer) {
                playerKeys.game_left_mut().fDown = !right;
                playerKeys.game_right_mut().fDown = right;
            }
        }

        // Down with turbo on the ground turns the tanooki suit to stone, and holding down keeps it there; that only
        // helps against a threat.
        if actionType != 2 && g.player(pPlayer).tanookisuit.is_on() && !g.player(pPlayer).inair && !g.player(pPlayer).superstomp.is_stomping() {
            if !g.player(pPlayer).tanookisuit.not_statue() {
                playerKeys.game_down_mut().fDown = false;
            } else if playerKeys.game_down().fDown {
                playerKeys.game_turbo_mut().fDown = false;
            }
        }

        if let Some(column) = wall_ahead(g, ix32, iy32, playerKeys.game_left().fDown, playerKeys.game_right().fDown) {
            playerKeys.game_jump_mut().fDown = true;
            this.wall_frames += 1;
            if this.wall_frames >= WALL_GIVE_UP_FRAMES || wall_too_tall(g, column, iy32) {
                this.wall_frames = 0;
                if actionType == 1 {
                    let id = ai_target(g, this.nearestObjects.goal).network_id();
                    this.attentionObjects.insert(id, Box::new(AttentionObject { iID: id, iType: 1, iTimer: UNREACHABLE_FRAMES }));
                } else if actionType == 0 {
                    this.ignored_player = (this.nearestObjects.player, UNREACHABLE_FRAMES);
                }
            }
        } else {
            this.wall_frames = 0;
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
            let ttLeftTile: i32 = ai_map(g, iDeathX1 as i32, iDeathY as i32);
            let ttRightTile: i32 = ai_map(g, iDeathX2 as i32, iDeathY as i32);

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
                || ai_block_nonnull(g, iDeathX1, iDeathY)
                || (ttRightTile & tile_flag_solid) != 0
                || (ttRightTile & tile_flag_solid_on_top) != 0
                || ai_block_nonnull(g, iDeathX2, iDeathY)
            {
                this.iFallDanger = 0;
                break 'ExitDeathCheck;
            }

            //Look through all platforms and see if we are hitting solid or death tiles in them
            let mut iPlatform: i16 = 0;
            while (iPlatform as usize) < g.g_map.platforms.len() {
                let lefttile: i32 = g.g_map.platform(g.g_map.platforms[iPlatform as usize]).get_tile_type_from_coord(ix, ((iDeathY as i32) << 5) as i16);
                let righttile: i32 =
                    g.g_map.platform(g.g_map.platforms[iPlatform as usize]).get_tile_type_from_coord((ix32 + PW) as i16, ((iDeathY as i32) << 5) as i16);

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
            let ttLeftTile: i32 = ai_map(g, iDeathX1 as i32, iDeathY as i32);
            let ttRightTile: i32 = ai_map(g, iDeathX2 as i32, iDeathY as i32);

            if heightlimit == 2
                && ((ttLeftTile & tile_flag_solid) != 0 || (ttRightTile & tile_flag_solid) != 0)
                && !ai_checkforwarp_up(g, iDeathX1, iDeathX2, iDeathY)
            {
                //Avoid jumping wildly in 1 tile high gaps
                playerKeys.game_jump_mut().fDown = false;
            }

            if ((ttLeftTile & tile_flag_solid) != 0 && (ttLeftTile & tile_flag_death_on_bottom) == 0)
                || ((ttRightTile & tile_flag_solid) != 0 && (ttRightTile & tile_flag_death_on_bottom) == 0)
                || ai_block_nonnull(g, iDeathX1, iDeathY)
                || ai_block_nonnull(g, iDeathX2, iDeathY)
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
                    | PowerupType::PWings => return g.player(pPlayer).powerup == -1,
                    PowerupType::Star => return !g.player(pPlayer).is_invincible(),
                    PowerupType::Bobomb => return !g.player(pPlayer).bobomb,
                    PowerupType::ShellGreen | PowerupType::ShellRed | PowerupType::ShellSpiny | PowerupType::ShellBuzzy => {
                        return carriedItem.is_null()
                    }
                    PowerupType::Tanooki => return !g.player(pPlayer).tanookisuit.is_on(),
                    PowerupType::JailKey => return g.player(pPlayer).jail.is_active(),
                    PowerupType::MysteryMushroom => {
                        //See if another player has a powerup
                        for iPlayer in 0..g.players.len() {
                            if iPlayer == g.player(pPlayer).localID as usize || g.player(g.players[iPlayer]).teamID == iTeamID {
                                continue;
                            }
                            if g.game_values.gamepowerups[g.player(g.players[iPlayer]).globalID as usize] > 0 {
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
#[derive(Clone)]
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
    fn think(&mut self, g: &mut Game, playerKeys: Ptr<COutputControl>) {
        let mut gp = g.ptr();
        let mut playerKeys = playerKeys;
        let pPlayer = self.pPlayer;

        playerKeys.game_left_mut().fDown = false;
        playerKeys.game_right_mut().fDown = false;

        playerKeys.game_down_mut().fDown = false;
        playerKeys.game_turbo_mut().fDown = false;
        playerKeys.game_powerup_mut().fDown = false;

        if g.player(pPlayer).isdead() || g.player(pPlayer).isspawning() {
            return;
        }

        //Hold down jump until player starts moving down again, then release jump button
        if g.player(pPlayer).inair {
            if g.player(pPlayer).vely > 0.0f32 {
                playerKeys.game_jump_mut().fDown = false;
            } else {
                playerKeys.game_jump_mut().fDown = true;
            }
        } else {
            //Try to jump 1 out of 50 chances when on ground
            if RandomNumberGenerator::generator(gp.get()).get_boolean_scale(ticks(50)) {
                playerKeys.game_jump_mut().fDown = true;
            } else {
                playerKeys.game_jump_mut().fDown = false;
            }
        }
    }
}

/// A target this think found; dead objects stay in their container until `clean_dead_objects`.
fn ai_target(g: &Game, id: ObjectId) -> Ptr<dyn CObjectTrait> {
    g.object(id).unwrap_or_else(|| panic!("no AI target {:?}", id))
}
