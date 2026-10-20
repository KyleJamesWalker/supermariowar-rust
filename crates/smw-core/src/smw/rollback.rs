//! Rollback for netplay v2 (docs/design/NETPLAY_V2.md): a snapshot of everything a match tick reads and writes, the
//! tick itself with given keys, and a state checksum. GGRS decides when to save, load and re-run.

use crate::common::eyecandy::{CEyecandyContainer, SpotlightManager};
use crate::common::game_mode::{CGameModeTrait, ModeId};
use crate::common::game_values::CGameValues;
use crate::common::input::{COutputControl, KEY_GAME_START};
use crate::common::map::CMap;
use crate::common::random_number_generator::RngState;
use crate::common::score::CScore;
use crate::common::sfx::SfxState;
use crate::game::Game;
use crate::smw::gamemodes::{bonus_house::CGM_Bonus, mini_boss::CGM_Boss_MiniGame, mini_boxes::CGM_Boxes_MiniGame, mini_pipe::CGM_Pipe_MiniGame};
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::harness;
use crate::smw::object_container::{CObjectContainer, ObjectId};
use crate::smw::players::{PlayerId, Players};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One player's keys for one tick: `down` at bit 2k and `pressed` at bit 2k+1 for key k.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub struct Keys(pub u16);

impl Keys {
    /// Packs the keys of one input slot.
    pub fn of(control: &COutputControl) -> Keys {
        let mut bits = 0u16;
        for (k, key) in control.keys.iter().enumerate() {
            bits |= (key.fDown as u16) << (2 * k) | (key.fPressed as u16) << (2 * k + 1);
        }
        Keys(bits)
    }

    /// Whether key `k` went down this tick.
    pub fn pressed(self, k: usize) -> bool {
        self.0 & 1 << (2 * k + 1) != 0
    }

    /// Writes these keys into one input slot.
    pub fn apply(self, control: &mut COutputControl) {
        for (k, key) in control.keys.iter_mut().enumerate() {
            key.fDown = self.0 & 1 << (2 * k) != 0;
            key.fPressed = self.0 & 1 << (2 * k + 1) != 0;
        }
    }
}

/// Where a player's keys go: a local input slot, or a remote player's slot (net games' `PlayerKeys`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeySlot {
    /// An index into `game_values.playerInput.outputControls`.
    Local(usize),
    /// An index into `netplay.netPlayerInput.outputControls`, by the remote player's slot.
    Net(usize),
}

/// Everything a match tick changes. Loading writes into the live allocations, which other code points into.
#[derive(Clone)]
pub struct MatchState {
    game_values: CGameValues,
    g_map: CMap,
    noncolcontainer: CObjectContainer,
    objectcontainer: [CObjectContainer; 3],
    eyecandy: [CEyecandyContainer; 3],
    players: Players,
    score: [CScore; 4],
    score_cnt: i16,
    mode: Option<ModeState>,
    currentgamemode: i16,
    stats_page: bool,
    rng: RngState,
    sfx: SfxState,
    x_shake: i16,
    y_shake: i16,
    shakeleft: bool,
    swirl_spawns: [[[i16; 25]; 2]; 4],
    spotlights: SpotlightManager,
    winning_player: i16,
    draw_index: i16,
    gps: Option<GameplayState>,
    /// The harness frame counter, which the virtual sound clock and [`checksum`] read.
    frame: u32,
}

/// The game mode a match plays, the only one its ticks change.
#[derive(Clone)]
enum ModeState {
    Standard(usize, Box<dyn CGameModeTrait>),
    BonusHouse(CGM_Bonus),
    Pipe(CGM_Pipe_MiniGame),
    Boss(CGM_Boss_MiniGame),
    Boxes(CGM_Boxes_MiniGame),
}

