//! Headless gfx smoke test: draws sprites, a palette-swapped skin and font text, saves a BMP.
//! Mirrors tools/gfx_smoke/gfx_smoke.cpp, which does the same with the C++ originals.
//!
//! SDL_VIDEODRIVER=dummy cargo run --example gfx_smoke -- <data dir> <out.bmp>
use smw::common::gfx::soft_surface::Rect;
use smw::common::gfx::gfx_font::gfxFont;
use smw::common::gfx::gfx_sprite::{ClipEdge, ImageLoader};
use smw::common::gfx::*;
use std::path::Path;

fn main() {
    let g = smw::game::Game::create();

    let mut gp = g.ptr();    let args: Vec<String> = std::env::args().collect();
    let data = args.get(1).cloned().unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data").to_string());
    let out = args.get(2).cloned().unwrap_or_else(|| "gfx_smoke_rust.bmp".to_string());

    smw::globals::init_globals(gp.get());
    unsafe {
        gp.RootDataDirectory = data.clone();
    }

    gfx_init(gp.get(), 640, 480, false);
    unsafe { gp.blitdest = gp.screen };

    let pack = format!("{}/gfx/packs/Classic", data);
    assert!(gfx_loadpalette(gp.get(), Path::new(&format!("{}/palette.png", pack))));

    let backdrop = ImageLoader::new(format!("{}/menu/menu_background.png", pack)).without_color_key().create(gp.get());
    let smw_logo = ImageLoader::new(format!("{}/menu/menu_smw.png", pack)).create(gp.get());
    let shade = ImageLoader::new(format!("{}/menu/menu_shade.png", pack)).with_alpha(72).without_color_key().create(gp.get());
    let ghost = ImageLoader::new(format!("{}/eyecandy/ghost.png", pack)).with_alpha(128).with_wrapping(640).create(gp.get());
    let overlay = ImageLoader::new(format!("{}/eyecandy/overlayholes.png", pack))
        .with_color_key(smw::common::gfx::color::RGB { r: 0, g: 255, b: 0 })
        .create(gp.get());

    let mut font_large = gfxFont::from_path(gp.get(), &format!("{}/menu/menu_font_large.png", pack));
    let font_small = gfxFont::from_path(gp.get(), &format!("{}/fonts/font_small.png", pack));

    let skin = gfx_loadfullskin(gp.get(), Path::new(&format!("{}/gfx/skins/0smw.png", data)), 1).unwrap();
    let menuskin = gfx_loadmenuskin(gp.get(), Path::new(&format!("{}/gfx/skins/0smw.png", data)), 2, true).unwrap();

    backdrop.draw(gp.get(), 0, 0);
    smw_logo.draw(gp.get(), 70, 30);
    shade.draw_part(gp.get(), 100, 200, 0, 0, 200, 100);
    ghost.draw_part(gp.get(), 620, 300, 0, 0, 32, 32);
    overlay.draw_part(gp.get(), 400, 350, 0, 0, 64, 64);
    for i in 0..10 {
        skin[i].draw_part(gp.get(), 20 + i as i32 * 34, 400, 0, 0, 32, 32);
        skin[i].draw_part(gp.get(), 20 + i as i32 * 34, 434, 32 * (i as i32 % 9), 0, 32, 32);
    }
    menuskin[0].draw_part(gp.get(), 380, 400, 0, 0, 32, 32);
    menuskin[3].draw_part(gp.get(), 414, 400, 96, 0, 32, 32);
    skin[0].draw_clip(gp.get(), 450, 400, &Rect { x: 0, y: 0, w: 32, h: 32 }, ClipEdge::Left, 460);
    skin[0].draw_stretch(&Rect { x: 0, y: 0, w: 32, h: 32 }, unsafe { gp.blitdest }, &Rect { x: 500, y: 380, w: 64, h: 64 });
    gfx_drawpreview(gp.get(), &skin[2], 620, 300, 0, 0, 32, 32, &Rect { x: 0, y: 0, w: 640, h: 480 }, true, None);

    font_large.draw(gp.get(), 10, 150, "Super Mario War: 0123456789");
    font_small.draw_centered(gp.get(), 320, 180, "centered small text ~!@#$%^&*()");
    font_large.draw_right_justified(gp.get(), 630, 210, "right justified");
    font_small.draw_chop_right(gp.get(), 10, 250, 120, "chopped right text that is long");
    font_small.draw_chop_left(gp.get(), 630, 250, 120, "chopped left text that is long");
    font_small.draw_chop_centered(gp.get(), 320, 270, 100, "chop centered text that is long");
    font_large.set_alpha(128);
    font_large.draw(gp.get(), 10, 300, "translucent");
    unsafe {
        gp.x_shake = 3;
        gp.y_shake = -2;
    }
    font_small.draw(gp.get(), 10, 330, "shaken");
    smw_logo.draw_part(gp.get(), 300, 330, 0, 0, 40, 20);

    assert!(gfx_save_screen_bmp(gp.get(), &out));
    println!("wrote {}", out);
}
