//! Port of src/smw/menu/options/GraphicsOptionsMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::select_field;
use crate::smw::ui::mi_packs_field::MI_PacksField;

/*
    You can switch between windowed and fullscreen mode,
    as well as change the graphic packs here.
*/
#[derive(Default)]
pub struct UI_GraphicsOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miTopLayerField: Ptr<MI_SelectField<bool>>,
    pub miFrameLimiterField: Ptr<MI_SelectField<i16>>,
    pub miFullscreenField: Ptr<MI_SelectField<bool>>,

    pub miMenuGraphicsPackField: Ptr<MI_PacksField>,
    pub miWorldGraphicsPackField: Ptr<MI_PacksField>,
    pub miGameGraphicsPackField: Ptr<MI_PacksField>,
    pub miGraphicsOptionsMenuBackButton: Ptr<MI_Button>,

    pub miGraphicsOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miGraphicsOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miGraphicsOptionsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_GraphicsOptionsMenu => ui_menu: UI_Menu);

impl UI_GraphicsOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miTopLayerField = select_field(spr_selectfield, 70, 120, "Draw Top Layer", 500, 220,
                &[("Background", false), ("Foreground", true)],
                &mut game_values.toplayer);
            this.miTopLayerField.set_auto_advance(true);

            this.miFrameLimiterField = select_field(spr_selectfield, 70, 160, "Frame Limit", 500, 220,
                &[
                    ("10 FPS", 100),
                    ("15 FPS", 67),
                    ("20 FPS", 50),
                    ("25 FPS", 40),
                    ("30 FPS", 33),
                    ("35 FPS", 28),
                    ("40 FPS", 25),
                    ("45 FPS", 22),
                    ("50 FPS", 20),
                    ("55 FPS", 18),
                    ("62 FPS (Normal)", 16),
                    ("66 FPS", 15),
                    ("71 FPS", 14),
                    ("77 FPS", 13),
                    ("83 FPS", 12),
                    ("90 FPS", 11),
                    ("100 FPS", 10),
                    ("111 FPS", 9),
                    ("125 FPS", 8),
                    ("142 FPS", 7),
                    ("166 FPS", 6),
                    ("200 FPS", 5),
                    ("250 FPS", 4),
                    ("333 FPS", 3),
                    ("500 FPS", 2),
                    ("No Limit", 0),
                ],
                &mut game_values.framelimiter);

            this.miFullscreenField = select_field(spr_selectfield, 70, 200, "Screen Size", 500, 220,
                &[("Windowed", false), ("Fullscreen", true)],
                &mut game_values.fullscreen);
            this.miFullscreenField.set_auto_advance(true);
            this.miFullscreenField.set_item_changed_code(MENU_CODE_TOGGLE_FULLSCREEN);

            this.miMenuGraphicsPackField = Ptr::new_box(MI_PacksField::new(spr_selectfield, 70, 240, "Menu Graphics", 500, 220, Ptr::from_mut(&mut menugraphicspacklist.simple_directory_list.simple_file_list), MENU_CODE_MENU_GRAPHICS_PACK_CHANGED));
            this.miWorldGraphicsPackField = Ptr::new_box(MI_PacksField::new(spr_selectfield, 70, 280, "World Graphics", 500, 220, Ptr::from_mut(&mut worldgraphicspacklist.simple_directory_list.simple_file_list), MENU_CODE_WORLD_GRAPHICS_PACK_CHANGED));
            this.miGameGraphicsPackField = Ptr::new_box(MI_PacksField::new(spr_selectfield, 70, 320, "Game Graphics", 500, 220, Ptr::from_mut(&mut gamegraphicspacklist.simple_directory_list.simple_file_list), MENU_CODE_GAME_GRAPHICS_PACK_CHANGED));

            this.miGraphicsOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miGraphicsOptionsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miGraphicsOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miGraphicsOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miGraphicsOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Graphics Options Menu", 320, 5));
        }

        let toplayer = ctl_ptr(this.miTopLayerField);
        let framelimiter = ctl_ptr(this.miFrameLimiterField);
        let fullscreen = ctl_ptr(this.miFullscreenField);
        let menugfx = ctl_ptr(this.miMenuGraphicsPackField);
        let worldgfx = ctl_ptr(this.miWorldGraphicsPackField);
        let gamegfx = ctl_ptr(this.miGameGraphicsPackField);
        let back = ctl_ptr(this.miGraphicsOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(toplayer, back, framelimiter, null, back);

        this.add_control(framelimiter, toplayer, fullscreen, null, back);
        this.add_control(fullscreen, framelimiter, menugfx, null, back);
        this.add_control(menugfx, fullscreen, worldgfx, null, back);

        this.add_control(worldgfx, menugfx, gamegfx, null, back);
        this.add_control(gamegfx, worldgfx, back, null, back);
        this.add_control(back, gamegfx, toplayer, gamegfx, null);

        let c = ctl_ptr(this.miGraphicsOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGraphicsOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miGraphicsOptionsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(toplayer);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }
}
