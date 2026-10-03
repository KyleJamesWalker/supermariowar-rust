//! Headless gfx smoke test: draws sprites, a palette-swapped skin and font text, saves a BMP.
//! Mirrors tools/gfx_smoke/gfx_smoke.cpp, which does the same with the C++ originals.
//!
//! SDL_VIDEODRIVER=dummy cargo run --example gfx_smoke -- <data dir> <out.bmp>

use sdl2::sys::SDL_Rect;
use smw::common::gfx::gfx_font::gfxFont;
use smw::common::gfx::gfx_sprite::{ClipEdge, SpriteBuilder};
use smw::common::gfx::*;
use smw::globals::*;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data = args.get(1).cloned().unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/data").to_string());
    let out = args.get(2).cloned().unwrap_or_else(|| "gfx_smoke_rust.bmp".to_string());

    smw::globals::init_globals();
    unsafe {
        RootDataDirectory = data.clone();
    }

    gfx_init(640, 480, false);
    unsafe { blitdest = screen };

    let pack = format!("{}/gfx/packs/Classic", data);
    assert!(gfx_loadpalette(Path::new(&format!("{}/palette.png", pack))));

    let backdrop = SpriteBuilder::new(format!("{}/menu/menu_background.png", pack)).without_color_key().create();
    let smw_logo = SpriteBuilder::new(format!("{}/menu/menu_smw.png", pack)).create();
    let shade = SpriteBuilder::new(format!("{}/menu/menu_shade.png", pack)).with_alpha(72).without_color_key().create();
    let ghost = SpriteBuilder::new(format!("{}/eyecandy/ghost.png", pack)).with_alpha(128).with_wrapping(640).create();
    let overlay = SpriteBuilder::new(format!("{}/eyecandy/overlayholes.png", pack))
        .with_color_key(smw::common::gfx::color::RGB { r: 0, g: 255, b: 0 })
        .create();

    let mut font_large = gfxFont::from_path(&format!("{}/menu/menu_font_large.png", pack));
    let font_small = gfxFont::from_path(&format!("{}/fonts/font_small.png", pack));

    let skin = gfx_loadfullskin(Path::new(&format!("{}/gfx/skins/0smw.png", data)), 1).unwrap();
    let menuskin = gfx_loadmenuskin(Path::new(&format!("{}/gfx/skins/0smw.png", data)), 2, true).unwrap();

    backdrop.draw(0, 0);
    smw_logo.draw(70, 30);
    shade.draw_part(100, 200, 0, 0, 200, 100);
    ghost.draw_part(620, 300, 0, 0, 32, 32);
    overlay.draw_part(400, 350, 0, 0, 64, 64);
    for i in 0..10 {
        skin[i].draw_part(20 + i as i32 * 34, 400, 0, 0, 32, 32);
        skin[i].draw_part(20 + i as i32 * 34, 434, 32 * (i as i32 % 9), 0, 32, 32);
    }
    menuskin[0].draw_part(380, 400, 0, 0, 32, 32);
    menuskin[3].draw_part(414, 400, 96, 0, 32, 32);
    skin[0].draw_clip(450, 400, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 }, ClipEdge::Left, 460);
    skin[0].draw_stretch(&SDL_Rect { x: 0, y: 0, w: 32, h: 32 }, unsafe { blitdest }, &SDL_Rect { x: 500, y: 380, w: 64, h: 64 });
    gfx_drawpreview(skin[2].get_surface(), 620, 300, 0, 0, 32, 32, 0, 0, 640, 480, true, None);

    font_large.draw(10, 150, "Super Mario War: 0123456789");
    font_small.draw_centered(320, 180, "centered small text ~!@#$%^&*()");
    font_large.draw_right_justified(630, 210, "right justified");
    font_small.draw_chop_right(10, 250, 120, "chopped right text that is long");
    font_small.draw_chop_left(630, 250, 120, "chopped left text that is long");
    font_small.draw_chop_centered(320, 270, 100, "chop centered text that is long");
    font_large.set_alpha(128);
    font_large.draw(10, 300, "translucent");
    unsafe {
        x_shake = 3;
        y_shake = -2;
    }
    font_small.draw(10, 330, "shaken");
    smw_logo.draw_part(300, 330, 0, 0, 40, 20);

    assert!(gfx_save_screen_bmp(&out));
    println!("wrote {}", out);
}
