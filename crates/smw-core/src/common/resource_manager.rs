//! Port of src/common/ResourceManager.cpp

use crate::game::Game;
use crate::common::gfx::color::RGB;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
use crate::common::gfx::{gfx_loadfullskin, gfx_loadmenuskin, SpriteStrip};
use crate::common::game::App;
use crate::common::global_constants::*;
use crate::common::path::{convert_path_pack, file_exists};
use crate::common::sfx::{sfxMusic, sfxSound, sfx_can_play_audio};
use crate::globals::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadedSpriteInfo {
    pub path: PathBuf,
    pub colorScheme: i16,
}

/// A sprite in `CResourceManager::sprites`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SpriteId(u16);

impl SpriteId {
    /// No sprite: what a null `gfxSprite*` was in the C++.
    pub const NONE: SpriteId = SpriteId(u16::MAX);

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// A player skin's sprites, one per `PGFX_*` frame.
pub type StripIds = [SpriteId; PGFX_LAST as usize];

#[derive(Default)]
pub struct CResourceManager {
    pub spr_player: [StripIds; 4],
    pub spr_shyguy: [StripIds; 4],
    pub spr_chocobo: [StripIds; 4],
    pub spr_bobomb: [StripIds; 4],

    pub loaded_player_sprites: [LoadedSpriteInfo; 4],
    pub spr_clouds: SpriteId,
    pub spr_ghosts: SpriteId,
    pub spr_fish: SpriteId,
    pub spr_leaves: SpriteId,
    pub spr_snow: SpriteId,
    pub spr_rain: SpriteId,
    pub spr_background: SpriteId,
    pub spr_backmap: [SpriteId; 2],
    pub spr_frontmap: [SpriteId; 2],
    pub spr_overlay: SpriteId,
    pub menu_backdrop: SpriteId,
    pub menu_font_small: gfxFont,
    pub menu_font_large: gfxFont,
    pub game_font_small: gfxFont,
    pub game_font_large: gfxFont,
    pub menu_shade: SpriteId,
    pub menu_smw: SpriteId,
    pub menu_version: SpriteId,
    pub menu_plain_field: SpriteId,
    pub menu_player_select: SpriteId,
    pub menu_map_filter: SpriteId,
    pub menu_match_select: SpriteId,
    pub menu_dialog: SpriteId,
    pub menu_slider_bar: SpriteId,
    pub menu_verticalarrows: SpriteId,
    pub menu_stomp: SpriteId,
    pub menu_survival: SpriteId,
    pub menu_egg: SpriteId,
    pub menu_mode_small: SpriteId,
    pub menu_mode_large: SpriteId,
    pub spr_dialog: SpriteId,
    pub spr_dialogbutton: SpriteId,
    pub spr_tournament_background: SpriteId,
    pub spr_tournament_powerup_splash: SpriteId,
    pub spr_player_select_background: SpriteId,
    pub spr_player_select_ready: SpriteId,
    pub spr_selectfield: SpriteId,
    pub spr_selectfielddisabled: SpriteId,
    pub spr_map_filter_icons: SpriteId,
    pub spr_tour_markers: SpriteId,
    pub spr_menu_boxed_numbers: SpriteId,
    pub spr_countdown_numbers: SpriteId,
    pub spr_thumbnail_warps: [SpriteId; 2],
    pub spr_thumbnail_mapitems: [SpriteId; 2],
    pub spr_platformstarttile: SpriteId,
    pub spr_platformendtile: SpriteId,
    pub spr_platformpath: SpriteId,
    pub spr_worldbackground: [SpriteId; 3],
    pub spr_worldforeground: [SpriteId; 3],
    pub spr_worldforegroundspecial: [SpriteId; 3],
    pub spr_worldpaths: [SpriteId; 3],
    pub spr_worldvehicle: [SpriteId; 3],
    pub spr_worlditems: SpriteId,
    pub spr_worlditempopup: SpriteId,
    pub spr_worlditemssmall: SpriteId,
    pub spr_worlditemsplace: SpriteId,
    pub spr_worldbonushouse: SpriteId,
    pub spr_announcementicons: SpriteId,
    pub spr_noteblock: SpriteId,
    pub spr_breakableblock: SpriteId,
    pub spr_powerupblock: SpriteId,
    pub spr_donutblock: SpriteId,
    pub spr_flipblock: SpriteId,
    pub spr_bounceblock: SpriteId,
    pub spr_throwblock: SpriteId,
    pub spr_switchblocks: SpriteId,
    pub spr_viewblock: SpriteId,
    pub spr_weaponbreakableblock: SpriteId,
    pub spr_brokenyellowblock: SpriteId,
    pub spr_brokenflipblock: SpriteId,
    pub spr_brokenblueblock: SpriteId,
    pub spr_brokengrayblock: SpriteId,
    pub spr_brokeniceblock: SpriteId,
    pub spr_iceblock: SpriteId,
    pub spr_tileanimation: [SpriteId; 3],
    pub spr_blocks: [SpriteId; 3],
    pub spr_unknowntile: [SpriteId; 3],
    pub spr_starpowerup: SpriteId,
    pub spr_1uppowerup: SpriteId,
    pub spr_2uppowerup: SpriteId,
    pub spr_3uppowerup: SpriteId,
    pub spr_5uppowerup: SpriteId,
    pub spr_firepowerup: SpriteId,
    pub spr_hammerpowerup: SpriteId,
    pub spr_icewandpowerup: SpriteId,
    pub spr_podobopowerup: SpriteId,
    pub spr_poisonpowerup: SpriteId,
    pub spr_mysterymushroompowerup: SpriteId,
    pub spr_boomerangpowerup: SpriteId,
    pub spr_clockpowerup: SpriteId,
    pub spr_bobombpowerup: SpriteId,
    pub spr_powpowerup: SpriteId,
    pub spr_modpowerup: SpriteId,
    pub spr_bulletbillpowerup: SpriteId,
    pub spr_featherpowerup: SpriteId,
    pub spr_leafpowerup: SpriteId,
    pub spr_bombpowerup: SpriteId,
    pub spr_pwingspowerup: SpriteId,
    pub spr_tanooki: SpriteId,
    pub spr_statue: SpriteId,
    pub spr_extraheartpowerup: SpriteId,
    pub spr_extratimepowerup: SpriteId,
    pub spr_jailkeypowerup: SpriteId,
    pub spr_secret1: SpriteId,
    pub spr_secret2: SpriteId,
    pub spr_secret3: SpriteId,
    pub spr_secret4: SpriteId,
    pub spr_shade: [SpriteId; 3],
    pub spr_scorehearts: SpriteId,
    pub spr_scorecards: SpriteId,
    pub spr_scorecoins: SpriteId,
    pub spr_timershade: SpriteId,
    pub spr_scoretext: SpriteId,
    pub spr_racetext: SpriteId,
    pub spr_crown: SpriteId,
    pub spr_warplock: SpriteId,
    pub spr_cape: SpriteId,
    pub spr_tail: SpriteId,
    pub spr_wings: SpriteId,
    pub spr_coinsparkle: SpriteId,
    pub spr_shinesparkle: SpriteId,
    pub spr_shellbounce: SpriteId,
    pub spr_superstomp: SpriteId,
    pub spr_coin: SpriteId,
    pub spr_egg: SpriteId,
    pub spr_eggnumbers: SpriteId,
    pub spr_star: SpriteId,
    pub spr_frenzycards: SpriteId,
    pub spr_collectcards: SpriteId,
    pub spr_flags: SpriteId,
    pub spr_yoshi: SpriteId,
    pub spr_thwomp: SpriteId,
    pub spr_podobo: SpriteId,
    pub spr_bowserfire: SpriteId,
    pub spr_areas: SpriteId,
    pub spr_kingofthehillarea: SpriteId,
    pub spr_jail: SpriteId,
    pub spr_racegoal: SpriteId,
    pub spr_pipegamebonus: SpriteId,
    pub spr_chicken: SpriteId,
    pub spr_bonuschest: SpriteId,
    pub spr_teleportstar: SpriteId,
    pub spr_goomba: SpriteId,
    pub spr_goombadead: SpriteId,
    pub spr_goombadeadflying: SpriteId,
    pub spr_koopa: SpriteId,
    pub spr_buzzybeetle: SpriteId,
    pub spr_spiny: SpriteId,
    pub spr_paragoomba: SpriteId,
    pub spr_parakoopa: SpriteId,
    pub spr_redparakoopa: SpriteId,
    pub spr_sledgebrothers: SpriteId,
    pub spr_sledgebrothersdead: SpriteId,
    pub spr_redkoopa: SpriteId,
    pub spr_cheepcheep: SpriteId,
    pub spr_cheepcheepdead: SpriteId,
    pub spr_bulletbill: SpriteId,
    pub spr_bulletbilldead: SpriteId,
    pub spr_fireball: SpriteId,
    pub spr_hammer: SpriteId,
    pub spr_iceblast: SpriteId,
    pub spr_boomerang: SpriteId,
    pub spr_shell: SpriteId,
    pub spr_shelldead: SpriteId,
    pub spr_blueblock: SpriteId,
    pub spr_spring: SpriteId,
    pub spr_spike: SpriteId,
    pub spr_bomb: SpriteId,
    pub spr_kuriboshoe: SpriteId,
    pub spr_throwbox: SpriteId,
    pub spr_sledgehammer: SpriteId,
    pub spr_superfireball: SpriteId,
    pub spr_hazard_fireball: [SpriteId; 3],
    pub spr_hazard_rotodisc: [SpriteId; 3],
    pub spr_hazard_bulletbill: [SpriteId; 3],
    pub spr_hazard_bulletbilldead: SpriteId,
    pub spr_hazard_flame: [SpriteId; 3],
    pub spr_hazard_pirhanaplant: [SpriteId; 3],
    pub spr_fireballexplosion: SpriteId,
    pub spr_frictionsmoke: SpriteId,
    pub spr_bobombsmoke: SpriteId,
    pub spr_explosion: SpriteId,
    pub spr_burnup: SpriteId,
    pub spr_fireworks: SpriteId,
    pub spr_poof: SpriteId,
    pub spr_spawnsmoke: SpriteId,
    pub spr_spawndoor: SpriteId,
    pub spr_bonus: SpriteId,
    pub spr_extralife: SpriteId,
    pub spr_award: SpriteId,
    pub spr_awardsolid: SpriteId,
    pub spr_awardsouls: SpriteId,
    pub spr_awardsoulspawn: SpriteId,
    pub spr_awardkillsinrow: SpriteId,
    pub spr_flagbases: SpriteId,
    pub spr_ownedtags: SpriteId,
    pub spr_phanto: SpriteId,
    pub spr_phantokey: SpriteId,
    pub spr_storedpowerupsmall: SpriteId,
    pub spr_storedpoweruplarge: SpriteId,
    pub spr_powerupselector: SpriteId,
    pub spr_scoreboard: SpriteId,
    pub spr_abovearrows: SpriteId,
    pub spr_windmeter: SpriteId,
    pub spr_overlayhole: SpriteId,
    pub spr_tiletypes: SpriteId,
    pub spr_transparenttiles: SpriteId,
    pub spr_backgroundlevel: SpriteId,
    pub spr_tilesetlevel: SpriteId,
    pub spr_eyecandy: SpriteId,
    pub spr_warps: [SpriteId; 3],
    pub spr_selectedtile: SpriteId,
    pub spr_nospawntile: SpriteId,
    pub spr_noitemspawntile: SpriteId,
    pub spr_mapitems: [SpriteId; 3],
    pub spr_powerups: SpriteId,
    pub spr_hidden_marker: SpriteId,
    pub spr_racegoals: SpriteId,
    pub spr_number_icons: SpriteId,
    pub sfx_announcer: [sfxSound; PANNOUNCER_SOUND_LAST as usize],
    pub sfx_mip: sfxSound,
    pub sfx_deathsound: sfxSound,
    pub sfx_jump: sfxSound,
    pub sfx_skid: sfxSound,
    pub sfx_capejump: sfxSound,
    pub sfx_invinciblemusic: sfxSound,
    pub sfx_extraguysound: sfxSound,
    pub sfx_sprout: sfxSound,
    pub sfx_collectpowerup: sfxSound,
    pub sfx_collectfeather: sfxSound,
    pub sfx_storepowerup: sfxSound,
    pub sfx_tailspin: sfxSound,
    pub sfx_breakblock: sfxSound,
    pub sfx_bump: sfxSound,
    pub sfx_coin: sfxSound,
    pub sfx_fireball: sfxSound,
    pub sfx_springjump: sfxSound,
    pub sfx_timewarning: sfxSound,
    pub sfx_hit: sfxSound,
    pub sfx_chicken: sfxSound,
    pub sfx_transform: sfxSound,
    pub sfx_yoshi: sfxSound,
    pub sfx_pause: sfxSound,
    pub sfx_bobombsound: sfxSound,
    pub sfx_areatag: sfxSound,
    pub sfx_cannon: sfxSound,
    pub sfx_burnup: sfxSound,
    pub sfx_pipe: sfxSound,
    pub sfx_thunder: sfxSound,
    pub sfx_slowdownmusic: sfxSound,
    pub sfx_flyingsound: sfxSound,
    pub sfx_storedpowerupsound: sfxSound,
    pub sfx_kicksound: sfxSound,
    pub sfx_racesound: sfxSound,
    pub sfx_bulletbillsound: sfxSound,
    pub sfx_boomerang: sfxSound,
    pub sfx_spit: sfxSound,
    pub sfx_starwarning: sfxSound,
    pub sfx_powerdown: sfxSound,
    pub sfx_switchpress: sfxSound,
    pub sfx_superspring: sfxSound,
    pub sfx_stun: sfxSound,
    pub sfx_inventory: sfxSound,
    pub sfx_worldmove: sfxSound,
    pub sfx_treasurechest: sfxSound,
    pub sfx_flamecannon: sfxSound,
    pub sfx_wand: sfxSound,
    pub sfx_enterstage: sfxSound,
    pub sfx_gameover: sfxSound,
    pub sfx_pickup: sfxSound,
    pub backgroundmusic: [sfxMusic; 6],
    pub sprites: Vec<Box<std::cell::UnsafeCell<gfxSprite>>>,
    pub _alias: Aliased,
}

impl CResourceManager {
    pub fn new() -> Self {
        fn next(n: &mut u16) -> SpriteId {
            *n += 1;
            SpriteId(*n - 1)
        }
        let mut rm = Self::default();
        let mut n = 0;
        rm.spr_player = std::array::from_fn(|_| std::array::from_fn(|_| next(&mut n)));
        rm.spr_shyguy = std::array::from_fn(|_| std::array::from_fn(|_| next(&mut n)));
        rm.spr_chocobo = std::array::from_fn(|_| std::array::from_fn(|_| next(&mut n)));
        rm.spr_bobomb = std::array::from_fn(|_| std::array::from_fn(|_| next(&mut n)));
        rm.spr_clouds = next(&mut n);
        rm.spr_ghosts = next(&mut n);
        rm.spr_fish = next(&mut n);
        rm.spr_leaves = next(&mut n);
        rm.spr_snow = next(&mut n);
        rm.spr_rain = next(&mut n);
        rm.spr_background = next(&mut n);
        rm.spr_backmap = std::array::from_fn(|_| next(&mut n));
        rm.spr_frontmap = std::array::from_fn(|_| next(&mut n));
        rm.spr_overlay = next(&mut n);
        rm.menu_backdrop = next(&mut n);
        rm.menu_shade = next(&mut n);
        rm.menu_smw = next(&mut n);
        rm.menu_version = next(&mut n);
        rm.menu_plain_field = next(&mut n);
        rm.menu_player_select = next(&mut n);
        rm.menu_map_filter = next(&mut n);
        rm.menu_match_select = next(&mut n);
        rm.menu_dialog = next(&mut n);
        rm.menu_slider_bar = next(&mut n);
        rm.menu_verticalarrows = next(&mut n);
        rm.menu_stomp = next(&mut n);
        rm.menu_survival = next(&mut n);
        rm.menu_egg = next(&mut n);
        rm.menu_mode_small = next(&mut n);
        rm.menu_mode_large = next(&mut n);
        rm.spr_dialog = next(&mut n);
        rm.spr_dialogbutton = next(&mut n);
        rm.spr_tournament_background = next(&mut n);
        rm.spr_tournament_powerup_splash = next(&mut n);
        rm.spr_player_select_background = next(&mut n);
        rm.spr_player_select_ready = next(&mut n);
        rm.spr_selectfield = next(&mut n);
        rm.spr_selectfielddisabled = next(&mut n);
        rm.spr_map_filter_icons = next(&mut n);
        rm.spr_tour_markers = next(&mut n);
        rm.spr_menu_boxed_numbers = next(&mut n);
        rm.spr_countdown_numbers = next(&mut n);
        rm.spr_thumbnail_warps = std::array::from_fn(|_| next(&mut n));
        rm.spr_thumbnail_mapitems = std::array::from_fn(|_| next(&mut n));
        rm.spr_platformstarttile = next(&mut n);
        rm.spr_platformendtile = next(&mut n);
        rm.spr_platformpath = next(&mut n);
        rm.spr_worldbackground = std::array::from_fn(|_| next(&mut n));
        rm.spr_worldforeground = std::array::from_fn(|_| next(&mut n));
        rm.spr_worldforegroundspecial = std::array::from_fn(|_| next(&mut n));
        rm.spr_worldpaths = std::array::from_fn(|_| next(&mut n));
        rm.spr_worldvehicle = std::array::from_fn(|_| next(&mut n));
        rm.spr_worlditems = next(&mut n);
        rm.spr_worlditempopup = next(&mut n);
        rm.spr_worlditemssmall = next(&mut n);
        rm.spr_worlditemsplace = next(&mut n);
        rm.spr_worldbonushouse = next(&mut n);
        rm.spr_announcementicons = next(&mut n);
        rm.spr_noteblock = next(&mut n);
        rm.spr_breakableblock = next(&mut n);
        rm.spr_powerupblock = next(&mut n);
        rm.spr_donutblock = next(&mut n);
        rm.spr_flipblock = next(&mut n);
        rm.spr_bounceblock = next(&mut n);
        rm.spr_throwblock = next(&mut n);
        rm.spr_switchblocks = next(&mut n);
        rm.spr_viewblock = next(&mut n);
        rm.spr_weaponbreakableblock = next(&mut n);
        rm.spr_brokenyellowblock = next(&mut n);
        rm.spr_brokenflipblock = next(&mut n);
        rm.spr_brokenblueblock = next(&mut n);
        rm.spr_brokengrayblock = next(&mut n);
        rm.spr_brokeniceblock = next(&mut n);
        rm.spr_iceblock = next(&mut n);
        rm.spr_tileanimation = std::array::from_fn(|_| next(&mut n));
        rm.spr_blocks = std::array::from_fn(|_| next(&mut n));
        rm.spr_unknowntile = std::array::from_fn(|_| next(&mut n));
        rm.spr_starpowerup = next(&mut n);
        rm.spr_1uppowerup = next(&mut n);
        rm.spr_2uppowerup = next(&mut n);
        rm.spr_3uppowerup = next(&mut n);
        rm.spr_5uppowerup = next(&mut n);
        rm.spr_firepowerup = next(&mut n);
        rm.spr_hammerpowerup = next(&mut n);
        rm.spr_icewandpowerup = next(&mut n);
        rm.spr_podobopowerup = next(&mut n);
        rm.spr_poisonpowerup = next(&mut n);
        rm.spr_mysterymushroompowerup = next(&mut n);
        rm.spr_boomerangpowerup = next(&mut n);
        rm.spr_clockpowerup = next(&mut n);
        rm.spr_bobombpowerup = next(&mut n);
        rm.spr_powpowerup = next(&mut n);
        rm.spr_modpowerup = next(&mut n);
        rm.spr_bulletbillpowerup = next(&mut n);
        rm.spr_featherpowerup = next(&mut n);
        rm.spr_leafpowerup = next(&mut n);
        rm.spr_bombpowerup = next(&mut n);
        rm.spr_pwingspowerup = next(&mut n);
        rm.spr_tanooki = next(&mut n);
        rm.spr_statue = next(&mut n);
        rm.spr_extraheartpowerup = next(&mut n);
        rm.spr_extratimepowerup = next(&mut n);
        rm.spr_jailkeypowerup = next(&mut n);
        rm.spr_secret1 = next(&mut n);
        rm.spr_secret2 = next(&mut n);
        rm.spr_secret3 = next(&mut n);
        rm.spr_secret4 = next(&mut n);
        rm.spr_shade = std::array::from_fn(|_| next(&mut n));
        rm.spr_scorehearts = next(&mut n);
        rm.spr_scorecards = next(&mut n);
        rm.spr_scorecoins = next(&mut n);
        rm.spr_timershade = next(&mut n);
        rm.spr_scoretext = next(&mut n);
        rm.spr_racetext = next(&mut n);
        rm.spr_crown = next(&mut n);
        rm.spr_warplock = next(&mut n);
        rm.spr_cape = next(&mut n);
        rm.spr_tail = next(&mut n);
        rm.spr_wings = next(&mut n);
        rm.spr_coinsparkle = next(&mut n);
        rm.spr_shinesparkle = next(&mut n);
        rm.spr_shellbounce = next(&mut n);
        rm.spr_superstomp = next(&mut n);
        rm.spr_coin = next(&mut n);
        rm.spr_egg = next(&mut n);
        rm.spr_eggnumbers = next(&mut n);
        rm.spr_star = next(&mut n);
        rm.spr_frenzycards = next(&mut n);
        rm.spr_collectcards = next(&mut n);
        rm.spr_flags = next(&mut n);
        rm.spr_yoshi = next(&mut n);
        rm.spr_thwomp = next(&mut n);
        rm.spr_podobo = next(&mut n);
        rm.spr_bowserfire = next(&mut n);
        rm.spr_areas = next(&mut n);
        rm.spr_kingofthehillarea = next(&mut n);
        rm.spr_jail = next(&mut n);
        rm.spr_racegoal = next(&mut n);
        rm.spr_pipegamebonus = next(&mut n);
        rm.spr_chicken = next(&mut n);
        rm.spr_bonuschest = next(&mut n);
        rm.spr_teleportstar = next(&mut n);
        rm.spr_goomba = next(&mut n);
        rm.spr_goombadead = next(&mut n);
        rm.spr_goombadeadflying = next(&mut n);
        rm.spr_koopa = next(&mut n);
        rm.spr_buzzybeetle = next(&mut n);
        rm.spr_spiny = next(&mut n);
        rm.spr_paragoomba = next(&mut n);
        rm.spr_parakoopa = next(&mut n);
        rm.spr_redparakoopa = next(&mut n);
        rm.spr_sledgebrothers = next(&mut n);
        rm.spr_sledgebrothersdead = next(&mut n);
        rm.spr_redkoopa = next(&mut n);
        rm.spr_cheepcheep = next(&mut n);
        rm.spr_cheepcheepdead = next(&mut n);
        rm.spr_bulletbill = next(&mut n);
        rm.spr_bulletbilldead = next(&mut n);
        rm.spr_fireball = next(&mut n);
        rm.spr_hammer = next(&mut n);
        rm.spr_iceblast = next(&mut n);
        rm.spr_boomerang = next(&mut n);
        rm.spr_shell = next(&mut n);
        rm.spr_shelldead = next(&mut n);
        rm.spr_blueblock = next(&mut n);
        rm.spr_spring = next(&mut n);
        rm.spr_spike = next(&mut n);
        rm.spr_bomb = next(&mut n);
        rm.spr_kuriboshoe = next(&mut n);
        rm.spr_throwbox = next(&mut n);
        rm.spr_sledgehammer = next(&mut n);
        rm.spr_superfireball = next(&mut n);
        rm.spr_hazard_fireball = std::array::from_fn(|_| next(&mut n));
        rm.spr_hazard_rotodisc = std::array::from_fn(|_| next(&mut n));
        rm.spr_hazard_bulletbill = std::array::from_fn(|_| next(&mut n));
        rm.spr_hazard_bulletbilldead = next(&mut n);
        rm.spr_hazard_flame = std::array::from_fn(|_| next(&mut n));
        rm.spr_hazard_pirhanaplant = std::array::from_fn(|_| next(&mut n));
        rm.spr_fireballexplosion = next(&mut n);
        rm.spr_frictionsmoke = next(&mut n);
        rm.spr_bobombsmoke = next(&mut n);
        rm.spr_explosion = next(&mut n);
        rm.spr_burnup = next(&mut n);
        rm.spr_fireworks = next(&mut n);
        rm.spr_poof = next(&mut n);
        rm.spr_spawnsmoke = next(&mut n);
        rm.spr_spawndoor = next(&mut n);
        rm.spr_bonus = next(&mut n);
        rm.spr_extralife = next(&mut n);
        rm.spr_award = next(&mut n);
        rm.spr_awardsolid = next(&mut n);
        rm.spr_awardsouls = next(&mut n);
        rm.spr_awardsoulspawn = next(&mut n);
        rm.spr_awardkillsinrow = next(&mut n);
        rm.spr_flagbases = next(&mut n);
        rm.spr_ownedtags = next(&mut n);
        rm.spr_phanto = next(&mut n);
        rm.spr_phantokey = next(&mut n);
        rm.spr_storedpowerupsmall = next(&mut n);
        rm.spr_storedpoweruplarge = next(&mut n);
        rm.spr_powerupselector = next(&mut n);
        rm.spr_scoreboard = next(&mut n);
        rm.spr_abovearrows = next(&mut n);
        rm.spr_windmeter = next(&mut n);
        rm.spr_overlayhole = next(&mut n);
        rm.spr_tiletypes = next(&mut n);
        rm.spr_transparenttiles = next(&mut n);
        rm.spr_backgroundlevel = next(&mut n);
        rm.spr_tilesetlevel = next(&mut n);
        rm.spr_eyecandy = next(&mut n);
        rm.spr_warps = std::array::from_fn(|_| next(&mut n));
        rm.spr_selectedtile = next(&mut n);
        rm.spr_nospawntile = next(&mut n);
        rm.spr_noitemspawntile = next(&mut n);
        rm.spr_mapitems = std::array::from_fn(|_| next(&mut n));
        rm.spr_powerups = next(&mut n);
        rm.spr_hidden_marker = next(&mut n);
        rm.spr_racegoals = next(&mut n);
        rm.spr_number_icons = next(&mut n);
        rm.sprites = (0..n).map(|_| Box::new(std::cell::UnsafeCell::new(gfxSprite::new()))).collect();
        rm
    }

