//! Port of src/smw/menu/MapFilterEditMenu.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_map_browser::MI_MapBrowser;

#[derive(Default)]
pub struct UI_MapFilterEditMenu {
    pub ui_menu: UI_Menu,

    pub miMapBrowser: Ctl<MI_MapBrowser>,
}
crate::impl_base!(UI_MapFilterEditMenu => ui_menu: UI_Menu);

impl UI_MapFilterEditMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut gp = g.ptr();
        let mut this = Box::<Self>::default();

        this.miMapBrowser = Ctl::new(MI_MapBrowser::new(), g);
        g.ctl(this.miMapBrowser).set_auto_modify(true);

        let b = this.miMapBrowser.id();
        this.add_control(g, b, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);
        this.set_initial_focus(gp.get(), b);
        this.set_cancel_code(MENU_CODE_MAP_BROWSER_EXIT);
        this
    }
}
