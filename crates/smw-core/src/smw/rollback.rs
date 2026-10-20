//! Rollback for netplay v2 (docs/design/NETPLAY_V2.md): a snapshot of everything a match tick reads and writes, the
//! tick itself with given keys, and a state checksum. GGRS decides when to save, load and re-run.

use crate::common::eyecandy::{CEyecandyContainer, SpotlightManager};
use crate::common::game_mode::CGameModeTrait;
use crate::common::game_values::CGameValues;
use crate::common::input::COutputControl;
use crate::common::map::CMap;
use crate::common::random_number_generator::RngState;
use crate::common::score::CScore;
use crate::common::sfx::SfxState;
use crate::game::Game;
use crate::smw::gamemodes::{bonus_house::CGM_Bonus, mini_boss::CGM_Boss_MiniGame, mini_boxes::CGM_Boxes_MiniGame, mini_pipe::CGM_Pipe_MiniGame};
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::harness;
use crate::smw::object_container::CObjectContainer;
use crate::smw::players::Players;
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
    gamemodes: Vec<Option<Box<dyn CGameModeTrait>>>,
    bonushousemode: Option<CGM_Bonus>,
    pipegamemode: Option<CGM_Pipe_MiniGame>,
    bossgamemode: Option<CGM_Boss_MiniGame>,
    boxesgamemode: Option<CGM_Boxes_MiniGame>,
    currentgamemode: i16,
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
        gamemodes: g.gamemodes.iter().map(|m| m.is_set().then(|| m.clone_box())).collect(),
        bonushousemode: g.bonushousemode.is_set().then(|| (*g.bonushousemode).clone()),
        pipegamemode: g.pipegamemode.is_set().then(|| (*g.pipegamemode).clone()),
        bossgamemode: g.bossgamemode.is_set().then(|| (*g.bossgamemode).clone()),
        boxesgamemode: g.boxesgamemode.is_set().then(|| (*g.boxesgamemode).clone()),
        currentgamemode: g.currentgamemode,
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

/// Puts `s` back. Call it between ticks, never from inside the gameplay state's own update.
pub fn load(g: &mut Game, s: &MatchState) {
    *g.game_values = s.game_values.clone();
    *g.g_map = s.g_map.clone();
    g.noncolcontainer = s.noncolcontainer.clone();
    g.objectcontainer = s.objectcontainer.clone();
    g.eyecandy = s.eyecandy.clone();
    g.players = s.players.clone();
    for (live, saved) in g.score.iter_mut().zip(&s.score) {
        **live = saved.clone();
    }
    g.score_cnt = s.score_cnt;
    for (live, saved) in g.gamemodes.iter_mut().zip(&s.gamemodes) {
        if let Some(mode) = saved {
            live.set(mode.clone_box());
        }
    }
    if let Some(mode) = &s.bonushousemode {
        *g.bonushousemode = mode.clone();
    }
    if let Some(mode) = &s.pipegamemode {
        *g.pipegamemode = mode.clone();
    }
    if let Some(mode) = &s.bossgamemode {
        *g.bossgamemode = mode.clone();
    }
    if let Some(mode) = &s.boxesgamemode {
        *g.boxesgamemode = mode.clone();
    }
    g.currentgamemode = s.currentgamemode;
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

/// A v2 match's pause: any player's start key toggles it on every peer at the same tick. True while paused.
fn shared_pause(g: &mut Game, keys: &[(KeySlot, Keys)], silent: bool) -> bool {
    const START_PRESSED: u16 = 1 << (2 * 6 + 1);
    let flags = &mut g.game_values.flags;
    let pressed = keys.iter().any(|(_, k)| k.0 & START_PRESSED != 0);
    if pressed && !flags.noexit && !g.mode().gameover {
        let flags = &mut g.game_values.flags;
        flags.pausegame = !flags.pausegame;
        if !silent {
            let mut gp = g.ptr();
            crate::common::game_values::if_sound_on_play(g, &mut gp.rm.sfx_pause);
        }
    }
    g.game_values.flags.pausegame
}

/// FNV-1a over the match's harness dump (game mode, players, objects): what a desync check compares.
pub fn checksum(g: &mut Game) -> u128 {
    let mut gp = g.ptr();
    let mut dump = Vec::new();
    if let Some(gameplay) = gp.gps.as_mut() {
        let _ = gameplay.harness_dump(g, &mut dump);
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in dump.iter().chain(&harness::frame(g).to_le_bytes()) {
        hash = (hash ^ *byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash as u128
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
                load(g, &cell.load().expect("GGRS loads only saved frames"));
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
    if let Ok(path) = std::env::var("SMW_V2_STATE_LOG") {
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
}