impl Clone for Box<dyn CGameModeTrait> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Copies the live match into a [`MatchState`]; the screen surface and display lists stay out.
pub fn save(g: &mut Game) -> MatchState {
    MatchState {
        game_values: (*g.game_values).clone(),
        g_map: (*g.g_map).clone(),
        noncolcontainer: g.noncolcontainer.clone(),
        objectcontainer: g.objectcontainer.clone(),
        eyecandy: g.eyecandy.clone(),
        players: g.players.clone(),
        score: std::array::from_fn(|i| (*g.score[i]).clone()),
        score_cnt: g.score_cnt,
        mode: match g.game_values.gamemode {
            None => None,
            Some(ModeId::Standard(i)) => g.gamemodes[i as usize].is_set().then(|| ModeState::Standard(i as usize, g.gamemodes[i as usize].clone_box())),
            Some(ModeId::BonusHouse) => g.bonushousemode.is_set().then(|| ModeState::BonusHouse((*g.bonushousemode).clone())),
            Some(ModeId::Pipe) => g.pipegamemode.is_set().then(|| ModeState::Pipe((*g.pipegamemode).clone())),
            Some(ModeId::Boss) => g.bossgamemode.is_set().then(|| ModeState::Boss((*g.bossgamemode).clone())),
            Some(ModeId::Boxes) => g.boxesgamemode.is_set().then(|| ModeState::Boxes((*g.boxesgamemode).clone())),
        },
        currentgamemode: g.currentgamemode,
        stats_page: g.main_loop.stats_page,
        rng: g.rng.clone(),
        sfx: g.sfx.clone(),
        x_shake: g.x_shake,
        y_shake: g.y_shake,
        shakeleft: g.shakeleft,
        swirl_spawns: g.g_iSwirlSpawnLocations,
        spotlights: (*g.spotlightManager).clone(),
        winning_player: g.g_iWinningPlayer,
        draw_index: g.g_iCurrentDrawIndex,
        gps: g.gps.clone(),
        frame: harness::frame(g),
    }
}

/// Puts `s` back. Call it between ticks, never from inside the gameplay state's own update. Players and objects are
/// copied over the live ones of the same type, which keeps their allocations.
pub fn load(g: &mut Game, s: &MatchState) {
    *g.game_values = s.game_values.clone();
    *g.g_map = s.g_map.clone();
    g.noncolcontainer.clone_from(&s.noncolcontainer);
    for (live, saved) in g.objectcontainer.iter_mut().zip(&s.objectcontainer) {
        live.clone_from(saved);
    }
    g.eyecandy = s.eyecandy.clone();
    g.players.clone_from(&s.players);
    for (live, saved) in g.score.iter_mut().zip(&s.score) {
        **live = saved.clone();
    }
    g.score_cnt = s.score_cnt;
    match &s.mode {
        None => {}
        Some(ModeState::Standard(i, mode)) => g.gamemodes[*i].set(mode.clone_box()),
        Some(ModeState::BonusHouse(mode)) => *g.bonushousemode = mode.clone(),
        Some(ModeState::Pipe(mode)) => *g.pipegamemode = mode.clone(),
        Some(ModeState::Boss(mode)) => *g.bossgamemode = mode.clone(),
        Some(ModeState::Boxes(mode)) => *g.boxesgamemode = mode.clone(),
    }
    g.currentgamemode = s.currentgamemode;
    g.main_loop.stats_page = s.stats_page;
    g.rng = s.rng.clone();
    let audible = g.sfx.sfx_audible;
    g.sfx = s.sfx.clone();
    g.sfx.sfx_audible = audible;
    g.x_shake = s.x_shake;
    g.y_shake = s.y_shake;
    g.shakeleft = s.shakeleft;
    g.g_iSwirlSpawnLocations = s.swirl_spawns;
    *g.spotlightManager = s.spotlights.clone();
    g.g_iWinningPlayer = s.winning_player;
    g.g_iCurrentDrawIndex = s.draw_index;
    g.gps = s.gps.clone();
    harness::set_frame(g, s.frame);
}

/// One match tick with each player's keys in place; slots not listed keep theirs (a CPU player's).
/// A `rerun` after a rollback plays no sound and rasterizes nothing. True when the match ended or restarted.
pub fn advance(g: &mut Game, keys: &[(KeySlot, Keys)], rerun: bool) -> bool {
    let mut gp = g.ptr();
    crate::common::sfx::sfx_virtual_advance(g);
    for &(slot, k) in keys {
        match slot {
            KeySlot::Local(i) => k.apply(&mut g.game_values.playerInput.outputControls[i]),
            KeySlot::Net(i) => k.apply(&mut g.netplay.netPlayerInput.outputControls[i]),
        }
    }
    let audible = g.sfx.sfx_audible;
    if rerun {
        g.sfx.sfx_audible = false;
    }
    let gameplay = gp.gps.as_mut().expect("rollback::advance outside a match");
    let paused = g.netplay.v2_match && shared_pause(g, keys, rerun);
    if !paused {
        gameplay.wind(g);
    }
    let changed = gameplay.simulate_tick(g, paused, rerun);
    g.sfx.sfx_audible = audible;
    harness::end_rollback_tick(g);
    changed
}

