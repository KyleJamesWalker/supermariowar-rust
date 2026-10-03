//! Port of src/common/ResourceManager.cpp

use crate::common::gfx::color::RGB;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::{gfxSprite, SpriteBuilder};
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

#[derive(Default)]
pub struct CResourceManager {
    pub spr_player: [SpriteStrip; 4],
    pub spr_shyguy: [SpriteStrip; 4],
    pub spr_chocobo: [SpriteStrip; 4],
    pub spr_bobomb: [SpriteStrip; 4],

    pub loaded_player_sprites: [LoadedSpriteInfo; 4],
    pub spr_clouds: gfxSprite,
    pub spr_ghosts: gfxSprite,
    pub spr_fish: gfxSprite,
    pub spr_leaves: gfxSprite,
    pub spr_snow: gfxSprite,
    pub spr_rain: gfxSprite,
    pub spr_background: gfxSprite,
    pub spr_backmap: [gfxSprite; 2],
    pub spr_frontmap: [gfxSprite; 2],
    pub spr_overlay: gfxSprite,
    pub menu_backdrop: gfxSprite,
    pub menu_font_small: gfxFont,
    pub menu_font_large: gfxFont,
    pub game_font_small: gfxFont,
    pub game_font_large: gfxFont,
    pub menu_shade: gfxSprite,
    pub menu_smw: gfxSprite,
    pub menu_version: gfxSprite,
    pub menu_plain_field: gfxSprite,
    pub menu_player_select: gfxSprite,
    pub menu_map_filter: gfxSprite,
    pub menu_match_select: gfxSprite,
    pub menu_dialog: gfxSprite,
    pub menu_slider_bar: gfxSprite,
    pub menu_verticalarrows: gfxSprite,
    pub menu_stomp: gfxSprite,
    pub menu_survival: gfxSprite,
    pub menu_egg: gfxSprite,
    pub menu_mode_small: gfxSprite,
    pub menu_mode_large: gfxSprite,
    pub spr_dialog: gfxSprite,
    pub spr_dialogbutton: gfxSprite,
    pub spr_tournament_background: gfxSprite,
    pub spr_tournament_powerup_splash: gfxSprite,
    pub spr_player_select_background: gfxSprite,
    pub spr_player_select_ready: gfxSprite,
    pub spr_selectfield: gfxSprite,
    pub spr_selectfielddisabled: gfxSprite,
    pub spr_map_filter_icons: gfxSprite,
    pub spr_tour_markers: gfxSprite,
    pub spr_menu_boxed_numbers: gfxSprite,
    pub spr_countdown_numbers: gfxSprite,
    pub spr_thumbnail_warps: [gfxSprite; 2],
    pub spr_thumbnail_mapitems: [gfxSprite; 2],
    pub spr_platformstarttile: gfxSprite,
    pub spr_platformendtile: gfxSprite,
    pub spr_platformpath: gfxSprite,
    pub spr_worldbackground: [gfxSprite; 3],
    pub spr_worldforeground: [gfxSprite; 3],
    pub spr_worldforegroundspecial: [gfxSprite; 3],
    pub spr_worldpaths: [gfxSprite; 3],
    pub spr_worldvehicle: [gfxSprite; 3],
    pub spr_worlditems: gfxSprite,
    pub spr_worlditempopup: gfxSprite,
    pub spr_worlditemssmall: gfxSprite,
    pub spr_worlditemsplace: gfxSprite,
    pub spr_worldbonushouse: gfxSprite,
    pub spr_announcementicons: gfxSprite,
    pub spr_noteblock: gfxSprite,
    pub spr_breakableblock: gfxSprite,
    pub spr_powerupblock: gfxSprite,
    pub spr_donutblock: gfxSprite,
    pub spr_flipblock: gfxSprite,
    pub spr_bounceblock: gfxSprite,
    pub spr_throwblock: gfxSprite,
    pub spr_switchblocks: gfxSprite,
    pub spr_viewblock: gfxSprite,
    pub spr_weaponbreakableblock: gfxSprite,
    pub spr_brokenyellowblock: gfxSprite,
    pub spr_brokenflipblock: gfxSprite,
    pub spr_brokenblueblock: gfxSprite,
    pub spr_brokengrayblock: gfxSprite,
    pub spr_brokeniceblock: gfxSprite,
    pub spr_iceblock: gfxSprite,
    pub spr_tileanimation: [gfxSprite; 3],
    pub spr_blocks: [gfxSprite; 3],
    pub spr_unknowntile: [gfxSprite; 3],
    pub spr_starpowerup: gfxSprite,
    pub spr_1uppowerup: gfxSprite,
    pub spr_2uppowerup: gfxSprite,
    pub spr_3uppowerup: gfxSprite,
    pub spr_5uppowerup: gfxSprite,
    pub spr_firepowerup: gfxSprite,
    pub spr_hammerpowerup: gfxSprite,
    pub spr_icewandpowerup: gfxSprite,
    pub spr_podobopowerup: gfxSprite,
    pub spr_poisonpowerup: gfxSprite,
    pub spr_mysterymushroompowerup: gfxSprite,
    pub spr_boomerangpowerup: gfxSprite,
    pub spr_clockpowerup: gfxSprite,
    pub spr_bobombpowerup: gfxSprite,
    pub spr_powpowerup: gfxSprite,
    pub spr_modpowerup: gfxSprite,
    pub spr_bulletbillpowerup: gfxSprite,
    pub spr_featherpowerup: gfxSprite,
    pub spr_leafpowerup: gfxSprite,
    pub spr_bombpowerup: gfxSprite,
    pub spr_pwingspowerup: gfxSprite,
    pub spr_tanooki: gfxSprite,
    pub spr_statue: gfxSprite,
    pub spr_extraheartpowerup: gfxSprite,
    pub spr_extratimepowerup: gfxSprite,
    pub spr_jailkeypowerup: gfxSprite,
    pub spr_secret1: gfxSprite,
    pub spr_secret2: gfxSprite,
    pub spr_secret3: gfxSprite,
    pub spr_secret4: gfxSprite,
    pub spr_shade: [gfxSprite; 3],
    pub spr_scorehearts: gfxSprite,
    pub spr_scorecards: gfxSprite,
    pub spr_scorecoins: gfxSprite,
    pub spr_timershade: gfxSprite,
    pub spr_scoretext: gfxSprite,
    pub spr_racetext: gfxSprite,
    pub spr_crown: gfxSprite,
    pub spr_warplock: gfxSprite,
    pub spr_cape: gfxSprite,
    pub spr_tail: gfxSprite,
    pub spr_wings: gfxSprite,
    pub spr_coinsparkle: gfxSprite,
    pub spr_shinesparkle: gfxSprite,
    pub spr_shellbounce: gfxSprite,
    pub spr_superstomp: gfxSprite,
    pub spr_coin: gfxSprite,
    pub spr_egg: gfxSprite,
    pub spr_eggnumbers: gfxSprite,
    pub spr_star: gfxSprite,
    pub spr_frenzycards: gfxSprite,
    pub spr_collectcards: gfxSprite,
    pub spr_flags: gfxSprite,
    pub spr_yoshi: gfxSprite,
    pub spr_thwomp: gfxSprite,
    pub spr_podobo: gfxSprite,
    pub spr_bowserfire: gfxSprite,
    pub spr_areas: gfxSprite,
    pub spr_kingofthehillarea: gfxSprite,
    pub spr_jail: gfxSprite,
    pub spr_racegoal: gfxSprite,
    pub spr_pipegamebonus: gfxSprite,
    pub spr_chicken: gfxSprite,
    pub spr_bonuschest: gfxSprite,
    pub spr_teleportstar: gfxSprite,
    pub spr_goomba: gfxSprite,
    pub spr_goombadead: gfxSprite,
    pub spr_goombadeadflying: gfxSprite,
    pub spr_koopa: gfxSprite,
    pub spr_buzzybeetle: gfxSprite,
    pub spr_spiny: gfxSprite,
    pub spr_paragoomba: gfxSprite,
    pub spr_parakoopa: gfxSprite,
    pub spr_redparakoopa: gfxSprite,
    pub spr_sledgebrothers: gfxSprite,
    pub spr_sledgebrothersdead: gfxSprite,
    pub spr_redkoopa: gfxSprite,
    pub spr_cheepcheep: gfxSprite,
    pub spr_cheepcheepdead: gfxSprite,
    pub spr_bulletbill: gfxSprite,
    pub spr_bulletbilldead: gfxSprite,
    pub spr_fireball: gfxSprite,
    pub spr_hammer: gfxSprite,
    pub spr_iceblast: gfxSprite,
    pub spr_boomerang: gfxSprite,
    pub spr_shell: gfxSprite,
    pub spr_shelldead: gfxSprite,
    pub spr_blueblock: gfxSprite,
    pub spr_spring: gfxSprite,
    pub spr_spike: gfxSprite,
    pub spr_bomb: gfxSprite,
    pub spr_kuriboshoe: gfxSprite,
    pub spr_throwbox: gfxSprite,
    pub spr_sledgehammer: gfxSprite,
    pub spr_superfireball: gfxSprite,
    pub spr_hazard_fireball: [gfxSprite; 3],
    pub spr_hazard_rotodisc: [gfxSprite; 3],
    pub spr_hazard_bulletbill: [gfxSprite; 3],
    pub spr_hazard_bulletbilldead: gfxSprite,
    pub spr_hazard_flame: [gfxSprite; 3],
    pub spr_hazard_pirhanaplant: [gfxSprite; 3],
    pub spr_fireballexplosion: gfxSprite,
    pub spr_frictionsmoke: gfxSprite,
    pub spr_bobombsmoke: gfxSprite,
    pub spr_explosion: gfxSprite,
    pub spr_burnup: gfxSprite,
    pub spr_fireworks: gfxSprite,
    pub spr_poof: gfxSprite,
    pub spr_spawnsmoke: gfxSprite,
    pub spr_spawndoor: gfxSprite,
    pub spr_bonus: gfxSprite,
    pub spr_extralife: gfxSprite,
    pub spr_award: gfxSprite,
    pub spr_awardsolid: gfxSprite,
    pub spr_awardsouls: gfxSprite,
    pub spr_awardsoulspawn: gfxSprite,
    pub spr_awardkillsinrow: gfxSprite,
    pub spr_flagbases: gfxSprite,
    pub spr_ownedtags: gfxSprite,
    pub spr_phanto: gfxSprite,
    pub spr_phantokey: gfxSprite,
    pub spr_storedpowerupsmall: gfxSprite,
    pub spr_storedpoweruplarge: gfxSprite,
    pub spr_powerupselector: gfxSprite,
    pub spr_scoreboard: gfxSprite,
    pub spr_abovearrows: gfxSprite,
    pub spr_windmeter: gfxSprite,
    pub spr_overlayhole: gfxSprite,
    pub spr_tiletypes: gfxSprite,
    pub spr_transparenttiles: gfxSprite,
    pub spr_backgroundlevel: gfxSprite,
    pub spr_tilesetlevel: gfxSprite,
    pub spr_eyecandy: gfxSprite,
    pub spr_warps: [gfxSprite; 3],
    pub spr_selectedtile: gfxSprite,
    pub spr_nospawntile: gfxSprite,
    pub spr_noitemspawntile: gfxSprite,
    pub spr_mapitems: [gfxSprite; 3],
    pub spr_powerups: gfxSprite,
    pub spr_hidden_marker: gfxSprite,
    pub spr_racegoals: gfxSprite,
    pub spr_number_icons: gfxSprite,
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
    pub _alias: Aliased,
}

