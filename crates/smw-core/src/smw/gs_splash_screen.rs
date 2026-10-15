//! Port of src/smw/GSSplashScreen.cpp

use smw_platform::InputEvent;
use crate::common::game::App;
use crate::common::game_values::{if_sound_on_play, AppState};
use crate::common::gfx::gfx_changefullscreen;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
use crate::common::path::convert_path_pack;
use crate::common::sfx::sfxMusic;
use crate::globals::*;
use crate::smw::game_state::{GameState, GameStateManager};
use crate::smw::gs_gameplay::{GameplayState};
use crate::smw::gs_menu::MenuState;
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_FillRect, SDL_KeyCode, SDL_Keymod};

//-----------------------------------------------------------------------------
// THE LOAD UP SEQUENCE + SPLASH SCREEN
//-----------------------------------------------------------------------------
//that's a bunch of ugly code, maybe i'll throw it out again

pub struct SplashScreenState {
    menu_credits: Ptr<gfxSprite>,
    alpha: i32,
    state: i32,
    timer: i32,
    firstFrame: bool,
    pub _alias: Aliased,
}

impl SplashScreenState {
    fn new() -> Self {
        SplashScreenState { _alias: Aliased::new(),
            menu_credits: Ptr::null(),
            alpha: 255,
            state: 7,
            timer: 120,
            firstFrame: true,
        }
    }

    pub fn instance() -> &'static mut SplashScreenState {
        game().get().ss.get_or_insert_with(SplashScreenState::new)
    }
}

impl GameState for SplashScreenState {
    fn on_leave_state(&mut self) {
        self.menu_credits.delete();
    }

    fn init(&mut self) -> bool {
        unsafe {
            game().rm.load_start_graphics();

            self.menu_credits = Ptr::new_box(gfxSprite::new());
            *self.menu_credits =
                ImageLoader::new(convert_path_pack("gfx/packs/menu/splash_credits.png", &game().menugraphicspacklist.current_path().to_string_lossy())).create();
        }

        true
    }

    fn update(&mut self) {
        unsafe {
            game().game_values.playerInput.clear_pressed_keys(1);

            // TODO: move this out of this method maybe

            let mut tapped = false;
            while let Some(input) = smw_sdl2::events::poll_input() {
                tapped |= input.event.tap().is_some();
                if input.event == InputEvent::Quit {
                    game().game_values.appstate = AppState::Quit;
                    return;
                } else if let InputEvent::Key { key, mods, down: true, .. } = input.event {
                    let alt = (mods as u32) & (SDL_Keymod::KMOD_LALT as u32 | SDL_Keymod::KMOD_RALT as u32) != 0;
                    if key.0 == SDL_KeyCode::SDLK_RETURN as i32 {
                        if alt {
                            game().game_values.fullscreen = !game().game_values.fullscreen;
                            gfx_changefullscreen(game().game_values.fullscreen);
                            game().blitdest = game().screen;
                        }
                    } else if key.0 == SDL_KeyCode::SDLK_F4 as i32 {
                        if alt {
                            game().game_values.appstate = AppState::Quit;
                            return;
                        }
                    } else if key.0 == SDL_KeyCode::SDLK_INSERT as i32 {
                        crate::common::gfx::gfx_take_screenshot();
                    }
                }

                game().game_values.playerInput.update_input(&input, 1);
            }

            // Not in upstream: a press before load_game_data() would open the menus without their graphics.
            if self.state == 8 {
                for iPlayer in 0..4usize {
                    let out = &game().game_values.playerInput.outputControls[iPlayer];
                    if tapped || out.menu_select().fPressed || out.menu_cancel().fPressed || out.menu_random().fPressed {
                        game().blitdest = game().rm.menu_backdrop.get_surface();
                        game().rm.menu_shade.setalpha(App::menuTransparency as u8);
                        game().rm.menu_shade.draw(0, 0);
                        game().blitdest = game().screen;

                        game().eyecandy[2].clean();

                        game().game_values.playerInput.reset_keys();
                        game().game_values.appstate = AppState::Menu;

                        MenuState::instance().init();
                        GameplayState::instance().init();
                        GameStateManager::instance().change_state_to(Ptr::from_mut(MenuState::instance() as &mut dyn GameState));
                        return;
                    }
                }
            }

            crate::common::gfx::blit::fill_rect(game().screen, std::ptr::null(), 0x0);

            if self.state == 6 || self.state == 7 || self.state == 8 {
                game().rm.menu_backdrop.setalpha(self.alpha as u8);
                game().rm.menu_backdrop.draw(0, 0);

                game().rm.menu_smw.setalpha(self.alpha as u8);
                game().rm.menu_smw.draw(App::screenWidth / 2 - ((game().rm.menu_smw.get_width() as i16 >> 1) as i32), 30); //smw logo

                game().rm.menu_version.setalpha(self.alpha as u8);
                game().rm.menu_version.draw(628 - game().rm.menu_version.get_width(), 10); //smw logo

                game().rm.menu_font_large.set_alpha(self.alpha as u8);

                self.menu_credits.setalpha(self.alpha as u8);
                self.menu_credits.draw(227, 200);
            }

            if self.state == 7 {
                game().rm.menu_font_large.draw_centered(App::screenWidth / 2, (App::screenHeight as f32 * 0.875f32) as i32, "Loading...");
            } else if self.state == 8 {
                game().rm.menu_font_large.draw_centered(App::screenWidth / 2, (App::screenHeight as f32 * 0.875f32) as i32, "Press Any Key To Continue");

                game().eyecandy[2].clean_dead_objects();
                game().eyecandy[2].update();
                game().eyecandy[2].draw();
            }

            // Only start loading after we displayed something, in the 2nd frame
            if self.firstFrame {
                self.firstFrame = false;
                return;
            }

            if self.state == 7 {
                load_game_data();

                if_sound_on_play(&mut game().rm.sfx_coin);

                self.state += 1;
            }
        }
    }
}

/// The splash screen's second-frame load; a segment replay (smw/checkpoint.rs) runs it before restoring a match.
pub fn load_game_data() {
    unsafe {
        // load initial coin sound
        game().rm.backgroundmusic[2] = sfxMusic::from_file(game().musiclist.music(1)).unwrap_or_else(|e| std::panic::panic_any(e));

        game().rm.load_all_graphics();
        game().rm.load_game_sounds();

        if !game().game_values.soundcapable {
            game().game_values.sound = false;
            game().game_values.music = false;
            game().game_values.soundvolume = 0;
            game().game_values.musicvolume = 0;
        }

        //Read the map filter lists
        game().maplist.read_filters();
        game().maplist.apply_filters(&game().game_values.pfFilters.clone());
    }
}