/// A v2 match's pause, the same on every peer at the same tick: any player's start key pauses, and only that
/// player's start key resumes. True while paused.
fn shared_pause(g: &mut Game, keys: &[(KeySlot, Keys)], silent: bool) -> bool {
    let flags = &g.game_values.flags;
    let toggler = if flags.pausegame {
        usize::try_from(flags.pauseowner).ok().filter(|&p| keys.get(p).is_some_and(|(_, k)| k.pressed(KEY_GAME_START)))
    } else {
        keys.iter().position(|(_, k)| k.pressed(KEY_GAME_START))
    };
    if let Some(player) = toggler.filter(|_| !flags.noexit && !g.mode().gameover) {
        let flags = &mut g.game_values.flags;
        flags.pausegame = !flags.pausegame;
        flags.pauseowner = if flags.pausegame { player as i16 } else { -1 };
        if !silent {
            let mut gp = g.ptr();
            crate::common::game_values::if_sound_on_play(g, &mut gp.rm.sfx_pause);
        }
    }
    g.game_values.flags.pausegame
}

/// FNV-1a over the match's harness dump (game mode, players, object counts) and the simulated state the dump leaves
/// out: the gameplay state and flags, the mode's own state, scores, every object and block with its type's own state,
/// every platform, the switches and the RNG. Floats go in as their bits, so every platform hashes the same state
/// alike. What a desync check compares.
pub fn checksum(g: &mut Game) -> u128 {
    let mut gp = g.ptr();
    let mut dump = Vec::new();
    if let Some(gameplay) = gp.gps.as_mut() {
        let _ = gameplay.harness_dump(g, &mut dump);
    }
    let mut h = Fnv::new();
    h.bytes(&dump);
    h.bytes(&harness::frame(g).to_le_bytes());
    if let Some(gameplay) = gp.gps.as_ref() {
        gameplay.harness_hash(&mut h);
    }
    let f = &g.game_values.flags;
    h.i16s(&[
        f.pauseowner,
        f.noexittimer,
        f.forceexittimer,
        f.teamdeadcounter,
        f.screenshaketimer,
        f.screenshakeplayerid,
        f.screenshaketeamid,
        f.screenshakekillscount,
        f.slowdownon,
        f.slowdowncounter,
        f.swapplayersblinkcount,
    ]);
    h.f32s(&[f.swapplayersposition, f.gamewindx, f.gamewindy]);
    h.bools(&[f.pausegame, f.noexit, f.screenshakekillinair, f.swapplayers, f.swapplayersblink, f.windaffectsplayers, f.spinscreen, f.reversewalk]);
    g.mode().get().harness_hash(&mut h);
    for score in &g.score[..g.score_cnt.max(0) as usize] {
        h.i16s(&[score.score]);
        h.i16s(&score.subscore);
    }
    for container in [&g.noncolcontainer, &g.objectcontainer[0], &g.objectcontainer[1], &g.objectcontainer[2]] {
        h.u32(container.list().len() as u32);
        for object in container.list() {
            let o = object.obj();
            h.u32(o.objectType as u32);
            h.u32(o.state as u32);
            h.u32(o.dead as u32);
            h.u32(o.ix as u32);
            h.u32(o.iy as u32);
            h.f32s(&[o.fx, o.fy, o.velx, o.vely]);
            let mut object = *object;
            match object.as_io_moving_object() {
                Some(moving) => moving.harness_hash(&mut h),
                None => object.harness_hash_object(&mut h),
            }
        }
    }
    for object in g.noncolcontainer.list() {
        let mut object = *object;
        if let Some(block) = object.as_io_block() {
            block.harness_hash(&mut h);
        }
    }
    let map = &g.g_map;
    for &id in map.platforms.iter().chain(&map.tempPlatforms) {
        let p = map.platform(id);
        h.u32(p.fDead as u32);
        h.f32s(&[p.fx, p.fy, p.fVelX, p.fVelY]);
    }
    for switch in map.iSwitches {
        h.u32(switch as u32);
    }
    if let Some((words, index)) = g.rng.generator_state() {
        for word in words {
            h.u32(word);
        }
        h.u32(index);
    }
    h.0 as u128
}

