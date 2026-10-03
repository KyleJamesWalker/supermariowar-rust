//! Port of src/common/WorldTourStop.cpp

use crate::common::game_mode::*;
use crate::common::game_mode_settings::*;
use crate::common::gameplay_styles::*;
use crate::common::global::{game_values, maplist};
use crate::common::global_constants::*;
use crate::common::match_types::Boss;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::version::Version;
use crate::globals::{Ptr, Aliased};
use crate::smw::main::gamemodes;

const DELIM_EMPTY: &str = "";
const DELIM_COMMA: &str = ",";
const DELIM_PIPE: &str = "|";
const UNKNOWN_MAP_NAME: &str = "-";

#[derive(Clone, Debug, Default)]
pub struct WorldStageBonus {
    pub iWinnerPlace: i16,
    pub iBonus: i16,
    pub szBonusString: String,
}

#[derive(Clone, Debug, Default)]
pub struct TourStop {
    pub pszMapFile: String,
    pub iMode: i16,
    pub iGoal: i16,
    pub iPoints: i16,
    pub iBonusType: i16,
    pub szName: String,

    pub fEndStage: bool,
    pub iNumBonuses: i16,
    pub wsbBonuses: [WorldStageBonus; 10],
    pub iStageType: i16,

    pub fUseSettings: bool,
    pub iNumUsedSettings: i16,
    pub gmsSettings: GameModeSettings,

    pub iBonusTextLines: i16,
    pub szBonusText: [String; 5],
    pub _alias: Aliased,
}

/// C `strtok` over a NUL-terminated byte buffer; positions are indices into `buf`.
pub struct Strtok {
    pub buf: Vec<u8>,
    next: Option<usize>,
}

impl Strtok {
    /// Copies `s` and appends the terminating NUL.
    pub fn new(s: &[u8]) -> Self {
        let mut buf = s.to_vec();
        buf.push(0);
        Strtok { buf, next: Some(0) }
    }

    /// `strtok(first ? buffer : NULL, delims)`
    pub fn tok(&mut self, delims: &[u8]) -> Option<usize> {
        let mut p = self.next?;
        while self.buf[p] != 0 && delims.contains(&self.buf[p]) {
            p += 1;
        }
        if self.buf[p] == 0 {
            self.next = None;
            return None;
        }
        let start = p;
        while self.buf[p] != 0 && !delims.contains(&self.buf[p]) {
            p += 1;
        }
        if self.buf[p] != 0 {
            self.buf[p] = 0;
            self.next = Some(p + 1);
        } else {
            self.next = None;
        }
        Some(start)
    }

    /// The C string starting at `p`.
    pub fn cstr(&self, p: usize) -> String {
        let end = p + self.buf[p..].iter().position(|&c| c == 0).unwrap_or(self.buf.len() - p);
        crate::common::file_io::cstr_bytes_to_string(&self.buf[p..end])
    }

    pub fn at(&self, p: usize) -> u8 {
        self.buf.get(p).copied().unwrap_or(0)
    }

    pub fn atoi(&self, p: usize) -> i32 {
        crate::common::map_list::atoi(&self.cstr(p))
    }

