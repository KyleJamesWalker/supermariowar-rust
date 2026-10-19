//! smw-sdl3 against SDL3 itself, on the main thread as SDL3's video requires: keys answer in SDL2's values, sounds
//! decode to the lengths the core's duration table lists, events survive SDL's queue, and a virtual gamepad's
//! events reach the game in smw-sdl2's translated layout, and the window and sound run on SDL's dummy drivers.

use sdl3_sys::everything::*;
use smw_platform::{AudioOut, Events, Frame, Input, InputEvent, Keyboard, Keycode, Overlay, OverlayImage, OverlayQuad, PadSlot, Pads, SoundId, TrackId, Video};
use std::cell::RefCell;
use std::rc::Rc;

/// SDL2's default keymap on its dummy video driver: `<scancode> <keycode> <name>`.
const SDL2_KEYS: &str = include_str!("sdl2_keys.txt");
const DURATIONS: &str = include_str!("../../smw-core/src/common/sfx_durations.txt");

fn keys_answer_in_sdl2_values() {
    let kb = smw_sdl3::Sdl3Keyboard;
    let mut checked = 0;
    let mut named = std::collections::HashSet::new();
    let mut skipped = Vec::new();
    for line in SDL2_KEYS.lines().filter(|l| !l.starts_with('#')) {
        let mut f = line.splitn(3, ' ');
        let scancode: i32 = f.next().unwrap().parse().unwrap();
        let key: i32 = f.next().unwrap().parse().unwrap();
        let name = f.next().unwrap();
        // SDL3 renamed the consumer keys; the game binds none of them.
        if (258..=290).contains(&scancode) {
            continue;
        }
        // Releases differ in which scancodes have names (3.4.16 has no "Front").
        if unsafe { *SDL_GetScancodeName(SDL_Scancode(scancode)) } == 0 {
            skipped.push(name);
            continue;
        }
        if named.insert(name.to_lowercase()) {
            assert_eq!(kb.key_from_name(name), key, "key_from_name({:?})", name);
        }
        assert_eq!(kb.key_name(key), name, "key_name({})", key);
        assert_eq!(kb.scancode_from_key(key), scancode, "scancode_from_key({})", key);
        checked += 1;
    }
    for c in b'!'..=b'~' {
        let name = (c as char).to_string();
        assert_eq!(kb.key_from_name(&name), (c as char).to_ascii_lowercase() as i32, "key_from_name({:?})", name);
    }
    assert_eq!(kb.key_from_name("Nonsense"), 0);
    println!("keys: {} SDL2 keys answer alike; this SDL3 does not name {:?}", checked, skipped);
}

fn sounds_decode_to_the_table_lengths() {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/");
    let (mut same, mut differ) = (0, Vec::new());
    for line in DURATIONS.lines().filter(|l| l.starts_with("chunk ")) {
        let f: Vec<&str> = line.split(' ').collect();
        let bytes = std::fs::read(format!("{}{}", data, f[1])).unwrap();
        let pcm = smw_sdl3::decode_wav(&bytes).unwrap_or_else(|e| panic!("{}: {}", f[1], e));
        let ms = (pcm.len() as u64 * 1000 / 176_400).to_string();
        if ms == f[3] {
            same += 1;
        } else {
            differ.push(format!("{} {} ms (SDL_mixer {} ms)", f[1], ms, f[3]));
        }
    }
    println!("sounds: {} of {} decode to SDL_mixer's length in ms", same, same + differ.len());
    assert!(differ.is_empty(), "{:#?}", differ);
}

fn events_survive_the_queue() {
    let mut events = smw_sdl3::Sdl3Events::new(Rc::new(RefCell::new(smw_sdl3::PadState::default())));
    while events.poll().is_some() {}
    let inputs = [
        InputEvent::Key { key: Keycode(97), scancode: 4, mods: 0, down: true, repeat: false, window: 0 },
        InputEvent::PadHat { pad: PadSlot(2), hat: 0, value: 4 },
        InputEvent::Text("Mario".into()),
        InputEvent::Tap { x: 12, y: 34 },
        InputEvent::QuitRequest,
        InputEvent::Quit,
    ];
    for event in &inputs {
        events.push(&Input { timestamp: 7, event: event.clone() });
    }
    let mut read = Vec::new();
    while let Some(input) = events.poll() {
        read.push(input.event);
    }
    assert_eq!(read, inputs);
    println!("events: {} kinds round trip through SDL's queue", inputs.len());
}

