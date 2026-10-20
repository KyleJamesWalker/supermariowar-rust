//! In-process replays on the headless backend (docs/design/ARCHITECTURE_V2.md, Testing): the game runs a replay as
//! `tools/run_rust.sh` does, and its dump and screenshots must equal the goldens `tools/parity.sh` reads.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The harness reads its settings from the environment and seeds libc's `rand`, both process-wide.
static GAME: Mutex<()> = Mutex::new(());

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// The value of the last `#@ key=value` line in a replay's text.
fn directive(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    text.lines().filter_map(|l| l.strip_prefix("#@").map(str::trim_start)).filter_map(|l| l.strip_prefix(&prefix)).next_back().map(str::to_string)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn decode_base64(text: &str) -> Vec<u8> {
    smw::smw::harness::base64_decode(text.trim()).expect("base64 settings file")
}

/// Runs `replay` as `tools/run_rust.sh` does, in a sandbox under `out`, and returns the dump's text. `segment`
/// plays only that match (`SMW_SEGMENT`).
fn run(replay: &Path, out: &Path, segment: Option<u32>) -> String {
    sandboxed(replay, out, segment, |argv, services| smw::smw::main::run_with(argv, Some(services)))
}

/// Sets up `run`'s sandbox and environment, calls `game` with the arguments and services to run on, and returns the dump.
fn sandboxed(replay: &Path, out: &Path, segment: Option<u32>, game: impl FnOnce(Vec<String>, smw_platform::Services)) -> String {
    let _game = GAME.lock().unwrap_or_else(|e| e.into_inner());
    let text = smw::smw::harness::read_recording(replay).unwrap();
    let _ = std::fs::remove_dir_all(out);
    let settings = out.join("settings");
    std::fs::create_dir_all(&settings).unwrap();
    if let Some(options) = directive(&text, "options") {
        std::fs::copy(replay.parent().unwrap().join(options), settings.join("options.bin")).unwrap();
    }
    for (key, file) in [("options_b64", "options.bin"), ("controls_b64", "controls.sdl2.bin"), ("servers_b64", "servers.toml")] {
        if let Some(b64) = directive(&text, key) {
            std::fs::write(settings.join(file), decode_base64(&b64)).unwrap();
        }
    }
    let data = out.join("data");
    copy_dir(&repo().join("data"), &data);
    let _ = std::fs::remove_file(data.join("maps/cache/mapsummary.txt"));
    let dump = out.join("dump.txt");

    let vars = [
        ("SMW_SEED", directive(&text, "seed").unwrap_or_else(|| "1".into())),
        ("SMW_NOLIMIT", "1".into()),
        ("SMW_FRAMES", directive(&text, "frames").unwrap_or_else(|| "600".into())),
        ("SMW_MAP", directive(&text, "map").unwrap_or_default()),
        ("SMW_REPLAY", replay.to_string_lossy().into_owned()),
        ("SMW_DUMP", dump.to_string_lossy().into_owned()),
        ("SMW_SHOT_FRAMES", directive(&text, "shots").unwrap_or_default()),
        ("SMW_SHOT_DIR", out.to_string_lossy().into_owned()),
        ("SMW_SEGMENT", segment.map(|k| k.to_string()).unwrap_or_default()),
    ];
    for (key, value) in &vars {
        std::env::set_var(key, value);
    }
    let dir = |p: &Path| PathBuf::from(format!("{}/", p.display()));
    let argv = vec!["smw".to_string(), "--datadir".to_string(), data.to_string_lossy().into_owned()];
    game(argv, smw_headless::services(dir(&settings), dir(out)));
    for (key, _) in &vars {
        std::env::remove_var(key);
    }
    let _ = std::fs::remove_dir_all(&data);
    std::fs::read_to_string(&dump).unwrap()
}

/// A text file, gunzipped when it starts with the gzip magic.
fn read_maybe_gzip(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut text = String::new();
        flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut text).unwrap();
        text
    } else {
        String::from_utf8(bytes).unwrap()
    }
}

/// The dump's `F` blocks by frame number.
fn frames(dump: &str) -> Vec<(i64, String)> {
    let mut out: Vec<(i64, String)> = Vec::new();
    for line in dump.lines() {
        if let Some(rest) = line.strip_prefix("F ") {
            out.push((rest.split(' ').next().unwrap().parse().unwrap(), String::new()));
        } else if out.is_empty() {
            out.push((-1, String::new()));
        }
        let block = &mut out.last_mut().unwrap().1;
        block.push_str(line);
        block.push('\n');
    }
    out
}