    /// The sprite `id` names. Sprites live as long as the resource manager, which lives for the whole run.
    #[allow(clippy::mut_from_ref)]
    pub fn sprite(&self, id: SpriteId) -> &'static mut gfxSprite {
        unsafe { &mut *self.sprites[id.0 as usize].get() }
    }

    /// A new sprite outside the fixed set, such as an editor's own images.
    pub fn add_sprite(&mut self, sprite: gfxSprite) -> SpriteId {
        self.sprites.push(Box::new(std::cell::UnsafeCell::new(sprite)));
        SpriteId(self.sprites.len() as u16 - 1)
    }

    /// Loads `strip` into the sprites `ids` names.
    pub fn set_strip(&self, ids: StripIds, strip: SpriteStrip) {
        for (id, sprite) in ids.into_iter().zip(strip) {
            *self.sprite(id) = sprite;
        }
    }

    /// Not in the C++: every sound, in a fixed order, for replay checkpoints (smw/checkpoint.rs).
    pub fn sounds(&mut self) -> Vec<*mut sfxSound> {
        let mut sounds: Vec<*mut sfxSound> = self.sfx_announcer.iter_mut().map(|s| s as *mut sfxSound).collect();
        sounds.extend([
            &mut self.sfx_mip as *mut sfxSound,
            &mut self.sfx_deathsound as *mut sfxSound,
            &mut self.sfx_jump as *mut sfxSound,
            &mut self.sfx_skid as *mut sfxSound,
            &mut self.sfx_capejump as *mut sfxSound,
            &mut self.sfx_invinciblemusic as *mut sfxSound,
            &mut self.sfx_extraguysound as *mut sfxSound,
            &mut self.sfx_sprout as *mut sfxSound,
            &mut self.sfx_collectpowerup as *mut sfxSound,
            &mut self.sfx_collectfeather as *mut sfxSound,
            &mut self.sfx_storepowerup as *mut sfxSound,
            &mut self.sfx_tailspin as *mut sfxSound,
            &mut self.sfx_breakblock as *mut sfxSound,
            &mut self.sfx_bump as *mut sfxSound,
            &mut self.sfx_coin as *mut sfxSound,
            &mut self.sfx_fireball as *mut sfxSound,
            &mut self.sfx_springjump as *mut sfxSound,
            &mut self.sfx_timewarning as *mut sfxSound,
            &mut self.sfx_hit as *mut sfxSound,
            &mut self.sfx_chicken as *mut sfxSound,
            &mut self.sfx_transform as *mut sfxSound,
            &mut self.sfx_yoshi as *mut sfxSound,
            &mut self.sfx_pause as *mut sfxSound,
            &mut self.sfx_bobombsound as *mut sfxSound,
            &mut self.sfx_areatag as *mut sfxSound,
            &mut self.sfx_cannon as *mut sfxSound,
            &mut self.sfx_burnup as *mut sfxSound,
            &mut self.sfx_pipe as *mut sfxSound,
            &mut self.sfx_thunder as *mut sfxSound,
            &mut self.sfx_slowdownmusic as *mut sfxSound,
            &mut self.sfx_flyingsound as *mut sfxSound,
            &mut self.sfx_storedpowerupsound as *mut sfxSound,
            &mut self.sfx_kicksound as *mut sfxSound,
            &mut self.sfx_racesound as *mut sfxSound,
            &mut self.sfx_bulletbillsound as *mut sfxSound,
            &mut self.sfx_boomerang as *mut sfxSound,
            &mut self.sfx_spit as *mut sfxSound,
            &mut self.sfx_starwarning as *mut sfxSound,
            &mut self.sfx_powerdown as *mut sfxSound,
            &mut self.sfx_switchpress as *mut sfxSound,
            &mut self.sfx_superspring as *mut sfxSound,
            &mut self.sfx_stun as *mut sfxSound,
            &mut self.sfx_inventory as *mut sfxSound,
            &mut self.sfx_worldmove as *mut sfxSound,
            &mut self.sfx_treasurechest as *mut sfxSound,
            &mut self.sfx_flamecannon as *mut sfxSound,
            &mut self.sfx_wand as *mut sfxSound,
            &mut self.sfx_enterstage as *mut sfxSound,
            &mut self.sfx_gameover as *mut sfxSound,
            &mut self.sfx_pickup as *mut sfxSound,
        ]);
        sounds
    }

