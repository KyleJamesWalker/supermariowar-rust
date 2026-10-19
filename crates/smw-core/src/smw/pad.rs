//! Not in upstream: which pads the players get, and pads plugged in or removed while the game runs. The backend
//! opens and translates them (smw_platform::Pads).

use crate::game::{Device, Game, MAX_DEVICES};
use crate::services::services;
use smw_platform::{PadChange, PadInfo};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

pub struct PadState {
    debug_input: bool,
}

impl PadState {
    pub const fn new() -> Self {
        PadState { debug_input: false }
    }
}

static GENERATION: AtomicU32 = AtomicU32::new(0);

/// Opens every connected pad unless a replay supplies its own devices; `translate` as `SMW_PAD_TRANSLATE`.
pub fn init(g: &mut Game, open: bool, translate: bool) -> Vec<Option<Device>> {
    g.pad.debug_input = std::env::var_os("SMW_DEBUG_INPUT").is_some();
    if g.pad.debug_input {
        start_watchdog();
    }
    services(g).pads.open_all(open, translate).into_iter().map(|info| info.map(device)).collect()
}

fn device(info: PadInfo) -> Device {
    Device::Pad { pad: info.pad, controller: info.controller }
}

/// In a live session, forgets removed pads and opens the ones connected since the last call. A new one
/// takes the first free index, so a pad that reconnects gets its index back. True when one was opened.
pub fn hotplug(g: &mut Game) -> bool {
    let mut added = false;
    for change in services(g).pads.hotplug() {
        match change {
            PadChange::Removed(pad) => {
                let Some(index) = g.devices.iter().position(|d| matches!(d, Some(Device::Pad { pad: p, .. }) if *p == pad)) else {
                    continue;
                };
                g.devices[index] = None;
                devices_changed();
                crate::smw::harness::record_device(g, index, false);
            }
            PadChange::Connected(id) => {
                let Some(index) = free_index(g) else {
                    continue;
                };
                if let Some(info) = services(g).pads.open(id, index) {
                    g.devices[index] = Some(device(info));
                    devices_changed();
                    crate::smw::harness::record_device(g, index, true);
                    added = true;
                }
            }
        }
    }
    added
}

/// Changes whenever the device table does.
pub fn generation() -> u32 {
    GENERATION.load(Ordering::Relaxed)
}

pub fn devices_changed() {
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// The first empty slot in `devices`, growing it by one when every slot is taken.
pub(crate) fn free_index(g: &mut Game) -> Option<usize> {
    let count = g.devices.len();
    if let Some(i) = (0..count).find(|&i| g.devices[i].is_none()) {
        return Some(i);
    }
    if count >= MAX_DEVICES {
        return None;
    }
    g.devices.push(None);
    Some(count)
}

/// The joysticks players get: the translated controllers if there are any, otherwise every open joystick.
pub fn player_pads(g: &mut Game) -> Vec<usize> {
    let controller = |d: &Option<Device>| matches!(d, Some(Device::Pad { controller: true, .. }));
    let mut open: Vec<usize> = (0..g.devices.len()).filter(|&i| g.devices[i].is_some()).collect();
    if g.devices.iter().any(controller) {
        open.retain(|&i| controller(&g.devices[i]));
    }
    open
}


/// `SMW_DEBUG_INPUT=1`: a heartbeat every 120 frames.
pub fn debug_frame(g: &mut Game, frame: u32) {
    FRAME.store(frame, Ordering::Relaxed);
    if g.pad.debug_input && frame % 120 == 0 {
        println!("[loop] frame {}", frame);
    }
}

static FRAME: AtomicU32 = AtomicU32::new(0);
static PHASE: AtomicUsize = AtomicUsize::new(0);
static BEATS: AtomicU64 = AtomicU64::new(0);
const PHASES: [&str; 8] = ["startup", "fps limiter", "sound", "recorder", "replay events", "update", "frame end", "flip"];

/// Marks the main loop's progress for the `SMW_DEBUG_INPUT` watchdog; `phase` indexes `PHASES`.
pub fn phase(phase: usize) {
    PHASE.store(phase, Ordering::Relaxed);
    BEATS.fetch_add(1, Ordering::Relaxed);
}

/// `SMW_DEBUG_INPUT=1`: reports a main loop that has not moved for 3 s.
fn start_watchdog() {
    std::thread::spawn(|| {
        let mut last = u64::MAX;
        let mut still = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let beats = BEATS.load(Ordering::Relaxed);
            if beats != last {
                last = beats;
                still = 0;
                continue;
            }
            still += 1;
            if still == 6 || (still > 6 && still % 20 == 0) {
                println!(
                    "[watchdog] main loop stalled for {} s at frame {}, phase {}",
                    still / 2,
                    FRAME.load(Ordering::Relaxed),
                    PHASES.get(PHASE.load(Ordering::Relaxed)).unwrap_or(&"?")
                );
            }
        }
    });
}