fn a_virtual_gamepad_takes_the_layout() {
    unsafe {
        assert!(SDL_InitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMEPAD));
        let mut desc = SDL_VirtualJoystickDesc::new();
        desc.r#type = SDL_JOYSTICK_TYPE_GAMEPAD.0 as _;
        desc.naxes = SDL_GAMEPAD_AXIS_COUNT.0 as u16;
        desc.nbuttons = SDL_GAMEPAD_BUTTON_COUNT.0 as u16;
        desc.name = c"smw virtual pad".as_ptr();
        let id = SDL_AttachVirtualJoystick(&desc);
        assert_ne!(id.0, 0, "SDL_AttachVirtualJoystick");

        let state = Rc::new(RefCell::new(smw_sdl3::PadState::default()));
        let mut pads = smw_sdl3::Sdl3Pads::new(state.clone());
        let mut events = smw_sdl3::Sdl3Events::new(state);
        let infos = pads.open_all(true, true);
        let info = infos.iter().flatten().find(|i| i.pad == PadSlot(id.0 as i32)).expect("the virtual pad opens");
        assert!(info.controller, "a gamepad is translated");
        let pad = info.pad;
        while events.poll().is_some() {}

        let js = SDL_GetJoystickFromID(id);
        let press = |button: SDL_GamepadButton, down: bool| {
            SDL_SetJoystickVirtualButton(js, button.0, down);
            SDL_UpdateJoysticks();
        };
        let mut read = || {
            let mut out = Vec::new();
            while let Some(input) = events.poll() {
                out.push(input.event);
            }
            out
        };
        press(SDL_GAMEPAD_BUTTON_NORTH, true);
        assert!(read().contains(&InputEvent::PadButton { pad, button: 5, down: true }), "Y is joystick button 5");
        press(SDL_GAMEPAD_BUTTON_START, true);
        assert!(read().contains(&InputEvent::PadButton { pad, button: 3, down: true }), "Start is joystick button 3");
        press(SDL_GAMEPAD_BUTTON_DPAD_LEFT, true);
        assert!(read().contains(&InputEvent::PadHat { pad, hat: 0, value: 8 }), "D-pad left is hat 0 left");
        SDL_SetJoystickVirtualAxis(js, SDL_GAMEPAD_AXIS_RIGHT_TRIGGER.0, 32767);
        SDL_UpdateJoysticks();
        assert!(read().contains(&InputEvent::PadButton { pad, button: 9, down: true }), "the right trigger is button 9");
        assert_eq!(pads.name(0, Some(pad)).as_deref(), Some("smw virtual pad"));
        SDL_DetachVirtualJoystick(id);
        println!("pads: a virtual gamepad reads in the translated layout");
    }
}

fn video_and_audio_run_on_dummy_drivers() {
    let mut video = smw_sdl3::Sdl3Video::new();
    video.open(false);
    assert_eq!(video.output_size(), (640, 480));
    let pixels = vec![0xFF20_4080u32; 640 * 480];
    let overlay = Overlay {
        generation: 1,
        images: vec![OverlayImage { w: 2, h: 2, rgba: vec![255; 16] }].into(),
        quads: vec![OverlayQuad { image: 0, x: 10, y: 10, w: 40, h: 40, alpha: 128, color: [255, 0, 0] }],
    };
    for _ in 0..3 {
        video.present(Frame { pixels: &pixels }, Some(&overlay));
    }

    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/");
    let mut audio = smw_sdl3::Sdl3Audio::new();
    audio.open(true);
    assert!(audio.is_open());
    let ms = audio.load_sound(SoundId(1), &std::fs::read(format!("{}sfx/packs/Classic/jump.wav", data)).unwrap()).unwrap();
    assert!(ms > 0);
    let track = DURATIONS.lines().find_map(|l| l.strip_prefix("music ")).and_then(|l| l.split(' ').next()).unwrap();
    assert!(audio.load_track(TrackId(1), &std::fs::read(format!("{}{}", data, track)).unwrap()).is_ok(), "{}", track);
    audio.set_sound_volume(64);
    audio.set_music_volume(64);
    audio.play(0, SoundId(1), 2);
    audio.play_track(TrackId(1), false);
    std::thread::sleep(std::time::Duration::from_millis(300));
    audio.pause_track(true);
    audio.halt(None);
    audio.stop_track();
    audio.close();
    assert!(!audio.is_open());
    println!("video and audio: frames presented, a sound and {} played", track);
}

fn main() {
    unsafe {
        SDL_SetHint(SDL_HINT_VIDEO_DRIVER, c"dummy".as_ptr());
        SDL_SetHint(SDL_HINT_AUDIO_DRIVER, c"dummy".as_ptr());
        SDL_SetHint(SDL_HINT_KEYCODE_OPTIONS, c"unmodified".as_ptr());
        assert!(SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS), "SDL_Init");
    }
    keys_answer_in_sdl2_values();
    sounds_decode_to_the_table_lengths();
    events_survive_the_queue();
    a_virtual_gamepad_takes_the_layout();
    video_and_audio_run_on_dummy_drivers();
    unsafe { SDL_Quit() };
}
