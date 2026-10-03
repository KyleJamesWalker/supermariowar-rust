//! Port of src/common/GameModeSettings.cpp

use crate::common::gameplay_styles::{DeathStyle, JailStyle, ScoringStyle, StarStyle};
use crate::common::global_constants::{NUMFRENZYCARDS, NUMSTOMPENEMIES, NUMSURVIVALENEMIES};
use crate::common::match_types::Boss;

macro_rules! default_via_new {
    ($($t:ty),*) => {
        $(impl Default for $t {
            fn default() -> Self {
                <$t>::new()
            }
        })*
    };
}

#[derive(Clone, Debug)]
pub struct ClassicGameModeSettings {
    pub style: DeathStyle,
    pub scoring: ScoringStyle,
}

impl ClassicGameModeSettings {
    pub fn new() -> Self {
        ClassicGameModeSettings { style: DeathStyle::Respawn, scoring: ScoringStyle::AllKills }
    }
}

#[derive(Clone, Debug)]
pub struct FragGameModeSettings {
    pub style: DeathStyle,
    pub scoring: ScoringStyle,
}

impl FragGameModeSettings {
    pub fn new() -> Self {
        FragGameModeSettings { style: DeathStyle::Respawn, scoring: ScoringStyle::AllKills }
    }
}

#[derive(Clone, Debug)]
pub struct TimeGameModeSettings {
    pub style: DeathStyle,
    pub scoring: ScoringStyle,
    pub percentextratime: i16,
}

impl TimeGameModeSettings {
    pub fn new() -> Self {
        TimeGameModeSettings { style: DeathStyle::Respawn, scoring: ScoringStyle::AllKills, percentextratime: 10 }
    }
}

#[derive(Clone, Debug)]
pub struct JailGameModeSettings {
    pub style: JailStyle,
    pub tagfree: bool,
    pub timetofree: i16,
    pub percentkey: i16,
}

impl JailGameModeSettings {
    pub fn new() -> Self {
        JailGameModeSettings { style: JailStyle::Owned, tagfree: true, timetofree: 1240, percentkey: 30 }
    }
}

#[derive(Clone, Debug)]
pub struct CoinGameModeSettings {
    pub penalty: bool,
    pub quantity: i16,
    pub percentextracoin: i16,
}

impl CoinGameModeSettings {
    pub fn new() -> Self {
        CoinGameModeSettings { penalty: false, quantity: 1, percentextracoin: 10 }
    }
}

#[derive(Clone, Debug)]
pub struct StompGameModeSettings {
    pub rate: i16,
    pub enemyweight: [i16; NUMSTOMPENEMIES as usize],
}

impl StompGameModeSettings {
    pub fn new() -> Self {
        StompGameModeSettings { rate: 90, enemyweight: [4, 4, 6, 2, 2, 4, 1, 1, 1] }
    }
}

#[derive(Clone, Debug)]
pub struct EggGameModeSettings {
    pub eggs: [i16; 4],
    pub yoshis: [i16; 4],
    pub explode: i16,
}