    /// `strstr(p, needle)` for a single-byte needle.
    pub fn strchr(&self, p: usize, c: u8) -> Option<usize> {
        let mut i = p;
        while self.buf[i] != 0 {
            if self.buf[i] == c {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

/// `ReadTourStopSetting`: the next integer token, or `defaultVal`; returns whether a token was read.
trait TourStopSetting: Sized + Copy {
    fn from_atoi(v: i32) -> Self;
    fn to_short(self) -> i16;
}
impl TourStopSetting for i16 {
    fn from_atoi(v: i32) -> Self {
        v as i16
    }
    fn to_short(self) -> i16 {
        self
    }
}
impl TourStopSetting for bool {
    fn from_atoi(v: i32) -> Self {
        v != 0
    }
    fn to_short(self) -> i16 {
        self as i16
    }
}
macro_rules! u8_enum_setting {
    ($($t:ty),*) => {$(
        impl TourStopSetting for $t {
            fn from_atoi(v: i32) -> Self {
                <$t>::from_u8(v as u8)
            }
            fn to_short(self) -> i16 {
                self as u8 as i16
            }
        }
    )*};
}
u8_enum_setting!(DeathStyle, ScoringStyle, JailStyle, StarStyle, Boss);

fn read<T: TourStopSetting>(st: &mut Strtok, output: &mut T, defaultVal: T) -> bool {
    match st.tok(b",\n") {
        Some(tok) => {
            *output = T::from_atoi(st.atoi(tok));
            true
        }
        None => {
            *output = defaultVal;
            false
        }
    }
}

fn serialize_gms(gamemodeId: i16, g: &GameModeSettings) -> Vec<i16> {
    let s = |v: &[i16]| v.to_vec();
    match gamemodeId as i32 {
        game_mode_classic => s(&[g.classic.style.to_short(), g.classic.scoring.to_short()]),
        game_mode_frag => s(&[g.frag.style.to_short(), g.frag.scoring.to_short()]),
        game_mode_timelimit => s(&[g.time.style.to_short(), g.time.scoring.to_short(), g.time.percentextratime]),
        game_mode_jail => s(&[g.jail.style.to_short(), g.jail.timetofree, g.jail.tagfree as i16, g.jail.percentkey]),
        game_mode_coins => s(&[g.coins.penalty as i16, g.coins.quantity, g.coins.percentextracoin]),
        game_mode_stomp => {
            let mut v = vec![g.stomp.rate];
            v.extend_from_slice(&g.stomp.enemyweight);
            v
        }
        game_mode_eggs => {
            let mut v = g.egg.eggs.to_vec();
            v.extend_from_slice(&g.egg.yoshis);
            v.push(g.egg.explode);
            v
        }
        game_mode_ctf => s(&[
            g.flag.speed,
            g.flag.touchreturn as i16,
            g.flag.pointmove as i16,
            g.flag.autoreturn,
            g.flag.homescore as i16,
            g.flag.centerflag as i16,
        ]),
        game_mode_chicken => s(&[g.chicken.usetarget as i16, g.chicken.glide as i16]),
        game_mode_tag => s(&[g.tag.tagontouch as i16]),
        game_mode_star => s(&[g.star.time, g.star.shine.to_short(), g.star.percentextratime]),
        game_mode_domination => s(&[
            g.domination.quantity,
            g.domination.relocationfrequency,
            g.domination.loseondeath as i16,
            g.domination.relocateondeath as i16,
            g.domination.stealondeath as i16,
        ]),
        game_mode_koth => s(&[g.kingofthehill.areasize, g.kingofthehill.relocationfrequency, g.kingofthehill.maxmultiplier]),
        game_mode_race => s(&[g.race.quantity, g.race.speed, g.race.penalty]),
        game_mode_frenzy => {
            let mut v = vec![g.frenzy.quantity, g.frenzy.rate, g.frenzy.storedshells as i16];
            v.extend_from_slice(&g.frenzy.powerupweight);
            v
        }
        game_mode_survival => {
            let mut v = g.survival.enemyweight.to_vec();
            v.push(g.survival.density);
            v.push(g.survival.speed);
            v.push(g.survival.shield as i16);
            v
        }
        game_mode_greed => s(&[g.greed.coinlife, g.greed.owncoins as i16, g.greed.multiplier, g.greed.percentextracoin]),
        game_mode_health => s(&[g.health.startlife, g.health.maxlife, g.health.percentextralife]),
        game_mode_collection => s(&[g.collection.quantity, g.collection.rate, g.collection.banktime, g.collection.cardlife]),
        game_mode_chase => {
            let mut v = vec![g.chase.phantospeed];
            v.extend_from_slice(&g.chase.phantoquantity);
            v
        }
        game_mode_shyguytag => s(&[g.shyguytag.tagonsuicide as i16, g.shyguytag.tagtransfer, g.shyguytag.freetime]),
        game_mode_boss_minigame => s(&[g.boss.bosstype.to_short(), g.boss.difficulty, g.boss.hitpoints]),
        _ => Vec::new(),
    }
}

fn deserialize_gms(st: &mut Strtok, gamemodeId: i16, g: &mut GameModeSettings) -> Vec<bool> {
    let d = unsafe { &game_values.gamemodemenusettings };
    let mut v = Vec::new();
    match gamemodeId as i32 {
        game_mode_classic => {
            v.push(read(st, &mut g.classic.style, d.classic.style));
            v.push(read(st, &mut g.classic.scoring, d.classic.scoring));
        }
        game_mode_frag => {
            v.push(read(st, &mut g.frag.style, d.frag.style));
            v.push(read(st, &mut g.frag.scoring, d.frag.scoring));
        }
        game_mode_timelimit => {
            v.push(read(st, &mut g.time.style, d.time.style));
            v.push(read(st, &mut g.time.scoring, d.time.scoring));
            v.push(read(st, &mut g.time.percentextratime, d.time.percentextratime));
        }
        game_mode_jail => {
            v.push(read(st, &mut g.jail.style, d.jail.style));
            v.push(read(st, &mut g.jail.timetofree, d.jail.timetofree));
            v.push(read(st, &mut g.jail.tagfree, d.jail.tagfree));
            v.push(read(st, &mut g.jail.percentkey, d.jail.percentkey));
        }
        game_mode_coins => {
            v.push(read(st, &mut g.coins.penalty, d.coins.penalty));
            v.push(read(st, &mut g.coins.quantity, d.coins.quantity));
            v.push(read(st, &mut g.coins.percentextracoin, d.coins.percentextracoin));
        }
        game_mode_stomp => {
            v.push(read(st, &mut g.stomp.rate, d.stomp.rate));
            for i in 0..g.stomp.enemyweight.len() {
                v.push(read(st, &mut g.stomp.enemyweight[i], d.stomp.enemyweight[i]));
            }
        }
        game_mode_eggs => {
            for i in 0..g.egg.eggs.len() {
                v.push(read(st, &mut g.egg.eggs[i], d.egg.eggs[i]));
            }
            for i in 0..g.egg.yoshis.len() {
                v.push(read(st, &mut g.egg.yoshis[i], d.egg.yoshis[i]));
            }
            v.push(read(st, &mut g.egg.explode, d.egg.explode));
        }
        game_mode_ctf => {
            v.push(read(st, &mut g.flag.speed, d.flag.speed));
            v.push(read(st, &mut g.flag.touchreturn, d.flag.touchreturn));
            v.push(read(st, &mut g.flag.pointmove, d.flag.pointmove));
            v.push(read(st, &mut g.flag.autoreturn, d.flag.autoreturn));
            v.push(read(st, &mut g.flag.homescore, d.flag.homescore));
            v.push(read(st, &mut g.flag.centerflag, d.flag.centerflag));
        }
        game_mode_chicken => {
            v.push(read(st, &mut g.chicken.usetarget, d.chicken.usetarget));
            v.push(read(st, &mut g.chicken.glide, d.chicken.glide));
        }
        game_mode_tag => {
            v.push(read(st, &mut g.tag.tagontouch, d.tag.tagontouch));
        }
        game_mode_star => {
            v.push(read(st, &mut g.star.time, d.star.time));
            v.push(read(st, &mut g.star.shine, d.star.shine));
            v.push(read(st, &mut g.star.percentextratime, d.star.percentextratime));
        }
        game_mode_domination => {
            v.push(read(st, &mut g.domination.quantity, d.domination.quantity));
            v.push(read(st, &mut g.domination.relocationfrequency, d.domination.relocationfrequency));
            v.push(read(st, &mut g.domination.loseondeath, d.domination.loseondeath));
            v.push(read(st, &mut g.domination.relocateondeath, d.domination.relocateondeath));
            v.push(read(st, &mut g.domination.stealondeath, d.domination.stealondeath));
        }
        game_mode_koth => {
            v.push(read(st, &mut g.kingofthehill.areasize, d.kingofthehill.areasize));
            v.push(read(st, &mut g.kingofthehill.relocationfrequency, d.kingofthehill.relocationfrequency));
            v.push(read(st, &mut g.kingofthehill.maxmultiplier, d.kingofthehill.maxmultiplier));
        }
        game_mode_race => {
            v.push(read(st, &mut g.race.quantity, d.race.quantity));
            v.push(read(st, &mut g.race.speed, d.race.speed));
            v.push(read(st, &mut g.race.penalty, d.race.penalty));
        }
        game_mode_frenzy => {
            v.push(read(st, &mut g.frenzy.quantity, d.frenzy.quantity));
            v.push(read(st, &mut g.frenzy.rate, d.frenzy.rate));
            v.push(read(st, &mut g.frenzy.storedshells, d.frenzy.storedshells));
            for i in 0..g.frenzy.powerupweight.len() {
                v.push(read(st, &mut g.frenzy.powerupweight[i], d.frenzy.powerupweight[i]));
            }
        }
        game_mode_survival => {
            for i in 0..g.survival.enemyweight.len() {
                v.push(read(st, &mut g.survival.enemyweight[i], d.survival.enemyweight[i]));
            }
            v.push(read(st, &mut g.survival.density, d.survival.density));
            v.push(read(st, &mut g.survival.speed, d.survival.speed));
            v.push(read(st, &mut g.survival.shield, d.survival.shield));
        }
        game_mode_greed => {
            v.push(read(st, &mut g.greed.coinlife, d.greed.coinlife));
            v.push(read(st, &mut g.greed.owncoins, d.greed.owncoins));
            v.push(read(st, &mut g.greed.multiplier, d.greed.multiplier));
            v.push(read(st, &mut g.greed.percentextracoin, d.greed.percentextracoin));
        }
        game_mode_health => {
            v.push(read(st, &mut g.health.startlife, d.health.startlife));
            v.push(read(st, &mut g.health.maxlife, d.health.maxlife));
            v.push(read(st, &mut g.health.percentextralife, d.health.percentextralife));
        }
        game_mode_collection => {
            v.push(read(st, &mut g.collection.quantity, d.collection.quantity));
            v.push(read(st, &mut g.collection.rate, d.collection.rate));
            v.push(read(st, &mut g.collection.banktime, d.collection.banktime));
            v.push(read(st, &mut g.collection.cardlife, d.collection.cardlife));
        }
        game_mode_chase => {
            v.push(read(st, &mut g.chase.phantospeed, d.chase.phantospeed));
            for i in 0..g.chase.phantoquantity.len() {
                v.push(read(st, &mut g.chase.phantoquantity[i], d.chase.phantoquantity[i]));
            }
        }
        game_mode_shyguytag => {
            v.push(read(st, &mut g.shyguytag.tagonsuicide, d.shyguytag.tagonsuicide));
            v.push(read(st, &mut g.shyguytag.tagtransfer, d.shyguytag.tagtransfer));
            v.push(read(st, &mut g.shyguytag.freetime, d.shyguytag.freetime));
        }
        game_mode_boss_minigame => {
            v.push(read(st, &mut g.boss.bosstype, d.boss.bosstype));
            v.push(read(st, &mut g.boss.difficulty, d.boss.difficulty));
            v.push(read(st, &mut g.boss.hitpoints, d.boss.hitpoints));
        }
        _ => {}
    }
    v
}

/// `ParseTourStopLine(char* buffer, ...)`; `buffer` is the line as read by `fgets`, without the NUL.
pub fn parse_tour_stop_line(buffer: &[u8], version: &Version, fIsWorld: bool) -> TourStop {
    let mut st = Strtok::new(buffer);
    let mut ts = TourStop { fUseSettings: false, iNumUsedSettings: 0, ..Default::default() };

    let mut pszTemp = st.tok(b",\n");

    ts.iStageType = 0;
    if fIsWorld {
        ts.iStageType = st.atoi(pszTemp.expect("strtok returned NULL")) as i16;
        if ts.iStageType < 0 || ts.iStageType > 1 {
            ts.iStageType = 0;
        }

        pszTemp = st.tok(b",\n");
    }

    for t in ts.szBonusText.iter_mut() {
        t.clear();
    }

    let v = |major, minor, patch, build| Version { major, minor, patch, build };

    if ts.iStageType == 0 {
        let szMap = st.cstr(pszTemp.expect("strtok returned NULL"));
        pszTemp = st.tok(b",\n");

        ts.iMode = match pszTemp {
            Some(p) => st.atoi(p) as i16,
            None => -1,
        };

        let mode = ts.iMode as i32;
        let isMinigame = mode == game_mode_pipe_minigame || mode == game_mode_boss_minigame || mode == game_mode_boxes_minigame;

        unsafe {
            if *version <= v(1, 8, 0, 2) && isMinigame {
                ts.pszMapFile = UNKNOWN_MAP_NAME.to_string();
            } else {
                maplist.save_current();

                let fMapFound = maplist.findexact(&szMap, true);
                if !fMapFound {
                    if isMinigame {
                        ts.pszMapFile = UNKNOWN_MAP_NAME.to_string();
                    } else {
                        maplist.random(false);
                        ts.pszMapFile = maplist.current_shortmapname().to_string();
                    }
                } else {
                    ts.pszMapFile = maplist.current_shortmapname().to_string();
                }

                maplist.resume_current();
            }
        }

        if *version <= v(1, 8, 0, 2) && ts.iMode == 24 {
            ts.iMode = game_mode_pipe_minigame as i16;
        }

        if ts.iMode < 0 || (ts.iMode as i32 >= GAMEMODE_LAST && !isMinigame) {
            ts.iMode = RANDOM_INT(GAMEMODE_LAST) as i16;
        }

        pszTemp = st.tok(b",\n");

        ts.iGoal = -1;
        if let Some(p) = pszTemp {
            ts.iGoal = st.atoi(p) as i16;
        }

        if ts.iGoal <= 0 {
            if (ts.iMode as i32) < GAMEMODE_LAST {
                let idx = RANDOM_INT(GAMEMODE_NUM_OPTIONS as i32 - 1) as usize;
                ts.iGoal = unsafe { gamemodes[ts.iMode as usize].get_options()[idx].iValue };
            } else {
                ts.iGoal = 50;
            }
        }

        if *version >= v(1, 7, 0, 2) {
            pszTemp = st.tok(b",\n");

            ts.iPoints = match pszTemp {
                Some(p) => st.atoi(p) as i16,
                None => 1,
            };

            pszTemp = st.tok(b",\n");

            if fIsWorld {
                ts.iBonusType = 0;
                ts.iNumBonuses = 0;

                let mut pszStart = pszTemp;

                while let Some(start) = pszStart {
                    let pszEnd = st.strchr(start, b'|');
                    if let Some(e) = pszEnd {
                        st.buf[e] = 0;
                    }

                    let mut iWinnerPlace: i16 = st.at(start) as i16 - 48;
                    if iWinnerPlace == 0 {
                        break;
                    } else if !(1..=4).contains(&iWinnerPlace) {
                        iWinnerPlace = 1;
                    }

                    let b = &mut ts.wsbBonuses[ts.iNumBonuses as usize];
                    b.szBonusString = st.cstr(start);
                    b.iWinnerPlace = iWinnerPlace - 1;

                    let mut iPowerupOffset: i16 = 0;
                    if st.at(start + 1) == b'w' || st.at(start + 1) == b'W' {
                        iPowerupOffset += NUM_POWERUPS as i16;
                    }

                    let mut iBonus: i16 = (st.atoi(start + 2) + iPowerupOffset as i32) as i16;
                    if iBonus < 0 || iBonus as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                        iBonus = 0;
                    }

                    ts.wsbBonuses[ts.iNumBonuses as usize].iBonus = iBonus;

                    ts.iNumBonuses += 1;
                    if ts.iNumBonuses >= 10 {
                        break;
                    }

                    pszStart = pszEnd.map(|e| e + 1);
                }
            } else {
                ts.iBonusType = match pszTemp {
                    Some(p) => st.atoi(p) as i16,
                    None => 0,
                };
            }

            pszTemp = st.tok(b",\n");

            ts.szName = match pszTemp {
                Some(p) => st.cstr(p),
                None => format!("Tour Stop {}", unsafe { game_values.tourstops.len() } + 1),
            };
        } else {
            ts.iPoints = 1;
            ts.iBonusType = 0;
            ts.szName = format!("Tour Stop {}", unsafe { game_values.tourstops.len() } + 1);
        }

        if *version >= v(1, 8, 0, 0) {
            if fIsWorld {
                pszTemp = st.tok(b",\n");

                ts.fEndStage = match pszTemp {
                    Some(p) => st.at(p) == b'1',
                    None => false,
                };
            }

            ts.gmsSettings = unsafe { game_values.gamemodemenusettings.clone() };

            let mut gms = ts.gmsSettings.clone();
            let usedSettings = deserialize_gms(&mut st, ts.iMode, &mut gms);
            ts.gmsSettings = gms;
            ts.iNumUsedSettings = usedSettings.iter().filter(|&&b| b).count() as i16;
            ts.fUseSettings = !usedSettings.is_empty();
        }
    } else if ts.iStageType == 1 {
        ts.szName = match pszTemp {
            Some(p) => st.cstr(p),
            None => format!("Bonus House {}", unsafe { game_values.tourstops.len() } + 1),
        };

        pszTemp = st.tok(b",\n");

        let mut iBonusOrdering: i16 = st.atoi(pszTemp.expect("strtok returned NULL")) as i16;
        if !(0..=1).contains(&iBonusOrdering) {
            iBonusOrdering = 0;
        }

        ts.iBonusType = iBonusOrdering;

        pszTemp = st.tok(b",\n");

        let mut pszStart = pszTemp;

        ts.iBonusTextLines = 0;
        while let Some(start) = pszStart {
            if st.at(start) == b'-' {
                break;
            }
            let pszEnd = st.strchr(start, b'|');

            if let Some(e) = pszEnd {
                st.buf[e] = 0;
            }

            ts.szBonusText[ts.iBonusTextLines as usize] = st.cstr(start);

            ts.iBonusTextLines += 1;
            if ts.iBonusTextLines >= 5 || pszEnd.is_none() {
                break;
            }

            pszStart = pszEnd.map(|e| e + 1);
        }

        ts.iNumBonuses = 0;
        pszTemp = st.tok(b",\n");
        while let Some(p) = pszTemp {
            ts.wsbBonuses[ts.iNumBonuses as usize].szBonusString = st.cstr(p);

            let mut iPowerupOffset: i16 = 0;
            let c = st.at(p);
            if c == b'w' || c == b'W' {
                iPowerupOffset += NUM_POWERUPS as i16;
            } else if c == b's' || c == b'S' {
                iPowerupOffset += (NUM_POWERUPS + NUM_WORLD_POWERUPS - 1) as i16;
            }

            let mut iBonus: i16 = (st.atoi(p + 1) + iPowerupOffset as i32) as i16;
            if iBonus < 0 || iBonus as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS + NUM_WORLD_SCORE_BONUSES {
                iBonus = 0;
            }

            ts.wsbBonuses[ts.iNumBonuses as usize].iBonus = iBonus;
            ts.wsbBonuses[ts.iNumBonuses as usize].iWinnerPlace = -1;

            ts.iNumBonuses += 1;
            if ts.iNumBonuses as i32 >= MAX_BONUS_CHESTS {
                break;
            }

            pszTemp = st.tok(b",\n");
        }
    }

    ts
}

pub fn write_tour_stop_line(ts: &TourStop, fIsWorld: bool) -> String {
    let mut fields: Vec<String> = Vec::new();

    if fIsWorld {
        fields.push(ts.iStageType.to_string());
    }

    if ts.iStageType == 0 {
        fields.push(ts.pszMapFile.clone());
        fields.push(ts.iMode.to_string());
        fields.push(ts.iGoal.to_string());
        fields.push(ts.iPoints.to_string());

        if fIsWorld {
            if ts.iNumBonuses <= 0 {
                fields.push("0".to_string());
            } else {
                let mut field = String::new();
                let mut delim = DELIM_EMPTY;
                for iBonus in 0..ts.iNumBonuses as usize {
                    field += delim;
                    field += &ts.wsbBonuses[iBonus].szBonusString;
                    delim = DELIM_PIPE;
                }
                fields.push(field);
            }
        } else {
            fields.push(ts.iBonusType.to_string());
        }

        fields.push(ts.szName.clone());

        if fIsWorld {
            fields.push((ts.fEndStage as i32).to_string());
        }

        if ts.fUseSettings {
            let mut values = serialize_gms(ts.iMode, &ts.gmsSettings);
            values.resize(ts.iNumUsedSettings.max(0) as usize, 0);
            for value in values {
                fields.push(value.to_string());
            }
        }
    } else if ts.iStageType == 1 {
        fields.push(ts.szName.clone());
        fields.push(ts.iBonusType.to_string());

        let mut field = String::new();
        let mut delim = DELIM_EMPTY;
        for iText in 0..ts.iBonusTextLines.max(0) as usize {
            field += delim;
            field += &ts.szBonusText[iText];
            delim = DELIM_PIPE;
        }
        let _ = field;

        if ts.iNumBonuses == 0 {
            fields.push("p0".to_string());
        } else {
            for iBonus in 0..ts.iNumBonuses.max(0) as usize {
                fields.push(ts.wsbBonuses[iBonus].szBonusString.clone());
            }
        }
    }

    let mut out = String::new();
    let mut delim = DELIM_EMPTY;
    for field in fields {
        out += delim;
        out += &field;
        delim = DELIM_COMMA;
    }

    out += "\n";
    out
}

pub fn reset_tour_stops() {
    unsafe {
        game_values.tourstopcurrent = 0;

        if !game_values.tourstops.is_empty() {
            game_values.tourstops.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strtok_matches_c() {
        let mut st = Strtok::new(b",,a,b\n");
        let a = st.tok(b",\n").unwrap();
        assert_eq!(st.cstr(a), "a");
        let b = st.tok(b",\n").unwrap();
        assert_eq!(st.cstr(b), "b");
        assert!(st.tok(b",\n").is_none());
        assert!(st.tok(b",\n").is_none());
    }
}