    pub fn load_menu_skin(&mut self, g: &mut Game, playerID: i16, skinID: i16, colorID: i16, fLoadBothDirections: bool) -> bool {
        let path = g.skinlist.at(skinID as usize).path.clone();
        self.load_menu_skin_path(g, playerID, &path, colorID, fLoadBothDirections)
    }

    pub fn load_menu_skin_path(&mut self, g: &mut Game, playerID: i16, path: &Path, colorID: i16, fLoadBothDirections: bool) -> bool {
        let p = playerID as usize;
        let new_info = LoadedSpriteInfo { path: path.to_path_buf(), colorScheme: colorID };
        // GSMenu's LoadFullSkin replaces spr_player without updating this cache, as in C++.
        if self.loaded_player_sprites[p] == new_info {
            let both_dirs_present = !self.sprite(self.spr_player[p][PGFX_STANDING_L as usize]).get_surface().is_null()
                && !self.sprite(self.spr_player[p][PGFX_RUNNING_L as usize]).get_surface().is_null();
            let already_loaded = !fLoadBothDirections || both_dirs_present;
            if already_loaded {
                return true;
            }
        }

        match gfx_loadmenuskin(g, path, colorID, fLoadBothDirections) {
            Ok(strip) => {
                self.set_strip(self.spr_player[p], strip);
                self.loaded_player_sprites[p] = new_info;
                true
            }
            Err(what) => {
                println!("ERROR: {}", what);
                false
            }
        }
    }