fn assert_same_frames(name: &str, expected: &[(i64, String)], actual: &[(i64, String)]) {
    if let Some((e, a)) = expected.iter().zip(actual).find(|(e, a)| e != a) {
        panic!("{}: diverged at frame {}\nexpected:\n{}actual:\n{}", name, e.0, e.1, a.1);
    }
    assert_eq!(expected.len(), actual.len(), "{}: frame count", name);
}

/// RGB rows of a PNG golden.
fn read_png(path: &Path) -> (u32, u32, Vec<[u8; 3]>) {
    let mut decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let step = info.color_type.samples();
    let pixels = buf[..info.buffer_size()].chunks(step).map(|p| if step >= 3 { [p[0], p[1], p[2]] } else { [p[0]; 3] }).collect();
    (info.width, info.height, pixels)
}

/// RGB rows of the 24- or 32-bit BMP the harness saves.
fn read_bmp(path: &Path) -> (u32, u32, Vec<[u8; 3]>) {
    let data = std::fs::read(path).unwrap();
    let u32_at = |i: usize| u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
    let (offset, w, h, bpp) = (u32_at(10) as usize, u32_at(18) as i32, u32_at(22) as i32, u16::from_le_bytes([data[28], data[29]]) as usize);
    let step = bpp / 8;
    let stride = (w as usize * step + 3) & !3;
    let mut pixels = Vec::new();
    for y in 0..h.unsigned_abs() as usize {
        let row = if h > 0 { h as usize - 1 - y } else { y };
        for x in 0..w as usize {
            let p = &data[offset + row * stride + x * step..];
            pixels.push([p[2], p[1], p[0]]);
        }
    }
    (w as u32, h.unsigned_abs(), pixels)
}

/// Runs `tools/replays/<name>.txt` and compares it with its golden in `tools/golden`.
fn check(name: &str) {
    let tools = repo().join("tools");
    let golden = tools.join("golden").join(name);
    let out = std::env::temp_dir().join(format!("smw-headless-{}-{}", std::process::id(), name));
    let dump = run(&tools.join("replays").join(format!("{}.txt", name)), &out, None);

    let golden_dump = golden.join("dump.txt");
    let golden_dump = if golden_dump.exists() { golden_dump } else { golden.join("dump.txt.gz") };
    assert_same_frames(name, &frames(&read_maybe_gzip(&golden_dump)), &frames(&dump));
    assert_same_shots(name, &golden, &out);
    let _ = std::fs::remove_dir_all(&out);
}

/// The screenshots under `out` equal the golden's.
fn assert_same_shots(name: &str, golden: &Path, out: &Path) {
    let mut shots = 0;
    for entry in std::fs::read_dir(&golden).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(frame) = file.strip_prefix("frame_").and_then(|f| f.strip_suffix(".png")) else { continue };
        let expected = read_png(&path);
        let actual = read_bmp(&out.join(format!("frame_{}.bmp", frame)));
        assert_eq!((expected.0, expected.1), (actual.0, actual.1), "{}: frame {} size", name, frame);
        let differing = expected.2.iter().zip(&actual.2).filter(|(e, a)| e != a).count();
        assert_eq!(differing, 0, "{}: frame {} has {} differing pixels", name, frame, differing);
        shots += 1;
    }
    assert!(shots > 0, "{}: no golden screenshots", name);
}

#[test]
fn start_classic() {
    check("start_classic");
}

#[test]
fn joy_menu() {
    check("joy_menu");
}

#[test]
fn joy_game() {
    check("joy_game");
}

#[test]
fn pad_hotplug() {
    check("pad_hotplug");
}

/// As `tools/segment_check.py`: match 1 played from its checkpoint dumps the full replay's frames of that match.
#[test]
fn segment_from_checkpoint() {
    let replay = repo().join("tools/checkpoint_fixtures/web_gamepad_ztar_b64.txt");
    let base = std::env::temp_dir().join(format!("smw-headless-{}-segment", std::process::id()));
    let marks: Vec<(i64, String)> = std::fs::read_to_string(&replay)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix("#@ mark frame="))
        .map(|l| {
            let (frame, rest) = l.split_once(' ').unwrap();
            (frame.parse().unwrap(), rest.split(' ').next().unwrap().trim_start_matches("state=").to_string())
        })
        .collect();
    let start = marks.iter().position(|m| m.1 == "gameplay").expect("a gameplay marker");
    let first = marks[start].0;
    let end = marks[start..].iter().find(|m| m.1 != "gameplay" && m.1 != "scoreboard").expect("a marker after the match").0;

    let full = frames(&run(&replay, &base.join("full"), None));
    let segment = frames(&run(&replay, &base.join("segment"), Some(1)));
    let expected: Vec<(i64, String)> = full.into_iter().filter(|f| (first..end).contains(&f.0)).collect();
    assert_eq!(expected.len() as i64, end - first, "full dump covers the match");
    assert_same_frames("segment 1", &expected, &segment);
    let _ = std::fs::remove_dir_all(&base);
}

