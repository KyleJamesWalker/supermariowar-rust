//! With `SMW_ROLLBACK_BENCH=1` (or a frame count, default 8), measures what rollback netplay would cost during a
//! match (docs/NETPLAY_V2.md): a `Game::clone_match` snapshot every tick, and every tenth tick a copy re-running that
//! many frames with record-only draws. Logs every 300 ticks. The copies run on headless services, so the game itself
//! is unchanged, only slower.

use crate::common::game_values::AppState;
use crate::game::Game;
use crate::smw::game_state::StateId;
use std::time::{Duration, Instant};

const DEFAULT_FRAMES: u32 = 8;
const RERUN_EVERY: u32 = 10;
const REPORT_TICKS: u32 = 300;

#[derive(Clone, Default)]
struct Stat {
    count: u32,
    total: Duration,
    worst: Duration,
}

impl Stat {
    fn add(&mut self, d: Duration) {
        self.count += 1;
        self.total += d;
        self.worst = self.worst.max(d);
    }

    fn avg_ms(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.total.as_secs_f64() * 1000.0 / self.count as f64 }
    }

    fn worst_ms(&self) -> f64 {
        self.worst.as_secs_f64() * 1000.0
    }
}

#[derive(Clone)]
pub struct RollbackBench {
    frames: u32,
    ticks: u32,
    tick: Stat,
    clone: Stat,
    drop: Stat,
    rerun: Stat,
    cut_short: u32,
}

impl RollbackBench {
    pub fn from_env() -> Option<Self> {
        let v = std::env::var("SMW_ROLLBACK_BENCH").ok().filter(|v| v != "0")?;
        let frames = v.parse::<u32>().ok().filter(|&n| n > 1).unwrap_or(DEFAULT_FRAMES);
        println!("[rollback] bench on: snapshot every tick, {}-frame re-run every {} ticks", frames, RERUN_EVERY);
        Some(Self::new(frames))
    }

    fn new(frames: u32) -> Self {
        RollbackBench { frames, ticks: 0, tick: Stat::default(), clone: Stat::default(), drop: Stat::default(), rerun: Stat::default(), cut_short: 0 }
    }

    /// After a tick that took `tick` of update and drawing.
    pub fn after_tick(&mut self, g: &mut Game, tick: Duration) {
        let gv = &g.game_values;
        if g.gsm.currentState != StateId::Gameplay || gv.appstate != AppState::Game || gv.screenfade != 0 || gv.screenfadespeed != 0 {
            return;
        }
        self.tick.add(tick);

        let start = Instant::now();
        let mut copy = g.clone_match(services());
        self.clone.add(start.elapsed());

        if self.ticks % RERUN_EVERY == 0 {
            copy.main_loop.interpolate = true;
            let start = Instant::now();
            for _ in 0..self.frames {
                crate::smw::main::step(&mut copy);
                if copy.game_values.appstate != AppState::Game || copy.gsm.currentState != StateId::Gameplay {
                    self.cut_short += 1;
                    break;
                }
            }
            self.rerun.add(start.elapsed());
        }

        let start = Instant::now();
        drop(copy);
        self.drop.add(start.elapsed());

        self.ticks += 1;
        if self.ticks == REPORT_TICKS {
            self.report();
            *self = Self::new(self.frames);
        }
    }

    fn report(&self) {
        let snapshot = self.clone.avg_ms() + self.drop.avg_ms();
        let budget = 1000.0 / crate::common::global_constants::TICKS_PER_SECOND as f64;
        println!(
            "[rollback] {} ticks: tick {:.2} ms (worst {:.2}); snapshot {:.2} ms (clone {:.2}, worst {:.2}; drop {:.2}); {}-frame re-run {:.2} ms (worst {:.2}, {:.2} per frame, {} cut short); tick+snapshot+re-run {:.2} of {:.2} ms",
            self.ticks,
            self.tick.avg_ms(),
            self.tick.worst_ms(),
            snapshot,
            self.clone.avg_ms(),
            self.clone.worst_ms(),
            self.drop.avg_ms(),
            self.frames,
            self.rerun.avg_ms(),
            self.rerun.worst_ms(),
            self.rerun.avg_ms() / self.frames as f64,
            self.cut_short,
            self.tick.avg_ms() + snapshot + self.rerun.avg_ms(),
            budget
        );
    }
}

fn services() -> smw_platform::Services {
    let dir = std::env::temp_dir().join("smw-rollback-bench");
    smw_headless::services(dir.clone(), dir)
}