impl CResourceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_menu_skin(&mut self, playerID: i16, skinID: i16, colorID: i16, fLoadBothDirections: bool) -> bool {
        let path = unsafe { skinlist.at(skinID as usize).path.clone() };
        self.load_menu_skin_path(playerID, &path, colorID, fLoadBothDirections)
    }

    pub fn load_menu_skin_path(&mut self, playerID: i16, path: &Path, colorID: i16, fLoadBothDirections: bool) -> bool {
        let p = playerID as usize;
        let new_info = LoadedSpriteInfo { path: path.to_path_buf(), colorScheme: colorID };
        // GSMenu's LoadFullSkin replaces spr_player without updating this cache, as in C++.
        if self.loaded_player_sprites[p] == new_info {
            let both_dirs_present = !self.spr_player[p][PGFX_STANDING_L as usize].get_surface().is_null()
                && !self.spr_player[p][PGFX_RUNNING_L as usize].get_surface().is_null();
            let already_loaded = !fLoadBothDirections || both_dirs_present;
            if already_loaded {
                return true;
            }
        }

        match gfx_loadmenuskin(path, colorID, fLoadBothDirections) {
            Ok(strip) => {
                self.spr_player[p] = strip;
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
    pub fn load_full_skin_path(&self, path: &Path, colorID: i16) -> SpriteStrip {
        gfx_loadfullskin(path, colorID).unwrap_or_else(|what| panic!("{}", what))
    }

    pub fn load_full_skin(&self, skinID: i16, colorID: i16) -> SpriteStrip {
        let path = unsafe { skinlist.at(skinID as usize).path.clone() };
        gfx_loadfullskin(&path, colorID).unwrap_or_else(|what| panic!("{}", what))
    }

    fn load_all_sprites(&mut self) {
        let graphicspack = unsafe { gamegraphicspacklist.current_path().to_string_lossy().into_owned() };
        let builder = |relpath: &str| SpriteBuilder::new(convert_path_pack(relpath, &graphicspack));

        let mut shyguyPath = convert_path_pack("gfx/packs/modeskins/shyguy.png", &graphicspack);
        if !file_exists(&shyguyPath) {
            shyguyPath = convert_path_pack("gfx/packs/modeskins/shyguy.bmp", &graphicspack);
        }

        let mut chickenPath = convert_path_pack("gfx/packs/modeskins/chicken.png", &graphicspack);
        if !file_exists(&chickenPath) {
            chickenPath = convert_path_pack("gfx/packs/modeskins/chicken.bmp", &graphicspack);
        }

        let mut bobombPath = convert_path_pack("gfx/packs/modeskins/bobomb.png", &graphicspack);
        if !file_exists(&bobombPath) {
            bobombPath = convert_path_pack("gfx/packs/modeskins/bobomb.bmp", &graphicspack);
        }

        //Just load menu skins for now (just standing right sprite)
        for k in 0..MAX_PLAYERS as i16 {
            self.spr_shyguy[k as usize] = self.load_full_skin_path(Path::new(&shyguyPath), k);
            self.spr_chocobo[k as usize] = self.load_full_skin_path(Path::new(&chickenPath), k);
            self.spr_bobomb[k as usize] = self.load_full_skin_path(Path::new(&bobombPath), k);
        }

        self.menu_survival = builder("gfx/packs/modeobjects/menu_survival.png").create();
        self.menu_stomp = builder("gfx/packs/modeobjects/menu_stomp.png").create();
        self.menu_egg = builder("gfx/packs/modeobjects/menu_egg.png").create();

        self.spr_clouds = builder("gfx/packs/eyecandy/cloud.png").with_alpha(255).with_wrapping(640).create();
        self.spr_ghosts = builder("gfx/packs/eyecandy/ghost.png").with_alpha(128).with_wrapping(640).create();
        self.spr_fish = builder("gfx/packs/eyecandy/fish.png").with_alpha(128).with_wrapping(640).create();
        self.spr_leaves = builder("gfx/packs/eyecandy/leaves.png").with_wrapping(640).create();
        self.spr_snow = builder("gfx/packs/eyecandy/snow.png").with_wrapping(640).create();
        self.spr_rain = builder("gfx/packs/eyecandy/rain.png").with_wrapping(640).create();

        self.spr_noteblock = builder("gfx/packs/blocks/noteblock.png").create();
        self.spr_breakableblock = builder("gfx/packs/blocks/breakableblock.png").create();
        self.spr_powerupblock = builder("gfx/packs/blocks/powerupblock.png").create();
        self.spr_donutblock = builder("gfx/packs/blocks/donutblock.png").create();
        self.spr_flipblock = builder("gfx/packs/blocks/flipblock.png").create();
        self.spr_bounceblock = builder("gfx/packs/blocks/bounceblock.png").create();
        self.spr_throwblock = builder("gfx/packs/blocks/throwblock.png").create();
        self.spr_switchblocks = builder("gfx/packs/blocks/switchblock.png").create();
        self.spr_viewblock = builder("gfx/packs/blocks/viewblock.png").create();
        self.spr_weaponbreakableblock = builder("gfx/packs/blocks/weaponbreakableblock.png").create();

        self.spr_spring = builder("gfx/packs/powerups/spring.png").with_wrapping(640).create();
        self.spr_spike = builder("gfx/packs/powerups/spike.png").with_wrapping(640).create();
        self.spr_kuriboshoe = builder("gfx/packs/powerups/kuriboshoe.png").with_wrapping(640).create();
        self.spr_throwbox = builder("gfx/packs/powerups/throwbox.png").with_wrapping(640).create();

        self.spr_tileanimation[0] = builder("gfx/packs/tilesets/tile_animation.png").create();
        self.spr_tileanimation[1] = builder("gfx/packs/tilesets/tile_animation_preview.png").create();
        self.spr_tileanimation[2] = builder("gfx/packs/tilesets/tile_animation_thumbnail.png").create();

        self.spr_blocks[0] = builder("gfx/packs/tilesets/blocks.png").create();
        self.spr_blocks[1] = builder("gfx/packs/tilesets/blocks_preview.png").create();
        self.spr_blocks[2] = builder("gfx/packs/tilesets/blocks_thumbnail.png").create();

        self.spr_unknowntile[0] = builder("gfx/packs/tilesets/unknown_tile.png").create();
        self.spr_unknowntile[1] = builder("gfx/packs/tilesets/unknown_tile_preview.png").create();
        self.spr_unknowntile[2] = builder("gfx/packs/tilesets/unknown_tile_thumbnail.png").create();

        self.spr_brokenyellowblock = builder("gfx/packs/eyecandy/brokenyellowblock.png").with_wrapping(640).create();
        self.spr_brokenflipblock = builder("gfx/packs/eyecandy/brokenflipblock.png").with_wrapping(640).create();
        self.spr_brokenblueblock = builder("gfx/packs/eyecandy/brokenblueblock.png").with_wrapping(640).create();
        self.spr_brokengrayblock = builder("gfx/packs/eyecandy/brokengrayblock.png").with_wrapping(640).create();

        self.spr_brokeniceblock = builder("gfx/packs/eyecandy/icecube.png").with_wrapping(640).create();
        self.spr_iceblock = builder("gfx/packs/eyecandy/iceblock.png").with_wrapping(640).create();

        self.spr_tanooki = builder("gfx/packs/powerups/tanooki.png").with_wrapping(640).create();
        self.spr_statue = builder("gfx/packs/projectiles/statue.png").with_wrapping(640).create();
        self.spr_starpowerup = builder("gfx/packs/powerups/starpowerup.png").with_wrapping(640).create();
        self.spr_1uppowerup = builder("gfx/packs/powerups/1uppowerup.png").with_wrapping(640).create();
        self.spr_2uppowerup = builder("gfx/packs/powerups/2uppowerup.png").with_wrapping(640).create();
        self.spr_3uppowerup = builder("gfx/packs/powerups/3uppowerup.png").with_wrapping(640).create();
        self.spr_5uppowerup = builder("gfx/packs/powerups/5uppowerup.png").with_wrapping(640).create();
        self.spr_firepowerup = builder("gfx/packs/powerups/fireflower.png").with_wrapping(640).create();
        self.spr_hammerpowerup = builder("gfx/packs/powerups/hammerpowerup.png").with_wrapping(640).create();
        self.spr_icewandpowerup = builder("gfx/packs/powerups/icewandpowerup.png").with_wrapping(640).create();
        self.spr_podobopowerup = builder("gfx/packs/powerups/podobopowerup.png").with_wrapping(640).create();
        self.spr_poisonpowerup = builder("gfx/packs/powerups/poisonpowerup.png").with_wrapping(640).create();
        self.spr_mysterymushroompowerup = builder("gfx/packs/powerups/mysterymushroom.png").with_wrapping(640).create();
        self.spr_boomerangpowerup = builder("gfx/packs/powerups/boomerangpowerup.png").with_wrapping(640).create();
        self.spr_clockpowerup = builder("gfx/packs/powerups/clockpowerup.png").with_wrapping(640).create();
        self.spr_bobombpowerup = builder("gfx/packs/powerups/bobombpowerup.png").with_wrapping(640).create();
        self.spr_powpowerup = builder("gfx/packs/powerups/powpowerup.png").with_wrapping(640).create();
        self.spr_modpowerup = builder("gfx/packs/powerups/modpowerup.png").with_wrapping(640).create();
        self.spr_bulletbillpowerup = builder("gfx/packs/powerups/bulletbillpowerup.png").with_wrapping(640).create();
        self.spr_featherpowerup = builder("gfx/packs/powerups/featherpowerup.png").with_wrapping(640).create();
        self.spr_leafpowerup = builder("gfx/packs/powerups/leafpowerup.png").with_wrapping(640).create();
        self.spr_bombpowerup = builder("gfx/packs/powerups/bombpowerup.png").with_wrapping(640).create();
        self.spr_pwingspowerup = builder("gfx/packs/powerups/pwings.png").with_wrapping(640).create();

        self.spr_extraheartpowerup = builder("gfx/packs/powerups/heartpowerup.png").with_wrapping(640).create();
        self.spr_extratimepowerup = builder("gfx/packs/powerups/extratimepowerup.png").with_wrapping(640).create();
        self.spr_jailkeypowerup = builder("gfx/packs/powerups/jailkeypowerup.png").with_wrapping(640).create();

        self.spr_secret1 = builder("gfx/packs/powerups/secret1.png").with_wrapping(640).create();
        self.spr_secret2 = builder("gfx/packs/powerups/secret2.png").with_wrapping(640).create();
        self.spr_secret3 = builder("gfx/packs/powerups/secret3.png").with_wrapping(640).create();
        self.spr_secret4 = builder("gfx/packs/powerups/secret4.png").with_wrapping(640).create();

        self.spr_shade[0] = builder("gfx/packs/eyecandy/shade1.png").with_alpha(64).create();
        self.spr_shade[1] = builder("gfx/packs/eyecandy/shade2.png").with_alpha(64).create();
        self.spr_shade[2] = builder("gfx/packs/eyecandy/shade3.png").with_alpha(64).create();
        self.spr_scorehearts = builder("gfx/packs/menu/score_hearts.png").create();
        self.spr_scorecards = builder("gfx/packs/menu/score_cards.png").create();
        self.spr_scorecoins = builder("gfx/packs/menu/score_coins.png").create();

        self.spr_timershade = builder("gfx/packs/eyecandy/timershade.png").with_alpha(64).create();
        self.spr_scoretext = builder("gfx/packs/fonts/score.png").create();
        self.spr_racetext = builder("gfx/packs/fonts/race.png").create();

        self.spr_crown = builder("gfx/packs/eyecandy/crown.png").with_wrapping(640).create();
        self.spr_cape = builder("gfx/packs/eyecandy/cape.png").with_wrapping(640).create();
        self.spr_tail = builder("gfx/packs/eyecandy/tail.png").with_wrapping(640).create();
        self.spr_wings = builder("gfx/packs/eyecandy/wings.png").with_wrapping(640).create();

        self.spr_warplock = builder("gfx/packs/eyecandy/warplock.png").create();
        self.spr_coinsparkle = builder("gfx/packs/eyecandy/coinsparks.png").with_wrapping(640).create();
        self.spr_shinesparkle = builder("gfx/packs/eyecandy/shinesparks.png").with_wrapping(640).create();
        self.spr_shellbounce = builder("gfx/packs/eyecandy/shellbounce.png").with_wrapping(640).create();
        self.spr_superstomp = builder("gfx/packs/eyecandy/supersmash.png").with_wrapping(640).create();

        self.spr_egg = builder("gfx/packs/modeobjects/egg.png").with_wrapping(640).create();
        self.spr_eggnumbers = builder("gfx/packs/modeobjects/eggnumbers.png").with_wrapping(640).create();
        self.spr_star = builder("gfx/packs/modeobjects/star.png").with_wrapping(640).create();
        self.spr_flags = builder("gfx/packs/modeobjects/flags.png").with_wrapping(640).create();
        self.spr_frenzycards = builder("gfx/packs/modeobjects/frenzycards.png").with_wrapping(640).create();
        self.spr_collectcards = builder("gfx/packs/modeobjects/collectcards.png").with_wrapping(640).create();

        self.spr_yoshi = builder("gfx/packs/modeobjects/yoshi.png").with_wrapping(640).create();
        self.spr_coin = builder("gfx/packs/modeobjects/coin.png").with_wrapping(640).create();
        self.spr_thwomp = builder("gfx/packs/modeobjects/thwomp.png").with_wrapping(640).create();
        self.spr_podobo = builder("gfx/packs/modeobjects/podobo.png").create();
        self.spr_bowserfire = builder("gfx/packs/modeobjects/bowserfire.png").create();
        self.spr_areas = builder("gfx/packs/modeobjects/areas.png").create();
        self.spr_kingofthehillarea = builder("gfx/packs/modeobjects/kingofthehill.png").with_alpha(128).create();
        self.spr_jail = builder("gfx/packs/modeobjects/jail.png").with_alpha(160).with_wrapping(640).create();
        self.spr_goomba = builder("gfx/packs/modeobjects/goomba.png").with_wrapping(640).create();
        self.spr_goombadead = builder("gfx/packs/eyecandy/goombadead.png").with_wrapping(640).create();
        self.spr_goombadeadflying = builder("gfx/packs/eyecandy/goombadeadflying.png").with_wrapping(640).create();
        self.spr_koopa = builder("gfx/packs/modeobjects/koopa.png").with_wrapping(640).create();
        self.spr_buzzybeetle = builder("gfx/packs/modeobjects/buzzybeetle.png").with_wrapping(640).create();
        self.spr_spiny = builder("gfx/packs/modeobjects/spiny.png").with_wrapping(640).create();
        self.spr_paragoomba = builder("gfx/packs/modeobjects/paragoomba.png").with_wrapping(640).create();
        self.spr_parakoopa = builder("gfx/packs/modeobjects/parakoopa.png").with_wrapping(640).create();
        self.spr_redparakoopa = builder("gfx/packs/modeobjects/redparakoopa.png").with_wrapping(640).create();
        self.spr_redkoopa = builder("gfx/packs/modeobjects/redkoopa.png").with_wrapping(640).create();
        self.spr_cheepcheep = builder("gfx/packs/modeobjects/cheepcheep.png").with_wrapping(640).create();
        self.spr_cheepcheepdead = builder("gfx/packs/eyecandy/cheepcheepdead.png").with_wrapping(640).create();

        self.spr_sledgebrothers = builder("gfx/packs/modeobjects/sledgebrothers.png").with_wrapping(640).create();
        self.spr_sledgebrothersdead = builder("gfx/packs/eyecandy/sledgebrothersdead.png").with_wrapping(640).create();

        self.spr_bulletbill = builder("gfx/packs/projectiles/bulletbill.png").create();
        self.spr_bulletbilldead = builder("gfx/packs/eyecandy/bulletbilldead.png").create();
        self.spr_chicken = builder("gfx/packs/modeobjects/chicken.png").with_alpha(160).with_wrapping(640).create();
        self.spr_racegoal = builder("gfx/packs/modeobjects/racegoal.png").create();
        self.spr_pipegamebonus = builder("gfx/packs/modeobjects/pipeminigamebonuses.png").with_wrapping(640).create();

        self.spr_phanto = builder("gfx/packs/modeobjects/phanto.png").with_wrapping(640).create();
        self.spr_phantokey = builder("gfx/packs/modeobjects/key.png").with_wrapping(640).create();

        self.spr_bonuschest = builder("gfx/packs/modeobjects/bonuschest.png").with_wrapping(640).create();
        self.spr_teleportstar = builder("gfx/packs/eyecandy/teleportstar.png").create();

        self.spr_fireball = builder("gfx/packs/projectiles/fireball.png").with_wrapping(640).create();
        self.spr_hammer = builder("gfx/packs/projectiles/hammer.png").with_wrapping(640).create();
        self.spr_iceblast = builder("gfx/packs/projectiles/wandblast.png").with_wrapping(640).create();
        self.spr_boomerang = builder("gfx/packs/projectiles/boomerang.png").with_wrapping(640).create();
        self.spr_shell = builder("gfx/packs/projectiles/shell.png").with_wrapping(640).create();
        self.spr_shelldead = builder("gfx/packs/eyecandy/shelldead.png").with_wrapping(640).create();
        self.spr_blueblock = builder("gfx/packs/projectiles/throwblock.png").with_wrapping(640).create();
        self.spr_bomb = builder("gfx/packs/projectiles/bomb.png").with_wrapping(640).create();

        self.spr_superfireball = builder("gfx/packs/modeobjects/superfire.png").with_wrapping(640).create();
        self.spr_sledgehammer = builder("gfx/packs/modeobjects/sledgehammer.png").with_wrapping(640).create();

        self.spr_hazard_fireball[0] = builder("gfx/packs/hazards/fireball.png").with_wrapping(640).create();
        self.spr_hazard_fireball[1] = builder("gfx/packs/hazards/fireball_preview.png").with_wrapping(640).create();
        self.spr_hazard_fireball[2] = builder("gfx/packs/hazards/fireball_thumbnail.png").with_wrapping(640).create();

        self.spr_hazard_rotodisc[0] = builder("gfx/packs/hazards/rotodisc.png").with_wrapping(640).create();
        self.spr_hazard_rotodisc[1] = builder("gfx/packs/hazards/rotodisc_preview.png").with_wrapping(640).create();
        self.spr_hazard_rotodisc[2] = builder("gfx/packs/hazards/rotodisc_thumbnail.png").with_wrapping(640).create();

        self.spr_hazard_bulletbill[0] = builder("gfx/packs/hazards/bulletbill.png").create();
        self.spr_hazard_bulletbill[1] = builder("gfx/packs/hazards/bulletbill_preview.png").create();
        self.spr_hazard_bulletbill[2] = builder("gfx/packs/hazards/bulletbill_thumbnail.png").create();

        self.spr_hazard_flame[0] = builder("gfx/packs/hazards/flame.png").with_wrapping(640).create();
        self.spr_hazard_flame[1] = builder("gfx/packs/hazards/flame_preview.png").with_wrapping(640).create();
        self.spr_hazard_flame[2] = builder("gfx/packs/hazards/flame_thumbnail.png").with_wrapping(640).create();

        self.spr_hazard_pirhanaplant[0] = builder("gfx/packs/hazards/pirhanaplant.png").with_wrapping(640).create();
        self.spr_hazard_pirhanaplant[1] = builder("gfx/packs/hazards/pirhanaplant_preview.png").with_wrapping(640).create();
        self.spr_hazard_pirhanaplant[2] = builder("gfx/packs/hazards/pirhanaplant_thumbnail.png").with_wrapping(640).create();

        self.spr_hazard_bulletbilldead = builder("gfx/packs/hazards/bulletbilldead.png").create();

        self.spr_fireballexplosion = builder("gfx/packs/eyecandy/fireballexplosion.png").with_alpha(160).with_wrapping(640).create();
        self.spr_frictionsmoke = builder("gfx/packs/eyecandy/frictionsmoke.png").with_alpha(160).with_wrapping(640).create();
        self.spr_bobombsmoke = builder("gfx/packs/eyecandy/bobombsmoke.png").with_alpha(160).with_wrapping(640).create();
        self.spr_explosion = builder("gfx/packs/eyecandy/explosion.png").with_wrapping(640).create();
        self.spr_burnup = builder("gfx/packs/eyecandy/burnup.png").with_alpha(192).with_wrapping(640).create();
        self.spr_fireworks = builder("gfx/packs/eyecandy/fireworks.png").with_wrapping(640).create();
        self.spr_poof = builder("gfx/packs/eyecandy/poof.png").with_wrapping(640).create();

        self.spr_spawnsmoke = builder("gfx/packs/eyecandy/spawnsmoke.png").with_alpha(128).with_wrapping(640).create();
        self.spr_spawndoor = builder("gfx/packs/eyecandy/spawndoor.png").with_wrapping(640).create();

        self.spr_bonus = builder("gfx/packs/eyecandy/bonus.png").with_wrapping(640).create();
        self.spr_extralife = builder("gfx/packs/eyecandy/extralife.png").with_wrapping(640).create();

        self.spr_windmeter = builder("gfx/packs/eyecandy/wind_meter.png").with_alpha(192).with_wrapping(640).create();
        self.spr_overlayhole = builder("gfx/packs/eyecandy/overlayholes.png").with_color_key(RGB { r: 0, g: 255, b: 0 }).with_wrapping(640).create();

        self.spr_award = builder("gfx/packs/awards/killsinrow.png").with_alpha(128).with_wrapping(640).create();
        self.spr_awardsolid = builder("gfx/packs/awards/killsinrow.png").with_wrapping(640).create();
        self.spr_awardsouls = builder("gfx/packs/awards/souls.png").with_wrapping(640).create();
        self.spr_awardsoulspawn = builder("gfx/packs/awards/soulspawn.png").with_wrapping(640).create();

        self.spr_awardkillsinrow = builder("gfx/packs/awards/killsinrownumbers.png").with_wrapping(640).create();

        self.spr_flagbases = builder("gfx/packs/modeobjects/flagbases.png").with_alpha(160).with_wrapping(640).create();
        self.spr_ownedtags = builder("gfx/packs/modeobjects/ownedtags.png").with_alpha(160).with_wrapping(640).create();

        self.spr_storedpowerupsmall = builder("gfx/packs/powerups/small.png").with_wrapping(640).create();
        self.spr_storedpoweruplarge = builder("gfx/packs/powerups/large.png").create();
        self.spr_powerupselector = builder("gfx/packs/awards/award.png").create();

        self.spr_abovearrows = builder("gfx/packs/eyecandy/abovearrows.png").with_wrapping(640).create();
    }

    pub fn load_menu_graphics(&mut self) {
        let graphicspack = unsafe { menugraphicspacklist.current_path().to_string_lossy().into_owned() };
        let builder = |relpath: &str| SpriteBuilder::new(convert_path_pack(relpath, &graphicspack));

        self.menu_shade = builder("gfx/packs/menu/menu_shade.png").with_alpha(App::menuTransparency as u8).without_color_key().create();

        self.spr_scoreboard = builder("gfx/packs/menu/scoreboard.png").create();
        self.menu_slider_bar = builder("gfx/packs/menu/menu_slider_bar.png").create();
        self.menu_plain_field = builder("gfx/packs/menu/menu_plain_field.png").create();
        self.menu_player_select = builder("gfx/packs/menu/menu_player_select.png").create();
        self.menu_dialog = builder("gfx/packs/menu/menu_dialog.png").create();
        self.menu_map_filter = builder("gfx/packs/menu/menu_map_filter.png").create();
        self.menu_match_select = builder("gfx/packs/menu/menu_match_select.png").create();

        self.menu_verticalarrows = builder("gfx/packs/menu/menu_vertical_arrows.png").create();

        self.menu_mode_small = builder("gfx/packs/menu/menu_mode_small.png").create();
        self.menu_mode_large = builder("gfx/packs/menu/menu_mode_large.png").create();

        self.spr_dialog = builder("gfx/packs/menu/dialog.png").create();
        self.spr_dialogbutton = builder("gfx/packs/menu/dialog_button.png").create();
        self.spr_tournament_background = builder("gfx/packs/menu/tournament_background.png").create();
        self.spr_tournament_powerup_splash = builder("gfx/packs/menu/tournament_powerup_splash.png").create();
        self.spr_player_select_background = builder("gfx/packs/menu/player_select_background.png").create();
        self.spr_player_select_ready = builder("gfx/packs/menu/player_select_ready.png").create();
        //spr_ipfield = builder("gfx/packs/menu/menu_ipfield.png").create();
        self.spr_selectfield = builder("gfx/packs/menu/menu_selectfield.png").create();
        self.spr_selectfielddisabled = builder("gfx/packs/menu/menu_selectfield_disabled.png").create();
        self.spr_map_filter_icons = builder("gfx/packs/menu/menu_map_flags.png").create();
        self.spr_tour_markers = builder("gfx/packs/menu/tour_markers.png").create();
        self.spr_menu_boxed_numbers = builder("gfx/packs/menu/menu_boxed_numbers.png").create();
        self.spr_countdown_numbers = builder("gfx/packs/menu/game_countdown_numbers.png").create();
        self.spr_thumbnail_warps[0] = builder("gfx/packs/menu/menu_warp_preview.png").create();
        self.spr_thumbnail_warps[1] = builder("gfx/packs/menu/menu_warp_thumbnail.png").create();
        self.spr_thumbnail_mapitems[0] = builder("gfx/packs/menu/menu_mapitems_preview.png").create();
        self.spr_thumbnail_mapitems[1] = builder("gfx/packs/menu/menu_mapitems_thumbnail.png").create();

        self.spr_announcementicons = builder("gfx/packs/menu/menu_announcement_icons.png").create();

        self.spr_platformstarttile = builder("gfx/leveleditor/leveleditor_platformstarttile.png").with_alpha(64).with_wrapping(640).create();
        self.spr_platformendtile = builder("gfx/leveleditor/leveleditor_selectedtile.png").with_alpha(64).with_wrapping(640).create();
        self.spr_platformpath = builder("gfx/leveleditor/leveleditor_platform_path.png").with_alpha(128).with_wrapping(640).create();
    }

    pub fn load_world_graphics(&mut self) {
        let graphicspack = unsafe { worldgraphicspacklist.current_path().to_string_lossy().into_owned() };
        let builder = |relpath: &str| SpriteBuilder::new(convert_path_pack(relpath, &graphicspack));

        self.spr_worldbackground[0] = builder("gfx/packs/world/world_background.png").create();
        self.spr_worldbackground[1] = builder("gfx/packs/world/preview/world_background.png").create();

        self.spr_worldforeground[0] = builder("gfx/packs/world/world_foreground.png").create();
        self.spr_worldforeground[1] = builder("gfx/packs/world/preview/world_foreground.png").create();

        self.spr_worldforegroundspecial[0] = builder("gfx/packs/world/world_foreground_special.png").create();
        self.spr_worldforegroundspecial[1] = builder("gfx/packs/world/preview/world_foreground_special.png").create();

        self.spr_worldpaths[0] = builder("gfx/packs/world/world_paths.png").create();
        self.spr_worldpaths[1] = builder("gfx/packs/world/preview/world_paths.png").create();

        self.spr_worldvehicle[0] = builder("gfx/packs/world/world_vehicles.png").create();
        self.spr_worldvehicle[1] = builder("gfx/packs/world/preview/world_vehicles.png").create();

        self.spr_worlditems = builder("gfx/packs/world/world_powerups.png").create();
        self.spr_worlditempopup = builder("gfx/packs/world/world_item_popup.png").create();
        self.spr_worlditemssmall = builder("gfx/packs/world/world_powerupssmall.png").create();
        self.spr_worlditemsplace = builder("gfx/packs/world/world_bonusplace.png").create();
        self.spr_worldbonushouse = builder("gfx/packs/world/world_bonushouse.png").create();
    }

    pub fn load_game_graphics(&mut self) {
        let graphicspack = unsafe { gamegraphicspacklist.current_path().to_string_lossy().into_owned() };

        unsafe { g_tilesetmanager.init(&graphicspack) };

        self.game_font_small = gfxFont::from_path(&convert_path_pack("gfx/packs/fonts/font_small.png", &graphicspack));
        self.game_font_large = gfxFont::from_path(&convert_path_pack("gfx/packs/fonts/font_large.png", &graphicspack));

        self.load_all_sprites();
    }

    pub fn load_start_graphics(&mut self) {
        let graphicspack = unsafe { menugraphicspacklist.current_path().to_string_lossy().into_owned() };
        let builder = |relpath: &str| SpriteBuilder::new(convert_path_pack(relpath, &graphicspack));

        self.menu_font_small = gfxFont::from_path(&convert_path_pack("gfx/packs/menu/menu_font_small.png", &graphicspack));
        self.menu_font_large = gfxFont::from_path(&convert_path_pack("gfx/packs/menu/menu_font_large.png", &graphicspack));

        //load basic stuff
        self.menu_backdrop = builder("gfx/packs/menu/menu_background.png").without_color_key().create();
        self.menu_smw = builder("gfx/packs/menu/menu_smw.png").create();
        self.menu_version = builder("gfx/packs/menu/menu_version.png").create();
    }

    pub fn load_all_graphics(&mut self) {
        let graphicspack = unsafe { gamegraphicspacklist.current_path().to_string_lossy().into_owned() };
        let builder = |relpath: &str| SpriteBuilder::new(convert_path_pack(relpath, &graphicspack));

        self.load_menu_graphics();
        self.load_world_graphics();
        self.load_game_graphics();

        self.spr_backmap[0] = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create();
        self.spr_backmap[1] = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create();
        self.spr_frontmap[0] = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create();
        self.spr_frontmap[1] = builder("gfx/packs/backgrounds/Land_Classic.png").without_color_key().create();

        self.spr_overlay = builder("gfx/packs/menu/menu_shade.png").create();
    }

    /// Panics where the C++ throws on a missing sound (the C++ callers do not catch).
    pub fn load_game_sounds(&mut self) -> bool {
        unsafe { game_values.soundcapable = false };

        if !sfx_can_play_audio() {
            return false;
        }

        let soundpack = unsafe { soundpacklist.current_path().to_string_lossy().into_owned() };
        let open = |relpath: &str| {
            sfxSound::from_file(Path::new(&convert_path_pack(relpath, &soundpack))).unwrap_or_else(|what| panic!("{}", what))
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

        unsafe { game_values.soundcapable = true };
        true
    }
}