impl EggGameModeSettings {
    pub fn new() -> Self {
        EggGameModeSettings { eggs: [0, 1, 0, 0], yoshis: [0, 1, 0, 0], explode: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct FlagGameModeSettings {
    pub speed: i16,
    pub touchreturn: bool,
    pub pointmove: bool,
    pub autoreturn: i16,
    pub homescore: bool,
    pub centerflag: bool,
}

impl FlagGameModeSettings {
    pub fn new() -> Self {
        FlagGameModeSettings { speed: 0, touchreturn: false, pointmove: true, autoreturn: 1240, homescore: false, centerflag: false }
    }
}

#[derive(Clone, Debug)]
pub struct ChickenGameModeSettings {
    pub usetarget: bool,
    pub glide: bool,
}

impl ChickenGameModeSettings {
    pub fn new() -> Self {
        ChickenGameModeSettings { usetarget: true, glide: false }
    }
}

#[derive(Clone, Debug)]
pub struct TagGameModeSettings {
    pub tagontouch: bool,
}

impl TagGameModeSettings {
    pub fn new() -> Self {
        TagGameModeSettings { tagontouch: true }
    }
}

#[derive(Clone, Debug)]
pub struct StarGameModeSettings {
    pub time: i16,
    pub shine: StarStyle,
    pub percentextratime: i16,
}

impl StarGameModeSettings {
    pub fn new() -> Self {
        StarGameModeSettings { time: 30, shine: StarStyle::Ztar, percentextratime: 10 }
    }
}

#[derive(Clone, Debug)]
pub struct DominationGameModeSettings {
    pub loseondeath: bool,
    pub stealondeath: bool,
    pub relocateondeath: bool,
    pub relocationfrequency: i16,
    pub quantity: i16,
}

impl DominationGameModeSettings {
    pub fn new() -> Self {
        DominationGameModeSettings { loseondeath: true, stealondeath: false, relocateondeath: false, relocationfrequency: 1240, quantity: 13 }
    }
}

#[derive(Clone, Debug)]
pub struct KingOfTheHillModeSettings {
    pub areasize: i16,
    pub relocationfrequency: i16,
    pub maxmultiplier: i16,
}

impl KingOfTheHillModeSettings {
    pub fn new() -> Self {
        KingOfTheHillModeSettings { areasize: 3, relocationfrequency: 1240, maxmultiplier: 1 }
    }
}

#[derive(Clone, Debug)]
pub struct RaceGameModeSettings {
    pub quantity: i16,
    pub speed: i16,
    pub penalty: i16,
}

impl RaceGameModeSettings {
    pub fn new() -> Self {
        RaceGameModeSettings { quantity: 4, speed: 4, penalty: 2 }
    }
}

#[derive(Clone, Debug)]
pub struct FrenzyGameModeSettings {
    pub quantity: i16,
    pub rate: i16,
    pub storedshells: bool,
    pub powerupweight: [i16; NUMFRENZYCARDS as usize],
}

impl FrenzyGameModeSettings {
    pub fn new() -> Self {
        let mut powerupweight = [0i16; NUMFRENZYCARDS as usize];
        powerupweight[1] = 1;
        powerupweight[2] = 1;
        FrenzyGameModeSettings { quantity: 6, rate: 186, storedshells: true, powerupweight }
    }
}

#[derive(Clone, Debug)]
pub struct SurvivalGameModeSettings {
    pub enemyweight: [i16; NUMSURVIVALENEMIES as usize],
    pub density: i16,
    pub speed: i16,
    pub shield: bool,
}

impl SurvivalGameModeSettings {
    pub fn new() -> Self {
        SurvivalGameModeSettings { enemyweight: [1, 0, 0], density: 20, speed: 4, shield: true }
    }
}

#[derive(Clone, Debug)]
pub struct GreedGameModeSettings {
    pub coinlife: i16,
    pub owncoins: bool,
    pub multiplier: i16,
    pub percentextracoin: i16,
}

impl GreedGameModeSettings {
    pub fn new() -> Self {
        GreedGameModeSettings { coinlife: 124, owncoins: true, multiplier: 2, percentextracoin: 10 }
    }
}

#[derive(Clone, Debug)]
pub struct HealthGameModeSettings {
    pub startlife: i16,
    pub maxlife: i16,
    pub percentextralife: i16,
}

impl HealthGameModeSettings {
    pub fn new() -> Self {
        HealthGameModeSettings { startlife: 6, maxlife: 1, percentextralife: 20 }
    }
}

#[derive(Clone, Debug)]
pub struct CollectionGameModeSettings {
    pub quantity: i16,
    pub rate: i16,
    pub banktime: i16,
    pub cardlife: i16,
}

impl CollectionGameModeSettings {
    pub fn new() -> Self {
        CollectionGameModeSettings { quantity: 6, rate: 186, banktime: 310, cardlife: 310 }
    }
}

#[derive(Clone, Debug)]
pub struct ChaseGameModeSettings {
    pub phantospeed: i16,
    pub phantoquantity: [i16; 3],
}

impl ChaseGameModeSettings {
    pub fn new() -> Self {
        ChaseGameModeSettings { phantospeed: 6, phantoquantity: [1, 1, 0] }
    }
}

#[derive(Clone, Debug)]
pub struct ShyGuyTagGameModeSettings {
    pub tagonsuicide: bool,
    pub tagtransfer: i16,
    pub freetime: i16,
}

impl ShyGuyTagGameModeSettings {
    pub fn new() -> Self {
        ShyGuyTagGameModeSettings { tagonsuicide: false, tagtransfer: 0, freetime: 5 }
    }
}

#[derive(Clone, Debug)]
pub struct BossGameModeSettings {
    pub bosstype: Boss,
    pub difficulty: i16,
    pub hitpoints: i16,
}

impl BossGameModeSettings {
    pub fn new() -> Self {
        BossGameModeSettings { bosstype: Boss::Hammer, difficulty: 2, hitpoints: 5 }
    }
}

default_via_new!(
    ClassicGameModeSettings,
    FragGameModeSettings,
    TimeGameModeSettings,
    JailGameModeSettings,
    CoinGameModeSettings,
    StompGameModeSettings,
    EggGameModeSettings,
    FlagGameModeSettings,
    ChickenGameModeSettings,
    TagGameModeSettings,
    StarGameModeSettings,
    DominationGameModeSettings,
    KingOfTheHillModeSettings,
    RaceGameModeSettings,
    FrenzyGameModeSettings,
    SurvivalGameModeSettings,
    GreedGameModeSettings,
    HealthGameModeSettings,
    CollectionGameModeSettings,
    ChaseGameModeSettings,
    ShyGuyTagGameModeSettings,
    BossGameModeSettings
);

#[derive(Clone, Debug, Default)]
pub struct GameModeSettings {
    pub classic: ClassicGameModeSettings,
    pub frag: FragGameModeSettings,
    pub time: TimeGameModeSettings,
    pub jail: JailGameModeSettings,
    pub coins: CoinGameModeSettings,
    pub stomp: StompGameModeSettings,
    pub egg: EggGameModeSettings,
    pub flag: FlagGameModeSettings,
    pub chicken: ChickenGameModeSettings,
    pub tag: TagGameModeSettings,
    pub star: StarGameModeSettings,
    pub domination: DominationGameModeSettings,
    pub kingofthehill: KingOfTheHillModeSettings,
    pub race: RaceGameModeSettings,
    pub frenzy: FrenzyGameModeSettings,
    pub survival: SurvivalGameModeSettings,
    pub greed: GreedGameModeSettings,
    pub health: HealthGameModeSettings,
    pub collection: CollectionGameModeSettings,
    pub chase: ChaseGameModeSettings,
    pub shyguytag: ShyGuyTagGameModeSettings,
    pub boss: BossGameModeSettings,
}

impl GameModeSettings {
    pub fn new() -> Self {
        GameModeSettings::default()
    }
}

/// Byte image of the C++ struct (clang layout, 194 bytes) for `read_raw` / `write_raw` of options.bin.
pub const GAMEMODESETTINGS_RAW_SIZE: usize = 194;

struct RawCursor<'a> {
    buf: &'a mut [u8],
    pos: usize,
    write: bool,
}

impl RawCursor<'_> {
    fn align(&mut self, a: usize) {
        self.pos = self.pos.div_ceil(a) * a;
    }
    fn u8(&mut self, v: &mut u8) {
        if self.write {
            self.buf[self.pos] = *v;
        } else {
            *v = self.buf[self.pos];
        }
        self.pos += 1;
    }
    fn bool(&mut self, v: &mut bool) {
        let mut b = *v as u8;
        self.u8(&mut b);
        *v = b != 0;
    }
    fn i16(&mut self, v: &mut i16) {
        self.align(2);
        if self.write {
            self.buf[self.pos..self.pos + 2].copy_from_slice(&v.to_le_bytes());
        } else {
            *v = i16::from_le_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        }
        self.pos += 2;
    }
    fn i16s(&mut self, v: &mut [i16]) {
        for x in v.iter_mut() {
            self.i16(x);
        }
    }
    fn begin(&mut self, align: usize) {
        self.align(align);
    }
    fn end(&mut self, align: usize) {
        self.align(align);
    }
}

macro_rules! raw_enum {
    ($c:expr, $field:expr, $t:ty) => {{
        let mut b = $field as u8;
        $c.u8(&mut b);
        $field = <$t>::from_u8(b);
    }};
}

impl GameModeSettings {
    fn raw_visit(&mut self, c: &mut RawCursor) {
        c.begin(1);
        raw_enum!(c, self.classic.style, DeathStyle);
        raw_enum!(c, self.classic.scoring, ScoringStyle);
        c.end(1);

        c.begin(1);
        raw_enum!(c, self.frag.style, DeathStyle);
        raw_enum!(c, self.frag.scoring, ScoringStyle);
        c.end(1);

        c.begin(2);
        raw_enum!(c, self.time.style, DeathStyle);
        raw_enum!(c, self.time.scoring, ScoringStyle);
        c.i16(&mut self.time.percentextratime);
        c.end(2);

        c.begin(2);
        raw_enum!(c, self.jail.style, JailStyle);
        c.bool(&mut self.jail.tagfree);
        c.i16(&mut self.jail.timetofree);
        c.i16(&mut self.jail.percentkey);
        c.end(2);

        c.begin(2);
        c.bool(&mut self.coins.penalty);
        c.i16(&mut self.coins.quantity);
        c.i16(&mut self.coins.percentextracoin);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.stomp.rate);
        c.i16s(&mut self.stomp.enemyweight);
        c.end(2);

