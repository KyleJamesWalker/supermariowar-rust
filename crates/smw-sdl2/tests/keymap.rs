//! smw-headless answers key queries as SDL2 does on the dummy video driver, which replays run on.

use smw_platform::Keyboard;

const TABLE: &str = include_str!("../../smw-headless/src/sdl2_keys.txt");

#[test]
fn headless_keys_match_sdl() {
    assert_eq!(smw_sdl2::keymap::measure(), TABLE, "regenerate crates/smw-headless/src/sdl2_keys.txt");

    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    assert_eq!(unsafe { sdl2::sys::SDL_InitSubSystem(sdl2::sys::SDL_INIT_VIDEO) }, 0);
    let (sdl, headless) = (smw_sdl2::events::Sdl2Keyboard, smw_headless::HeadlessKeyboard::new());

    let mut names: Vec<String> = vec![String::new(), "Nonsense".into(), "é".into(), "éa".into()];
    let mut keys: Vec<i32> = (0..=255).chain([1 << 30, (1 << 30) | 600, -1]).collect();
    for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
        let mut fields = line.splitn(3, ' ');
        fields.next();
        keys.push(fields.next().unwrap().parse().unwrap());
        if let Some(name) = fields.next() {
            names.extend([name.to_string(), name.to_lowercase(), name.to_uppercase()]);
        }
    }
    names.extend((1..=126u8).map(|c| (c as char).to_string()));

    for name in &names {
        assert_eq!(headless.key_from_name(name), sdl.key_from_name(name), "key_from_name({:?})", name);
    }
    for &key in &keys {
        assert_eq!(headless.key_name(key), sdl.key_name(key), "key_name({})", key);
        // sdl2-compat maps the consumer keys (AudioNext to EndCall) through SDL3's numbering.
        if !((1 << 30) | 258..=(1 << 30) | 290).contains(&key) {
            assert_eq!(headless.scancode_from_key(key), sdl.scancode_from_key(key), "scancode_from_key({})", key);
        }
    }
}