    /// Panics where the C++ throws (the C++ callers do not catch).
    pub fn load_full_skin_path(&self, g: &mut Game, path: &Path, colorID: i16) -> SpriteStrip {
        gfx_loadfullskin(g, path, colorID).unwrap_or_else(|what| panic!("{}", what))
    }

    pub fn load_full_skin(&self, g: &mut Game, skinID: i16, colorID: i16) -> SpriteStrip {
        let path = g.skinlist.at(skinID as usize).path.clone();
        gfx_loadfullskin(g, &path, colorID).unwrap_or_else(|what| panic!("{}", what))
    }

    fn load_all_sprites(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let graphicspack = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let builder = |relpath: &str| ImageLoader::new(convert_path_pack(gp.get(), relpath, &graphicspack));

        let mut shyguyPath = convert_path_pack(g, "gfx/packs/modeskins/shyguy.png", &graphicspack);
        if !file_exists(&shyguyPath) {
            shyguyPath = convert_path_pack(g, "gfx/packs/modeskins/shyguy.bmp", &graphicspack);
        }

        let mut chickenPath = convert_path_pack(g, "gfx/packs/modeskins/chicken.png", &graphicspack);
        if !file_exists(&chickenPath) {
            chickenPath = convert_path_pack(g, "gfx/packs/modeskins/chicken.bmp", &graphicspack);
        }

        let mut bobombPath = convert_path_pack(g, "gfx/packs/modeskins/bobomb.png", &graphicspack);
        if !file_exists(&bobombPath) {
            bobombPath = convert_path_pack(g, "gfx/packs/modeskins/bobomb.bmp", &graphicspack);
        }

        //Just load menu skins for now (just standing right sprite)
        for k in 0..MAX_PLAYERS as i16 {
            self.set_strip(self.spr_shyguy[k as usize], self.load_full_skin_path(g, Path::new(&shyguyPath), k));
            self.set_strip(self.spr_chocobo[k as usize], self.load_full_skin_path(g, Path::new(&chickenPath), k));
            self.set_strip(self.spr_bobomb[k as usize], self.load_full_skin_path(g, Path::new(&bobombPath), k));
        }

        *self.sprite(self.menu_survival) = builder("gfx/packs/modeobjects/menu_survival.png").create(g);
        *self.sprite(self.menu_stomp) = builder("gfx/packs/modeobjects/menu_stomp.png").create(g);
        *self.sprite(self.menu_egg) = builder("gfx/packs/modeobjects/menu_egg.png").create(g);

        *self.sprite(self.spr_clouds) = builder("gfx/packs/eyecandy/cloud.png").with_alpha(255).with_wrapping(640).create(g);
        *self.sprite(self.spr_ghosts) = builder("gfx/packs/eyecandy/ghost.png").with_alpha(128).with_wrapping(640).create(g);
        *self.sprite(self.spr_fish) = builder("gfx/packs/eyecandy/fish.png").with_alpha(128).with_wrapping(640).create(g);
        *self.sprite(self.spr_leaves) = builder("gfx/packs/eyecandy/leaves.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_snow) = builder("gfx/packs/eyecandy/snow.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_rain) = builder("gfx/packs/eyecandy/rain.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_noteblock) = builder("gfx/packs/blocks/noteblock.png").create(g);
        *self.sprite(self.spr_breakableblock) = builder("gfx/packs/blocks/breakableblock.png").create(g);
        *self.sprite(self.spr_powerupblock) = builder("gfx/packs/blocks/powerupblock.png").create(g);
        *self.sprite(self.spr_donutblock) = builder("gfx/packs/blocks/donutblock.png").create(g);
        *self.sprite(self.spr_flipblock) = builder("gfx/packs/blocks/flipblock.png").create(g);
        *self.sprite(self.spr_bounceblock) = builder("gfx/packs/blocks/bounceblock.png").create(g);
        *self.sprite(self.spr_throwblock) = builder("gfx/packs/blocks/throwblock.png").create(g);
        *self.sprite(self.spr_switchblocks) = builder("gfx/packs/blocks/switchblock.png").create(g);
        *self.sprite(self.spr_viewblock) = builder("gfx/packs/blocks/viewblock.png").create(g);
        *self.sprite(self.spr_weaponbreakableblock) = builder("gfx/packs/blocks/weaponbreakableblock.png").create(g);

        *self.sprite(self.spr_spring) = builder("gfx/packs/powerups/spring.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_spike) = builder("gfx/packs/powerups/spike.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_kuriboshoe) = builder("gfx/packs/powerups/kuriboshoe.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_throwbox) = builder("gfx/packs/powerups/throwbox.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_tileanimation[0]) = builder("gfx/packs/tilesets/tile_animation.png").create(g);
        *self.sprite(self.spr_tileanimation[1]) = builder("gfx/packs/tilesets/tile_animation_preview.png").create(g);
        *self.sprite(self.spr_tileanimation[2]) = builder("gfx/packs/tilesets/tile_animation_thumbnail.png").create(g);

        *self.sprite(self.spr_blocks[0]) = builder("gfx/packs/tilesets/blocks.png").create(g);
        *self.sprite(self.spr_blocks[1]) = builder("gfx/packs/tilesets/blocks_preview.png").create(g);
        *self.sprite(self.spr_blocks[2]) = builder("gfx/packs/tilesets/blocks_thumbnail.png").create(g);

        *self.sprite(self.spr_unknowntile[0]) = builder("gfx/packs/tilesets/unknown_tile.png").create(g);
        *self.sprite(self.spr_unknowntile[1]) = builder("gfx/packs/tilesets/unknown_tile_preview.png").create(g);
        *self.sprite(self.spr_unknowntile[2]) = builder("gfx/packs/tilesets/unknown_tile_thumbnail.png").create(g);

        *self.sprite(self.spr_brokenyellowblock) = builder("gfx/packs/eyecandy/brokenyellowblock.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_brokenflipblock) = builder("gfx/packs/eyecandy/brokenflipblock.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_brokenblueblock) = builder("gfx/packs/eyecandy/brokenblueblock.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_brokengrayblock) = builder("gfx/packs/eyecandy/brokengrayblock.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_brokeniceblock) = builder("gfx/packs/eyecandy/icecube.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_iceblock) = builder("gfx/packs/eyecandy/iceblock.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_tanooki) = builder("gfx/packs/powerups/tanooki.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_statue) = builder("gfx/packs/projectiles/statue.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_starpowerup) = builder("gfx/packs/powerups/starpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_1uppowerup) = builder("gfx/packs/powerups/1uppowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_2uppowerup) = builder("gfx/packs/powerups/2uppowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_3uppowerup) = builder("gfx/packs/powerups/3uppowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_5uppowerup) = builder("gfx/packs/powerups/5uppowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_firepowerup) = builder("gfx/packs/powerups/fireflower.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hammerpowerup) = builder("gfx/packs/powerups/hammerpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_icewandpowerup) = builder("gfx/packs/powerups/icewandpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_podobopowerup) = builder("gfx/packs/powerups/podobopowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_poisonpowerup) = builder("gfx/packs/powerups/poisonpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_mysterymushroompowerup) = builder("gfx/packs/powerups/mysterymushroom.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_boomerangpowerup) = builder("gfx/packs/powerups/boomerangpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_clockpowerup) = builder("gfx/packs/powerups/clockpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_bobombpowerup) = builder("gfx/packs/powerups/bobombpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_powpowerup) = builder("gfx/packs/powerups/powpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_modpowerup) = builder("gfx/packs/powerups/modpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_bulletbillpowerup) = builder("gfx/packs/powerups/bulletbillpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_featherpowerup) = builder("gfx/packs/powerups/featherpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_leafpowerup) = builder("gfx/packs/powerups/leafpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_bombpowerup) = builder("gfx/packs/powerups/bombpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_pwingspowerup) = builder("gfx/packs/powerups/pwings.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_extraheartpowerup) = builder("gfx/packs/powerups/heartpowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_extratimepowerup) = builder("gfx/packs/powerups/extratimepowerup.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_jailkeypowerup) = builder("gfx/packs/powerups/jailkeypowerup.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_secret1) = builder("gfx/packs/powerups/secret1.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_secret2) = builder("gfx/packs/powerups/secret2.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_secret3) = builder("gfx/packs/powerups/secret3.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_secret4) = builder("gfx/packs/powerups/secret4.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_shade[0]) = builder("gfx/packs/eyecandy/shade1.png").with_alpha(64).create(g);
        *self.sprite(self.spr_shade[1]) = builder("gfx/packs/eyecandy/shade2.png").with_alpha(64).create(g);
        *self.sprite(self.spr_shade[2]) = builder("gfx/packs/eyecandy/shade3.png").with_alpha(64).create(g);
        *self.sprite(self.spr_scorehearts) = builder("gfx/packs/menu/score_hearts.png").create(g);
        *self.sprite(self.spr_scorecards) = builder("gfx/packs/menu/score_cards.png").create(g);
        *self.sprite(self.spr_scorecoins) = builder("gfx/packs/menu/score_coins.png").create(g);

        *self.sprite(self.spr_timershade) = builder("gfx/packs/eyecandy/timershade.png").with_alpha(64).create(g);
        *self.sprite(self.spr_scoretext) = builder("gfx/packs/fonts/score.png").create(g);
        *self.sprite(self.spr_racetext) = builder("gfx/packs/fonts/race.png").create(g);

        *self.sprite(self.spr_crown) = builder("gfx/packs/eyecandy/crown.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_cape) = builder("gfx/packs/eyecandy/cape.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_tail) = builder("gfx/packs/eyecandy/tail.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_wings) = builder("gfx/packs/eyecandy/wings.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_warplock) = builder("gfx/packs/eyecandy/warplock.png").create(g);
        *self.sprite(self.spr_coinsparkle) = builder("gfx/packs/eyecandy/coinsparks.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_shinesparkle) = builder("gfx/packs/eyecandy/shinesparks.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_shellbounce) = builder("gfx/packs/eyecandy/shellbounce.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_superstomp) = builder("gfx/packs/eyecandy/supersmash.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_egg) = builder("gfx/packs/modeobjects/egg.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_eggnumbers) = builder("gfx/packs/modeobjects/eggnumbers.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_star) = builder("gfx/packs/modeobjects/star.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_flags) = builder("gfx/packs/modeobjects/flags.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_frenzycards) = builder("gfx/packs/modeobjects/frenzycards.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_collectcards) = builder("gfx/packs/modeobjects/collectcards.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_yoshi) = builder("gfx/packs/modeobjects/yoshi.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_coin) = builder("gfx/packs/modeobjects/coin.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_thwomp) = builder("gfx/packs/modeobjects/thwomp.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_podobo) = builder("gfx/packs/modeobjects/podobo.png").create(g);
        *self.sprite(self.spr_bowserfire) = builder("gfx/packs/modeobjects/bowserfire.png").create(g);
        *self.sprite(self.spr_areas) = builder("gfx/packs/modeobjects/areas.png").create(g);
        *self.sprite(self.spr_kingofthehillarea) = builder("gfx/packs/modeobjects/kingofthehill.png").with_alpha(128).create(g);
        *self.sprite(self.spr_jail) = builder("gfx/packs/modeobjects/jail.png").with_alpha(160).with_wrapping(640).create(g);
        *self.sprite(self.spr_goomba) = builder("gfx/packs/modeobjects/goomba.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_goombadead) = builder("gfx/packs/eyecandy/goombadead.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_goombadeadflying) = builder("gfx/packs/eyecandy/goombadeadflying.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_koopa) = builder("gfx/packs/modeobjects/koopa.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_buzzybeetle) = builder("gfx/packs/modeobjects/buzzybeetle.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_spiny) = builder("gfx/packs/modeobjects/spiny.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_paragoomba) = builder("gfx/packs/modeobjects/paragoomba.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_parakoopa) = builder("gfx/packs/modeobjects/parakoopa.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_redparakoopa) = builder("gfx/packs/modeobjects/redparakoopa.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_redkoopa) = builder("gfx/packs/modeobjects/redkoopa.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_cheepcheep) = builder("gfx/packs/modeobjects/cheepcheep.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_cheepcheepdead) = builder("gfx/packs/eyecandy/cheepcheepdead.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_sledgebrothers) = builder("gfx/packs/modeobjects/sledgebrothers.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_sledgebrothersdead) = builder("gfx/packs/eyecandy/sledgebrothersdead.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_bulletbill) = builder("gfx/packs/projectiles/bulletbill.png").create(g);
        *self.sprite(self.spr_bulletbilldead) = builder("gfx/packs/eyecandy/bulletbilldead.png").create(g);
        *self.sprite(self.spr_chicken) = builder("gfx/packs/modeobjects/chicken.png").with_alpha(160).with_wrapping(640).create(g);
        *self.sprite(self.spr_racegoal) = builder("gfx/packs/modeobjects/racegoal.png").create(g);
        *self.sprite(self.spr_pipegamebonus) = builder("gfx/packs/modeobjects/pipeminigamebonuses.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_phanto) = builder("gfx/packs/modeobjects/phanto.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_phantokey) = builder("gfx/packs/modeobjects/key.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_bonuschest) = builder("gfx/packs/modeobjects/bonuschest.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_teleportstar) = builder("gfx/packs/eyecandy/teleportstar.png").create(g);

        *self.sprite(self.spr_fireball) = builder("gfx/packs/projectiles/fireball.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hammer) = builder("gfx/packs/projectiles/hammer.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_iceblast) = builder("gfx/packs/projectiles/wandblast.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_boomerang) = builder("gfx/packs/projectiles/boomerang.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_shell) = builder("gfx/packs/projectiles/shell.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_shelldead) = builder("gfx/packs/eyecandy/shelldead.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_blueblock) = builder("gfx/packs/projectiles/throwblock.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_bomb) = builder("gfx/packs/projectiles/bomb.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_superfireball) = builder("gfx/packs/modeobjects/superfire.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_sledgehammer) = builder("gfx/packs/modeobjects/sledgehammer.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_hazard_fireball[0]) = builder("gfx/packs/hazards/fireball.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_fireball[1]) = builder("gfx/packs/hazards/fireball_preview.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_fireball[2]) = builder("gfx/packs/hazards/fireball_thumbnail.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_hazard_rotodisc[0]) = builder("gfx/packs/hazards/rotodisc.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_rotodisc[1]) = builder("gfx/packs/hazards/rotodisc_preview.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_rotodisc[2]) = builder("gfx/packs/hazards/rotodisc_thumbnail.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_hazard_bulletbill[0]) = builder("gfx/packs/hazards/bulletbill.png").create(g);
        *self.sprite(self.spr_hazard_bulletbill[1]) = builder("gfx/packs/hazards/bulletbill_preview.png").create(g);
        *self.sprite(self.spr_hazard_bulletbill[2]) = builder("gfx/packs/hazards/bulletbill_thumbnail.png").create(g);

        *self.sprite(self.spr_hazard_flame[0]) = builder("gfx/packs/hazards/flame.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_flame[1]) = builder("gfx/packs/hazards/flame_preview.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_flame[2]) = builder("gfx/packs/hazards/flame_thumbnail.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_hazard_pirhanaplant[0]) = builder("gfx/packs/hazards/pirhanaplant.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_pirhanaplant[1]) = builder("gfx/packs/hazards/pirhanaplant_preview.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_hazard_pirhanaplant[2]) = builder("gfx/packs/hazards/pirhanaplant_thumbnail.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_hazard_bulletbilldead) = builder("gfx/packs/hazards/bulletbilldead.png").create(g);

        *self.sprite(self.spr_fireballexplosion) = builder("gfx/packs/eyecandy/fireballexplosion.png").with_alpha(160).with_wrapping(640).create(g);
        *self.sprite(self.spr_frictionsmoke) = builder("gfx/packs/eyecandy/frictionsmoke.png").with_alpha(160).with_wrapping(640).create(g);
        *self.sprite(self.spr_bobombsmoke) = builder("gfx/packs/eyecandy/bobombsmoke.png").with_alpha(160).with_wrapping(640).create(g);
        *self.sprite(self.spr_explosion) = builder("gfx/packs/eyecandy/explosion.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_burnup) = builder("gfx/packs/eyecandy/burnup.png").with_alpha(192).with_wrapping(640).create(g);
        *self.sprite(self.spr_fireworks) = builder("gfx/packs/eyecandy/fireworks.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_poof) = builder("gfx/packs/eyecandy/poof.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_spawnsmoke) = builder("gfx/packs/eyecandy/spawnsmoke.png").with_alpha(128).with_wrapping(640).create(g);
        *self.sprite(self.spr_spawndoor) = builder("gfx/packs/eyecandy/spawndoor.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_bonus) = builder("gfx/packs/eyecandy/bonus.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_extralife) = builder("gfx/packs/eyecandy/extralife.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_windmeter) = builder("gfx/packs/eyecandy/wind_meter.png").with_alpha(192).with_wrapping(640).create(g);
        *self.sprite(self.spr_overlayhole) = builder("gfx/packs/eyecandy/overlayholes.png").with_color_key(RGB { r: 0, g: 255, b: 0 }).with_wrapping(640).create(g);

        *self.sprite(self.spr_award) = builder("gfx/packs/awards/killsinrow.png").with_alpha(128).with_wrapping(640).create(g);
        *self.sprite(self.spr_awardsolid) = builder("gfx/packs/awards/killsinrow.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_awardsouls) = builder("gfx/packs/awards/souls.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_awardsoulspawn) = builder("gfx/packs/awards/soulspawn.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_awardkillsinrow) = builder("gfx/packs/awards/killsinrownumbers.png").with_wrapping(640).create(g);

        *self.sprite(self.spr_flagbases) = builder("gfx/packs/modeobjects/flagbases.png").with_alpha(160).with_wrapping(640).create(g);
        *self.sprite(self.spr_ownedtags) = builder("gfx/packs/modeobjects/ownedtags.png").with_alpha(160).with_wrapping(640).create(g);

        *self.sprite(self.spr_storedpowerupsmall) = builder("gfx/packs/powerups/small.png").with_wrapping(640).create(g);
        *self.sprite(self.spr_storedpoweruplarge) = builder("gfx/packs/powerups/large.png").create(g);
        *self.sprite(self.spr_powerupselector) = builder("gfx/packs/awards/award.png").create(g);

        *self.sprite(self.spr_abovearrows) = builder("gfx/packs/eyecandy/abovearrows.png").with_wrapping(640).create(g);
    }

    pub fn load_menu_graphics(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let graphicspack = g.menugraphicspacklist.current_path().to_string_lossy().into_owned();
        let builder = |relpath: &str| ImageLoader::new(convert_path_pack(gp.get(), relpath, &graphicspack));

        *self.sprite(self.menu_shade) = builder("gfx/packs/menu/menu_shade.png").with_alpha(App::menuTransparency as u8).without_color_key().create(g);

        *self.sprite(self.spr_scoreboard) = builder("gfx/packs/menu/scoreboard.png").create(g);
        *self.sprite(self.menu_slider_bar) = builder("gfx/packs/menu/menu_slider_bar.png").create(g);
        *self.sprite(self.menu_plain_field) = builder("gfx/packs/menu/menu_plain_field.png").create(g);
        *self.sprite(self.menu_player_select) = builder("gfx/packs/menu/menu_player_select.png").create(g);
        *self.sprite(self.menu_dialog) = builder("gfx/packs/menu/menu_dialog.png").create(g);
        *self.sprite(self.menu_map_filter) = builder("gfx/packs/menu/menu_map_filter.png").create(g);
        *self.sprite(self.menu_match_select) = builder("gfx/packs/menu/menu_match_select.png").create(g);

        *self.sprite(self.menu_verticalarrows) = builder("gfx/packs/menu/menu_vertical_arrows.png").create(g);

        *self.sprite(self.menu_mode_small) = builder("gfx/packs/menu/menu_mode_small.png").create(g);
        *self.sprite(self.menu_mode_large) = builder("gfx/packs/menu/menu_mode_large.png").create(g);

        *self.sprite(self.spr_dialog) = builder("gfx/packs/menu/dialog.png").create(g);
        *self.sprite(self.spr_dialogbutton) = builder("gfx/packs/menu/dialog_button.png").create(g);
        *self.sprite(self.spr_tournament_background) = builder("gfx/packs/menu/tournament_background.png").create(g);
        *self.sprite(self.spr_tournament_powerup_splash) = builder("gfx/packs/menu/tournament_powerup_splash.png").create(g);
        *self.sprite(self.spr_player_select_background) = builder("gfx/packs/menu/player_select_background.png").create(g);
        *self.sprite(self.spr_player_select_ready) = builder("gfx/packs/menu/player_select_ready.png").create(g);
        //spr_ipfield = builder("gfx/packs/menu/menu_ipfield.png").create();
        *self.sprite(self.spr_selectfield) = builder("gfx/packs/menu/menu_selectfield.png").create(g);
        *self.sprite(self.spr_selectfielddisabled) = builder("gfx/packs/menu/menu_selectfield_disabled.png").create(g);
        *self.sprite(self.spr_map_filter_icons) = builder("gfx/packs/menu/menu_map_flags.png").create(g);
        *self.sprite(self.spr_tour_markers) = builder("gfx/packs/menu/tour_markers.png").create(g);
        *self.sprite(self.spr_menu_boxed_numbers) = builder("gfx/packs/menu/menu_boxed_numbers.png").create(g);
        *self.sprite(self.spr_countdown_numbers) = builder("gfx/packs/menu/game_countdown_numbers.png").create(g);
        *self.sprite(self.spr_thumbnail_warps[0]) = builder("gfx/packs/menu/menu_warp_preview.png").create(g);
        *self.sprite(self.spr_thumbnail_warps[1]) = builder("gfx/packs/menu/menu_warp_thumbnail.png").create(g);
        *self.sprite(self.spr_thumbnail_mapitems[0]) = builder("gfx/packs/menu/menu_mapitems_preview.png").create(g);
        *self.sprite(self.spr_thumbnail_mapitems[1]) = builder("gfx/packs/menu/menu_mapitems_thumbnail.png").create(g);

        *self.sprite(self.spr_announcementicons) = builder("gfx/packs/menu/menu_announcement_icons.png").create(g);

        *self.sprite(self.spr_platformstarttile) = builder("gfx/leveleditor/leveleditor_platformstarttile.png").with_alpha(64).with_wrapping(640).create(g);
        *self.sprite(self.spr_platformendtile) = builder("gfx/leveleditor/leveleditor_selectedtile.png").with_alpha(64).with_wrapping(640).create(g);
        *self.sprite(self.spr_platformpath) = builder("gfx/leveleditor/leveleditor_platform_path.png").with_alpha(128).with_wrapping(640).create(g);
    }

    pub fn load_world_graphics(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let graphicspack = g.worldgraphicspacklist.current_path().to_string_lossy().into_owned();
        let builder = |relpath: &str| ImageLoader::new(convert_path_pack(gp.get(), relpath, &graphicspack));

        *self.sprite(self.spr_worldbackground[0]) = builder("gfx/packs/world/world_background.png").create(g);
        *self.sprite(self.spr_worldbackground[1]) = builder("gfx/packs/world/preview/world_background.png").create(g);

        *self.sprite(self.spr_worldforeground[0]) = builder("gfx/packs/world/world_foreground.png").create(g);
        *self.sprite(self.spr_worldforeground[1]) = builder("gfx/packs/world/preview/world_foreground.png").create(g);

        *self.sprite(self.spr_worldforegroundspecial[0]) = builder("gfx/packs/world/world_foreground_special.png").create(g);
        *self.sprite(self.spr_worldforegroundspecial[1]) = builder("gfx/packs/world/preview/world_foreground_special.png").create(g);

        *self.sprite(self.spr_worldpaths[0]) = builder("gfx/packs/world/world_paths.png").create(g);
        *self.sprite(self.spr_worldpaths[1]) = builder("gfx/packs/world/preview/world_paths.png").create(g);

        *self.sprite(self.spr_worldvehicle[0]) = builder("gfx/packs/world/world_vehicles.png").create(g);
        *self.sprite(self.spr_worldvehicle[1]) = builder("gfx/packs/world/preview/world_vehicles.png").create(g);

        *self.sprite(self.spr_worlditems) = builder("gfx/packs/world/world_powerups.png").create(g);
        *self.sprite(self.spr_worlditempopup) = builder("gfx/packs/world/world_item_popup.png").create(g);
        *self.sprite(self.spr_worlditemssmall) = builder("gfx/packs/world/world_powerupssmall.png").create(g);
        *self.sprite(self.spr_worlditemsplace) = builder("gfx/packs/world/world_bonusplace.png").create(g);
        *self.sprite(self.spr_worldbonushouse) = builder("gfx/packs/world/world_bonushouse.png").create(g);
    }

    pub fn load_game_graphics(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let graphicspack = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();

        g.g_tilesetmanager = Ptr::new_box(crate::common::tileset_manager::CTilesetManager::new(g, &graphicspack));

        self.game_font_small = gfxFont::from_path(g, &convert_path_pack(gp.get(), "gfx/packs/fonts/font_small.png", &graphicspack));
        self.game_font_large = gfxFont::from_path(g, &convert_path_pack(gp.get(), "gfx/packs/fonts/font_large.png", &graphicspack));

        self.load_all_sprites(g);
    }

    pub fn load_start_graphics(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let graphicspack = g.menugraphicspacklist.current_path().to_string_lossy().into_owned();
        let builder = |relpath: &str| ImageLoader::new(convert_path_pack(gp.get(), relpath, &graphicspack));

        self.menu_font_small = gfxFont::from_path(g, &convert_path_pack(gp.get(), "gfx/packs/menu/menu_font_small.png", &graphicspack));
        self.menu_font_large = gfxFont::from_path(g, &convert_path_pack(gp.get(), "gfx/packs/menu/menu_font_large.png", &graphicspack));

        //load basic stuff
        *self.sprite(self.menu_backdrop) = builder("gfx/packs/menu/menu_background.png").without_color_key().create(g);
        *self.sprite(self.menu_smw) = builder("gfx/packs/menu/menu_smw.png").create(g);
        *self.sprite(self.menu_version) = builder("gfx/packs/menu/menu_version.png").create(g);
    }

    pub fn load_all_graphics(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let graphicspack = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let builder = |relpath: &str| ImageLoader::new(convert_path_pack(gp.get(), relpath, &graphicspack));

        self.load_menu_graphics(g);
        self.load_world_graphics(g);
        self.load_game_graphics(g);

        *self.sprite(self.spr_backmap[0]) = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create(g);
        *self.sprite(self.spr_backmap[1]) = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create(g);
        *self.sprite(self.spr_frontmap[0]) = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create(g);
        *self.sprite(self.spr_frontmap[1]) = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create(g);

        *self.sprite(self.spr_overlay) = builder("gfx/packs/menu/menu_shade.png").create(g);
    }

    /// Panics where the C++ throws on a missing sound (the C++ callers do not catch).
    pub fn load_game_sounds(&mut self, g: &mut Game) -> bool {
        let mut gp = g.ptr();
        g.game_values.soundcapable = false;

        if !sfx_can_play_audio(g) {
            return false;
        }

        let soundpack = g.soundpacklist.current_path().to_string_lossy().into_owned();
        let mut open = |relpath: &str| {
            sfxSound::from_file(g, Path::new(&convert_path_pack(gp.get(), relpath, &soundpack))).unwrap_or_else(|what| panic!("{}", what))
        };

        self.sfx_mip = open("sfx/packs/mip.wav");
        self.sfx_deathsound = open("sfx/packs/death.wav");
        self.sfx_jump = open("sfx/packs/jump.wav");
        self.sfx_skid = open("sfx/packs/skid.wav");
        self.sfx_capejump = open("sfx/packs/capejump.wav");
        self.sfx_invinciblemusic = open("sfx/packs/invincible.wav");
        self.sfx_extraguysound = open("sfx/packs/1up.wav");
        self.sfx_sprout = open("sfx/packs/sprout.wav");
        self.sfx_collectpowerup = open("sfx/packs/collectpowerup.wav");
        self.sfx_collectfeather = open("sfx/packs/feather.wav");
        self.sfx_tailspin = open("sfx/packs/tail.wav");
        self.sfx_storepowerup = open("sfx/packs/storeitem.wav");
        self.sfx_breakblock = open("sfx/packs/breakblock.wav");
        self.sfx_bump = open("sfx/packs/bump.wav");
        self.sfx_coin = open("sfx/packs/coin.wav");
        self.sfx_fireball = open("sfx/packs/fireball.wav");
        self.sfx_springjump = open("sfx/packs/springjump.wav");
        self.sfx_timewarning = open("sfx/packs/timewarning.wav");
        self.sfx_hit = open("sfx/packs/hit.wav");
        self.sfx_chicken = open("sfx/packs/chicken.wav");
        self.sfx_transform = open("sfx/packs/transform.wav");
        self.sfx_yoshi = open("sfx/packs/yoshi.wav");
        self.sfx_pause = open("sfx/packs/pause.wav");
        self.sfx_bobombsound = open("sfx/packs/bob-omb.wav");
        self.sfx_areatag = open("sfx/packs/dcoin.wav");
        self.sfx_cannon = open("sfx/packs/cannon.wav");
        self.sfx_burnup = open("sfx/packs/burnup.wav");
        self.sfx_pipe = open("sfx/packs/warp.wav");
        self.sfx_thunder = open("sfx/packs/thunder.wav");
        self.sfx_slowdownmusic = open("sfx/packs/clock.wav");
        self.sfx_flyingsound = open("sfx/packs/slowdown.wav");
        self.sfx_storedpowerupsound = open("sfx/packs/storedpowerup.wav");
        self.sfx_kicksound = open("sfx/packs/kick.wav");
        self.sfx_racesound = open("sfx/packs/race.wav");
        self.sfx_bulletbillsound = open("sfx/packs/bulletbill.wav");
        self.sfx_boomerang = open("sfx/packs/boomerang.wav");
        self.sfx_spit = open("sfx/packs/spit.wav");
        self.sfx_starwarning = open("sfx/packs/starwarning.wav");
        self.sfx_powerdown = open("sfx/packs/powerdown.wav");
        self.sfx_switchpress = open("sfx/packs/switchpress.wav");
        self.sfx_superspring = open("sfx/packs/superspring.wav");
        self.sfx_stun = open("sfx/packs/stun.wav");
        self.sfx_inventory = open("sfx/packs/inventory.wav");
        self.sfx_worldmove = open("sfx/packs/mapmove.wav");
        self.sfx_treasurechest = open("sfx/packs/treasurechest.wav");
        self.sfx_flamecannon = open("sfx/packs/flamecannon.wav");
        self.sfx_wand = open("sfx/packs/wand.wav");
        self.sfx_enterstage = open("sfx/packs/enter-stage.wav");
        self.sfx_gameover = open("sfx/packs/gameover.wav");
        self.sfx_pickup = open("sfx/packs/pickup.wav");

        g.game_values.soundcapable = true;
        true
    }
}