/// Handles step 6 (docs/design/HANDLES.md): a copy of the match made at frame `at` plays the rest of the replay alongside
/// the original. Both dump what the golden holds and the original's screenshots stay the golden's, so the copy shares
/// nothing the match changes.
fn clone_mid_match(name: &str, at: u32) {
    use smw::common::game_values::AppState;
    use smw::smw::game_state::StateId;

    let tools = repo().join("tools");
    let out = std::env::temp_dir().join(format!("smw-headless-{}-clone-{}", std::process::id(), name));
    let copy_dump = out.join("copy_dump.txt");
    let copy_settings = PathBuf::from(format!("{}/", out.join("copy").display()));
    let mut copied = 0;
    let dump = sandboxed(&tools.join("replays").join(format!("{}.txt", name)), &out, None, |argv, services| {
        let g = smw::game::Game::create();
        g.services = Some(services);
        smw::globals::init_globals(g);
        g.RootDataDirectory = argv[2].clone();
        smw::smw::main::startup(g);
        smw::smw::main::gameloop_start(g);
        let mut copy: Option<Box<smw::game::Game>> = None;
        while g.game_values.appstate != AppState::Quit {
            smw::smw::main::frame(g);
            if let Some(c) = copy.as_mut().filter(|_| g.gsm.currentState == StateId::Gameplay) {
                smw::smw::main::step(c);
                copied += 1;
            } else if smw::smw::harness::frame(g) == at {
                assert_eq!(g.gsm.currentState, StateId::Gameplay, "{}: frame {} is not in a match", name, at);
                let mut c = g.clone_match(smw_headless::services(copy_settings.clone(), copy_settings.clone()));
                smw::smw::harness::dump_to(&mut c, &copy_dump.to_string_lossy());
                copy = Some(c);
            }
        }
        smw::smw::main::shutdown(g);
    });
    assert!(copied >= 300, "{}: the copy played {} frames", name, copied);

    let golden = tools.join("golden").join(name);
    let golden_dump = golden.join("dump.txt");
    let golden_dump = if golden_dump.exists() { golden_dump } else { golden.join("dump.txt.gz") };
    let expected = frames(&read_maybe_gzip(&golden_dump));
    assert_same_frames(name, &expected, &frames(&dump));
    assert_same_shots(name, &golden, &out);
    let copy = frames(&std::fs::read_to_string(&copy_dump).unwrap());
    assert_eq!(copy.len(), copied, "{}: the copy's dump", name);
    let expected: Vec<(i64, String)> = expected.into_iter().filter(|f| f.0 >= at as i64).take(copied).collect();
    assert_same_frames(&format!("{} copy", name), &expected, &copy);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn clone_capture_the_flag() {
    clone_mid_match("cpu_ctf", 1500);
}

#[test]
fn clone_frenzy() {
    clone_mid_match("frenzy_cpu", 1200);
}

/// Netplay v2 rollback (docs/design/NETPLAY_V2.md): from frame `at`, the match runs under GGRS's sync test, which
/// loads the state from `check` ticks back and re-runs them every tick and fails on any checksum difference. The
/// checksum at the end must match the golden, so a platform whose simulation differs fails too.
fn rollback_mid_match(name: &str, at: Option<u32>, ticks: u32, check: usize) -> u32 {
    let (ran, _, sum) = stats_after(name, at, ticks, Some(check));
    if ran > 0 {
        let at = at.map_or("-".to_string(), |at| at.to_string());
        assert_rollback_checksum(&format!("{name} {at} {ticks}"), sum);
    }
    ran
}

/// The rollback checksums each run must end on, one `<replay> <frame or -> <ticks> <checksum>` line per run.
const ROLLBACK_CHECKSUMS: &str = "tools/golden/rollback_checksums.txt";
static CHECKSUM_FILE: Mutex<()> = Mutex::new(());

/// Checks `sum` against the golden for `key`, or records it with SMW_WRITE_ROLLBACK_CHECKSUMS=1.
fn assert_rollback_checksum(key: &str, sum: u128) {
    let _file = CHECKSUM_FILE.lock().unwrap_or_else(|e| e.into_inner());
    let path = repo().join(ROLLBACK_CHECKSUMS);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let golden = text.lines().find_map(|l| l.rsplit_once(' ').filter(|(k, _)| *k == key).map(|(_, v)| v.to_string()));
    let actual = format!("{sum:016x}");
    if std::env::var_os("SMW_WRITE_ROLLBACK_CHECKSUMS").is_some() {
        let mut lines: Vec<String> = text.lines().filter(|l| l.rsplit_once(' ').is_none_or(|(k, _)| k != key)).map(String::from).collect();
        lines.push(format!("{key} {actual}"));
        lines.sort();
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    } else {
        assert_eq!(golden.as_deref(), Some(actual.as_str()), "{key}: rollback checksum; SMW_WRITE_ROLLBACK_CHECKSUMS=1 records it in {ROLLBACK_CHECKSUMS}");
    }
}

/// Runs `ticks` ticks of the match from frame `at`, under the sync test with `check`, or plainly without it, and
/// returns how many ran and the match stats and checksum at the end.
fn stats_after(name: &str, at: Option<u32>, ticks: u32, check: Option<usize>) -> (u32, smw::smw::stats::MatchStats, u128) {
    use smw::common::game_values::AppState;
    use smw::smw::game_state::StateId;
    use smw::smw::rollback::{handle_requests, GgrsConfig, KeySlot, Keys};

    let tools = repo().join("tools");
    let out = std::env::temp_dir().join(format!("smw-headless-{}-rollback-{}", std::process::id(), name));
    let mut ran = 0;
    let mut failure = None;
    let mut in_match = 0;
    let mut stats = smw::smw::stats::MatchStats::new();
    let mut sum = 0;
    let mut plain = false;
    sandboxed(&tools.join("replays").join(format!("{}.txt", name)), &out, None, |argv, services| {
        let g = smw::game::Game::create();
        g.services = Some(services);
        smw::globals::init_globals(g);
        g.RootDataDirectory = argv[2].clone();
        smw::smw::main::startup(g);
        smw::smw::main::gameloop_start(g);
        let mut session: Option<ggrs::SyncTestSession<GgrsConfig<usize>>> = None;
        while g.game_values.appstate != AppState::Quit && ran < ticks && failure.is_none() {
            if plain {
                if g.gsm.currentState != StateId::Gameplay {
                    break;
                }
                smw::smw::main::frame(g);
                ran += 1;
                continue;
            }
            let Some(s) = session.as_mut().filter(|_| g.gsm.currentState == StateId::Gameplay) else {
                in_match = if g.gsm.currentState == StateId::Gameplay { in_match + 1 } else { 0 };
                let start = match at {
                    Some(at) => smw::smw::harness::frame(g) == at,
                    None => in_match == 300,
                };
                if start {
                    let frame = smw::smw::harness::frame(g);
                    assert_eq!(g.gsm.currentState, StateId::Gameplay, "{}: frame {} is not in a match", name, frame);
                    let Some(check) = check else {
                        plain = true;
                        continue;
                    };
                    let builder = ggrs::SessionBuilder::<GgrsConfig<usize>>::new().with_num_players(4).unwrap().with_check_distance(check);
                    session = Some(builder.start_synctest_session().unwrap());
                    continue;
                }
                if session.is_some() {
                    break;
                }
                smw::smw::main::frame(g);
                continue;
            };
            smw::smw::harness::frame_start_input(g);
            let mut gp = g.ptr();
            gp.gps.as_mut().unwrap().gather_input(g);
            for p in 0..4 {
                s.add_local_input(p, Keys::of(&g.game_values.playerInput.outputControls[p])).unwrap();
            }
            match s.advance_frame() {
                Ok(requests) => {
                    handle_requests(g, requests, &[KeySlot::Local(0), KeySlot::Local(1), KeySlot::Local(2), KeySlot::Local(3)], true, 1);
                }
                Err(e) => failure = Some(format!("{} at tick {}: {}", name, ran, e)),
            }
            ran += 1;
        }
        stats = g.game_values.stats;
        sum = smw::smw::rollback::checksum(g);
        smw::smw::main::shutdown(g);
    });
    let _ = std::fs::remove_dir_all(&out);
    if let Some(failure) = failure {
        panic!("{}", failure);
    }
    assert!(ran == 0 || ran >= ticks.min(300), "{}: only {} ticks under rollback", name, ran);
    (ran, stats, sum)
}

/// Rolled-back ticks must not count twice: the sync test's re-runs leave the same stats as a plain run.
#[test]
fn rollback_keeps_match_stats() {
    let (ran, rolled, _) = stats_after("cpu_ctf", Some(1500), 400, Some(7));
    let (plain_ran, plain, _) = stats_after("cpu_ctf", Some(1500), 400, None);
    assert_eq!((ran, plain_ran), (400, 400));
    assert!(plain.players.iter().any(|p| p.jumps > 0 && p.alive_ticks > 0), "no stats counted");
    assert_eq!(rolled, plain);
}

#[test]
fn rollback_capture_the_flag() {
    assert_eq!(rollback_mid_match("cpu_ctf", Some(1500), 400, 7), 400);
}

#[test]
fn rollback_frenzy() {
    assert_eq!(rollback_mid_match("frenzy_cpu", Some(1200), 400, 7), 400);
}

/// Prints the average cost of a rollback save, a load of the copy GGRS hands back, and a plain tick over `ticks` ticks
/// of the first match, from its 300th tick, played on `map` when given.
fn snapshot_cost(name: &str, map: Option<&str>, ticks: u32) {
    use smw::smw::rollback::{load, save};
    use std::time::{Duration, Instant};

    let out = std::env::temp_dir().join(format!("smw-headless-{}-snapshot-cost-{}", std::process::id(), name));
    let mut replay = repo().join("tools").join("replays").join(format!("{}.txt", name));
    if let Some(map) = map {
        let text = std::fs::read_to_string(&replay).unwrap();
        let text: String = text.lines().map(|l| if l.starts_with("#@ map=") { format!("#@ map={map}\n") } else { format!("{l}\n") }).collect();
        replay = std::env::temp_dir().join(format!("smw-headless-{}-snapshot-cost-{}.txt", std::process::id(), name));
        std::fs::write(&replay, text).unwrap();
    }
    let (mut saving, mut loading, mut ticking, mut ran) = (Duration::ZERO, Duration::ZERO, Duration::ZERO, 0);
    sandboxed(&replay, &out, None, |argv, services| {
        let g = smw::game::Game::create();
        g.services = Some(services);
        smw::globals::init_globals(g);
        g.RootDataDirectory = argv[2].clone();
        smw::smw::main::startup(g);
        smw::smw::main::gameloop_start(g);
        let mut in_match = 0;
        while in_match < 300 && g.game_values.appstate != smw::common::game_values::AppState::Quit {
            smw::smw::main::frame(g);
            in_match = if g.gsm.currentState == smw::smw::game_state::StateId::Gameplay { in_match + 1 } else { 0 };
        }
        while ran < ticks && g.gsm.currentState == smw::smw::game_state::StateId::Gameplay {
            let t = Instant::now();
            let state = save(g);
            saving += t.elapsed();
            let t = Instant::now();
            load(g, state.clone());
            loading += t.elapsed();
            let t = Instant::now();
            smw::smw::main::frame(g);
            ticking += t.elapsed();
            ran += 1;
        }
        smw::smw::main::shutdown(g);
    });
    let _ = std::fs::remove_dir_all(&out);
    if map.is_some() {
        let _ = std::fs::remove_file(&replay);
    }
    let avg = |d: Duration| d.as_secs_f64() * 1000.0 / ran.max(1) as f64;
    println!("SNAPSHOT {name} {}: {ran} ticks, save {:.4} ms, load {:.4} ms, tick {:.4} ms", map.unwrap_or("-"), avg(saving), avg(loading), avg(ticking));
}

/// Run with --ignored --nocapture. Log Balancing has animated tiles and four moving platforms.
#[test]
#[ignore]
fn rollback_snapshot_cost() {
    snapshot_cost("frenzy_cpu", None, 600);
    snapshot_cost("cpu_greed", None, 600);
    snapshot_cost("cpu_greed", Some("Log Balancing"), 600);
}

/// Every replay's first match under rollback; `SMW_ROLLBACK_SWEEP=<substring>` picks replays. Run with --ignored.
#[test]
#[ignore]
fn rollback_sweep() {
    let pick = std::env::var("SMW_ROLLBACK_SWEEP").unwrap_or_default();
    let mut names: Vec<String> = std::fs::read_dir(repo().join("tools").join("replays"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".txt").map(String::from))
        .filter(|n| n.contains(&pick))
        .collect();
    names.sort();
    let mut failed = Vec::new();
    for name in &names {
        match std::panic::catch_unwind(|| rollback_mid_match(name, None, 600, 7)) {
            Err(e) => {
                let msg = e.downcast_ref::<String>().cloned().unwrap_or_else(|| "panic".into());
                println!("ROLLBACK FAIL {}", msg);
                failed.push(name.clone());
            }
            Ok(0) => println!("ROLLBACK SKIP {} (no match)", name),
            Ok(ticks) => println!("ROLLBACK OK {} ({} ticks)", name, ticks),
        }
    }
    assert!(failed.is_empty(), "{} of {} replays failed: {:?}", failed.len(), names.len(), failed);
}

