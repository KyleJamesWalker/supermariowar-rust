//! Graphics options: top layer, frame rate, fullscreen, the menu, world and game graphics packs, and, where the
//! backend supports them, the shader preset and integer scaling.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::global_constants::FRAME_RATE_DISPLAY;
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;
use crate::smw::ui::mi_packs_field::MI_PacksField;

/// Windowed or fullscreen mode, frame rate and the graphics packs.
#[derive(Default)]
pub struct UI_GraphicsOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miTopLayerField: Ctl<MI_SelectField<bool>>,
    /// Writes `framelimiter`: `FRAME_RATE_DISPLAY`, or 201 to 207 for a fixed rate.
    pub miFrameLimiterField: Ctl<MI_SelectField<i16>>,
    pub miFullscreenField: Ctl<MI_SelectField<bool>>,

    pub miMenuGraphicsPackField: Ctl<MI_PacksField>,
    pub miWorldGraphicsPackField: Ctl<MI_PacksField>,
    pub miGameGraphicsPackField: Ctl<MI_PacksField>,
    /// Writes `Game::video.shader`; made only when the backend supports shaders.
    pub miShaderField: Ctl<MI_SelectField<i16>>,
    pub miIntegerScaleField: Ctl<MI_SelectField<bool>>,
    pub miGraphicsOptionsMenuBackButton: Ctl<MI_Button>,

    pub miGraphicsOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miGraphicsOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miGraphicsOptionsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_GraphicsOptionsMenu => ui_menu: UI_Menu);

impl UI_GraphicsOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;

            this.miTopLayerField = select_field(spr_selectfield, 70, 120, "Draw Top Layer", 500, 220,
                &[("Background", false), ("Foreground", true)],
                &mut g.game_values.toplayer, g);
            g.ctl(this.miTopLayerField).set_auto_advance(true);

            this.miFrameLimiterField = select_field(spr_selectfield, 70, 160, "Frame Rate", 500, 220,
                &[
                    ("Display", FRAME_RATE_DISPLAY),
                    ("30 FPS", 201),
                    ("40 FPS", 202),
                    ("45 FPS", 203),
                    ("60 FPS", 204),
                    ("90 FPS", 205),
                    ("120 FPS", 206),
                    ("144 FPS", 207),
                ],
                &mut g.game_values.framelimiter, g);

            this.miFullscreenField = select_field(spr_selectfield, 70, 200, "Screen Size", 500, 220,
                &[("Windowed", false), ("Fullscreen", true)],
                &mut g.game_values.fullscreen, g);
            g.ctl(this.miFullscreenField).set_auto_advance(true);
            g.ctl(this.miFullscreenField).set_item_changed_code(MENU_CODE_TOGGLE_FULLSCREEN);

            this.miMenuGraphicsPackField = Ctl::new(MI_PacksField::new(spr_selectfield, 70, 240, "Menu Graphics", 500, 220, Ptr::from_mut(&mut g.menugraphicspacklist.simple_directory_list.simple_file_list), MENU_CODE_MENU_GRAPHICS_PACK_CHANGED), g);
            this.miWorldGraphicsPackField = Ctl::new(MI_PacksField::new(spr_selectfield, 70, 280, "World Graphics", 500, 220, Ptr::from_mut(&mut g.worldgraphicspacklist.simple_directory_list.simple_file_list), MENU_CODE_WORLD_GRAPHICS_PACK_CHANGED), g);
            this.miGameGraphicsPackField = Ctl::new(MI_PacksField::new(spr_selectfield, 70, 320, "Game Graphics", 500, 220, Ptr::from_mut(&mut g.gamegraphicspacklist.simple_directory_list.simple_file_list), MENU_CODE_GAME_GRAPHICS_PACK_CHANGED), g);

            if crate::services::services(g).video.supports_shaders() {
                let names: Vec<(String, i16)> = std::iter::once(("Off".to_string(), 0))
                    .chain(g.video.presets.iter().enumerate().map(|(i, p)| (p.name.clone(), i as i16 + 1)))
                    .collect();
                let items: Vec<(&str, i16)> = names.iter().map(|(n, v)| (n.as_str(), *v)).collect();
                this.miShaderField = select_field(spr_selectfield, 70, 360, "Shader", 500, 220, &items, &mut g.video.shader, g);
                g.ctl(this.miShaderField).set_item_changed_code(MENU_CODE_SHADER_CHANGED);
                for failed in g.video.failed.clone() {
                    g.ctl(this.miShaderField).hide_item(failed, true);
                }
                this.miIntegerScaleField = select_field(spr_selectfield, 70, 400, "Integer Scaling", 500, 220,
                    &[("Off", false), ("On", true)],
                    &mut g.video.integer_scale, g);
                g.ctl(this.miIntegerScaleField).set_auto_advance(true);
                g.ctl(this.miIntegerScaleField).set_item_changed_code(MENU_CODE_INTEGER_SCALE_CHANGED);
            }

            this.miGraphicsOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miGraphicsOptionsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miGraphicsOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miGraphicsOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miGraphicsOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Graphics Options Menu", 320, 5), g);
        }

        let toplayer = this.miTopLayerField.id();
        let framelimiter = this.miFrameLimiterField.id();
        let fullscreen = this.miFullscreenField.id();
        let menugfx = this.miMenuGraphicsPackField.id();
        let worldgfx = this.miWorldGraphicsPackField.id();
        let gamegfx = this.miGameGraphicsPackField.id();
        let back = this.miGraphicsOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, toplayer, back, framelimiter, null, back);

        this.add_control(g, framelimiter, toplayer, fullscreen, null, back);
        this.add_control(g, fullscreen, framelimiter, menugfx, null, back);
        this.add_control(g, menugfx, fullscreen, worldgfx, null, back);

        this.add_control(g, worldgfx, menugfx, gamegfx, null, back);
        if this.miShaderField.id() == ControlId::NONE {
            this.add_control(g, gamegfx, worldgfx, back, null, back);
            this.add_control(g, back, gamegfx, toplayer, gamegfx, null);
        } else {
            let (shader, intscale) = (this.miShaderField.id(), this.miIntegerScaleField.id());
            this.add_control(g, gamegfx, worldgfx, shader, null, back);
            this.add_control(g, shader, gamegfx, intscale, null, back);
            this.add_control(g, intscale, shader, back, null, back);
            this.add_control(g, back, intscale, toplayer, intscale, null);
        }

        let c = this.miGraphicsOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miGraphicsOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miGraphicsOptionsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, toplayer);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
