//! Port of src/common/GameModeSettingsSerialization.cpp

use crate::common::game_mode::*;
use crate::common::game_mode_settings::*;
use crate::common::gameplay_styles::*;
use crate::common::match_types::Boss;
use crate::common::world_tour_stop::Strtok;

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

/// Turns the selected game mode settings into a list of integers
pub fn serialize_gms(gamemodeId: i16, g: &GameModeSettings) -> Vec<i16> {
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

/// Parses the active tokenizer of `strtok` (a list of integers) into the selected game mode settings;
/// settings missing from the line take their value from `current`.
pub fn deserialize_gms(st: &mut Strtok, gamemodeId: i16, g: &mut GameModeSettings, d: &GameModeSettings) -> Vec<bool> {
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