/// 64-bit FNV-1a.
pub struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    pub fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ *byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn i16s(&mut self, values: &[i16]) {
        for value in values {
            self.bytes(&value.to_le_bytes());
        }
    }

    pub fn bools(&mut self, values: &[bool]) {
        for value in values {
            self.bytes(&[*value as u8]);
        }
    }

    pub fn f32s(&mut self, values: &[f32]) {
        for value in values {
            self.u32(value.to_bits());
        }
    }

    pub fn player(&mut self, id: PlayerId) {
        let (slot, generation) = id.raw();
        self.bytes(&[slot, generation]);
    }

    pub fn object(&mut self, id: ObjectId) {
        let (container, slot, generation) = id.raw();
        self.bytes(&[container]);
        self.bytes(&slot.to_le_bytes());
        self.u32(generation);
    }
}

/// The GGRS session parameters of a match: a player's keys per tick, addressed by peer.
pub struct GgrsConfig<A>(std::marker::PhantomData<A>);

impl<A: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + 'static> ggrs::Config for GgrsConfig<A> {
    type Input = Keys;
    type InputPredictor = ggrs::PredictRepeatLast;
    type State = MatchState;
    type Address = A;
}

/// Carries out a session's requests. `slots[p]` is where GGRS player `p`'s keys go. With `quiet_reruns`, the ticks a
/// rollback re-runs play no sound and draw nothing; the last tick is the new one and does both. Saved frames carry a
/// checksum every `checksum_every` frames, the ones GGRS compares.
pub fn handle_requests<A>(
    g: &mut Game,
    requests: Vec<ggrs::GgrsRequest<GgrsConfig<A>>>,
    slots: &[KeySlot],
    quiet_reruns: bool,
    checksum_every: i32,
) -> bool
where
    A: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + 'static,
{
    let last_advance = requests.iter().rposition(|r| matches!(r, ggrs::GgrsRequest::AdvanceFrame { .. }));
    let mut rerun = false;
    let mut changed = false;
    for (index, request) in requests.into_iter().enumerate() {
        match request {
            ggrs::GgrsRequest::SaveGameState { cell, frame } => {
                let (state, sum) = logged_save(g, frame, checksum_every);
                cell.save(frame, Some(state), sum);
            }
            ggrs::GgrsRequest::LoadGameState { cell, .. } => {
                load(g, &cell.data().expect("GGRS loads only saved frames"));
                rerun = true;
            }
            ggrs::GgrsRequest::AdvanceFrame { inputs } => {
                let keys: Vec<(KeySlot, Keys)> = inputs.iter().enumerate().map(|(player, (k, _status))| (slots[player], *k)).collect();
                changed |= advance(g, &keys, rerun && quiet_reruns && Some(index) != last_advance);
            }
        }
    }
    changed
}

/// A GGRS request as a v2 recording keeps it (`#@ v2` lines, docs/guides/REPLAY.md).
#[derive(Clone, Debug, PartialEq)]
pub enum Req {
    /// Save the state of this GGRS frame.
    Save(i32),
    /// Load the state saved at this GGRS frame.
    Load(i32),
    /// Run one tick with these keys, one per GGRS player handle.
    Advance(Vec<Keys>),
}

impl Req {
    /// The recordable part of a live GGRS request.
    pub fn of<A>(request: &ggrs::GgrsRequest<GgrsConfig<A>>) -> Req
    where
        A: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    {
        match request {
            ggrs::GgrsRequest::SaveGameState { frame, .. } => Req::Save(*frame),
            ggrs::GgrsRequest::LoadGameState { frame, .. } => Req::Load(*frame),
            ggrs::GgrsRequest::AdvanceFrame { inputs } => Req::Advance(inputs.iter().map(|(k, _)| *k).collect()),
        }
    }