        c.begin(2);
        c.i16s(&mut self.egg.eggs);
        c.i16s(&mut self.egg.yoshis);
        c.i16(&mut self.egg.explode);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.flag.speed);
        c.bool(&mut self.flag.touchreturn);
        c.bool(&mut self.flag.pointmove);
        c.i16(&mut self.flag.autoreturn);
        c.bool(&mut self.flag.homescore);
        c.bool(&mut self.flag.centerflag);
        c.end(2);

        c.begin(1);
        c.bool(&mut self.chicken.usetarget);
        c.bool(&mut self.chicken.glide);
        c.end(1);

        c.begin(1);
        c.bool(&mut self.tag.tagontouch);
        c.end(1);

        c.begin(2);
        c.i16(&mut self.star.time);
        raw_enum!(c, self.star.shine, StarStyle);
        c.i16(&mut self.star.percentextratime);
        c.end(2);

        c.begin(2);
        c.bool(&mut self.domination.loseondeath);
        c.bool(&mut self.domination.stealondeath);
        c.bool(&mut self.domination.relocateondeath);
        c.i16(&mut self.domination.relocationfrequency);
        c.i16(&mut self.domination.quantity);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.kingofthehill.areasize);
        c.i16(&mut self.kingofthehill.relocationfrequency);
        c.i16(&mut self.kingofthehill.maxmultiplier);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.race.quantity);
        c.i16(&mut self.race.speed);
        c.i16(&mut self.race.penalty);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.frenzy.quantity);
        c.i16(&mut self.frenzy.rate);
        c.bool(&mut self.frenzy.storedshells);
        c.i16s(&mut self.frenzy.powerupweight);
        c.end(2);

        c.begin(2);
        c.i16s(&mut self.survival.enemyweight);
        c.i16(&mut self.survival.density);
        c.i16(&mut self.survival.speed);
        c.bool(&mut self.survival.shield);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.greed.coinlife);
        c.bool(&mut self.greed.owncoins);
        c.i16(&mut self.greed.multiplier);
        c.i16(&mut self.greed.percentextracoin);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.health.startlife);
        c.i16(&mut self.health.maxlife);
        c.i16(&mut self.health.percentextralife);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.collection.quantity);
        c.i16(&mut self.collection.rate);
        c.i16(&mut self.collection.banktime);
        c.i16(&mut self.collection.cardlife);
        c.end(2);

        c.begin(2);
        c.i16(&mut self.chase.phantospeed);
        c.i16s(&mut self.chase.phantoquantity);
        c.end(2);

        c.begin(2);
        c.bool(&mut self.shyguytag.tagonsuicide);
        c.i16(&mut self.shyguytag.tagtransfer);
        c.i16(&mut self.shyguytag.freetime);
        c.end(2);

        c.begin(2);
        raw_enum!(c, self.boss.bosstype, Boss);
        c.i16(&mut self.boss.difficulty);
        c.i16(&mut self.boss.hitpoints);
        c.end(2);
    }

    pub fn to_raw_bytes(&self) -> [u8; GAMEMODESETTINGS_RAW_SIZE] {
        let mut buf = [0u8; GAMEMODESETTINGS_RAW_SIZE];
        let mut copy = self.clone();
        let mut c = RawCursor { buf: &mut buf, pos: 0, write: true };
        copy.raw_visit(&mut c);
        debug_assert_eq!(c.pos, GAMEMODESETTINGS_RAW_SIZE);
        buf
    }

    pub fn from_raw_bytes(&mut self, bytes: &[u8; GAMEMODESETTINGS_RAW_SIZE]) {
        let mut buf = *bytes;
        let mut c = RawCursor { buf: &mut buf, pos: 0, write: false };
        self.raw_visit(&mut c);
        debug_assert_eq!(c.pos, GAMEMODESETTINGS_RAW_SIZE);
    }
}

#[cfg(test)]
mod raw_tests {
    use super::*;

    #[test]
    fn raw_layout_roundtrip() {
        let mut s = GameModeSettings::new();
        s.boss.hitpoints = 77;
        s.tag.tagontouch = false;
        let bytes = s.to_raw_bytes();
        // boss at offset 188 in the C++ struct (tools/ref/layout_ref.cpp)
        assert_eq!(i16::from_le_bytes([bytes[192], bytes[193]]), 77);
        let mut t = GameModeSettings::new();
        t.from_raw_bytes(&bytes);
        assert_eq!(t.boss.hitpoints, 77);
        assert!(!t.tag.tagontouch);
        assert_eq!(t.to_raw_bytes(), bytes);
    }
}
