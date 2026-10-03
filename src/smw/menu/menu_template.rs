//! Port of src/smw/menu/MenuTemplate.cpp

use crate::common::uimenu::UI_Menu;

/*
    TODO: Description.
*/
#[derive(Default)]
pub struct UI_XXX {
    pub ui_menu: UI_Menu,
}
crate::impl_base!(UI_XXX => ui_menu: UI_Menu);

impl UI_XXX {
    pub fn new() -> Box<Self> {
        Box::<Self>::default()
    }
}