    /// `s<frame>`, `l<frame>` or `a<keys>,<keys>,...` with the keys in hex.
    pub fn token(&self) -> String {
        match self {
            Req::Save(f) => format!("s{f}"),
            Req::Load(f) => format!("l{f}"),
            Req::Advance(keys) => format!("a{}", keys.iter().map(|k| format!("{:x}", k.0)).collect::<Vec<_>>().join(",")),
        }
    }

    /// Reads a [`Req::token`]; `None` for any other token.
    pub fn parse(token: &str) -> Option<Req> {
        let (kind, rest) = token.split_at_checked(1)?;
        match kind {
            "s" => rest.parse().ok().map(Req::Save),
            "l" => rest.parse().ok().map(Req::Load),
            "a" => rest.split(',').map(|k| u16::from_str_radix(k, 16).ok().map(Keys)).collect::<Option<Vec<_>>>().map(Req::Advance),
            _ => None,
        }
    }
}

/// `handle_requests` for recorded requests: the same saves, loads and ticks, with the saved states kept in `saves`.
pub fn replay_requests(g: &mut Game, requests: &[Req], slots: &[KeySlot], saves: &mut BTreeMap<i32, MatchState>, checksum_every: i32) -> bool {
    let last_advance = requests.iter().rposition(|r| matches!(r, Req::Advance(_)));
    let mut rerun = false;
    let mut changed = false;
    for (index, request) in requests.iter().enumerate() {
        match request {
            Req::Save(frame) => {
                saves.retain(|&f, _| f > frame - 2 * MAX_SAVED_FRAMES);
                saves.insert(*frame, logged_save(g, *frame, checksum_every).0);
            }
            Req::Load(frame) => {
                let Some(state) = saves.get(frame) else { harness::fail(format!("the v2 replay loads frame {frame}, which it never saved")) };
                load(g, state);
                rerun = true;
            }
            Req::Advance(keys) => {
                let keys: Vec<(KeySlot, Keys)> = slots.iter().copied().zip(keys.iter().copied()).collect();
                changed |= advance(g, &keys, rerun && Some(index) != last_advance);
            }
        }
    }
    changed
}

/// How far back a replay keeps saved states; GGRS never loads one older than its prediction window.
const MAX_SAVED_FRAMES: i32 = 16;

/// A save for GGRS, with its checksum on every `checksum_every`-th frame, also written to SMW_V2_STATE_LOG.
fn logged_save(g: &mut Game, frame: i32, checksum_every: i32) -> (MatchState, Option<u128>) {
    let sum = (frame % checksum_every.max(1) == 0).then(|| checksum(g));
    static STATE_LOG: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    if let Some(path) = STATE_LOG.get_or_init(|| std::env::var("SMW_V2_STATE_LOG").ok()) {
        let mut gp = g.ptr();
        let mut dump = Vec::new();
        if let Some(gameplay) = gp.gps.as_mut() {
            let _ = gameplay.harness_dump(g, &mut dump);
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "#F {} hframe={} sum={:x}", frame, harness::frame(g), sum.unwrap_or(0));
            let _ = f.write_all(&dump);
        }
    }
    (save(g), sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_tokens_round_trip() {
        let requests = [Req::Save(-1), Req::Load(462), Req::Advance(vec![Keys(0), Keys(0xa2f0), Keys(0xffff)])];
        let tokens: Vec<String> = requests.iter().map(Req::token).collect();
        assert_eq!(tokens, ["s-1", "l462", "a0,a2f0,ffff"]);
        assert_eq!(tokens.iter().map(|t| Req::parse(t)).collect::<Option<Vec<_>>>().as_deref(), Some(&requests[..]));
        assert_eq!(Req::parse("w"), None);
        assert_eq!(Req::parse("ax"), None);
    }

    #[test]
    fn mode_state_changes_the_mode_hash() {
        use crate::smw::gamemodes::survival::CGM_Survival;
        let hash = |mode: &CGM_Survival| {
            let mut h = Fnv::new();
            mode.harness_hash(&mut h);
            h.0
        };
        let mut mode = CGM_Survival::new();
        let before = hash(&mode);
        assert_eq!(hash(&mode.clone()), before);
        mode.timer += 1;
        assert_ne!(hash(&mode), before);
        mode.timer -= 1;
        mode.gameover = true;
        assert_ne!(hash(&mode), before);
    }
}
