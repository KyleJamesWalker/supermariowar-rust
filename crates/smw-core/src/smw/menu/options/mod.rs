use crate::game::Game;
pub mod eye_candy_options_menu;
pub mod gameplay_options_menu;
pub mod graphics_options_menu;
pub mod powerup_drop_rates_menu;
pub mod powerup_settings_menu;
pub mod projectile_limits_menu;
pub mod projectile_options_menu;
pub mod sound_options_menu;
pub mod team_options_menu;

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::globals::*;

/// `new MI_SelectField<T>(spr, x, y, name, width, indent)`, `add` of each item, `setOutputPtr(output)`,
/// `setCurrentValue(*output)`: the construction sequence every options menu field uses.
pub(crate) unsafe fn select_field<T: Copy + PartialEq + 'static>(
    spr: SpriteId,
    x: i16,
    y: i16,
    name: &str,
    width: i16,
    indent: i16,
    items: &[(&str, T)],
    output: *mut T,
    g: &mut Game,
) -> Ctl<MI_SelectField<T>> {
    let mut f = Ctl::new(MI_SelectField::new(spr, x, y, name, width, indent), g);
    for &(item_name, value) in items {
        g.ctl(f).add(item_name, value);
    }
    g.ctl(f).set_output_ptr(output);
    g.ctl(f).set_current_value(*output);
    f
}

pub(crate) const OFF_ON: &[(&str, bool)] = &[("Off", false), ("On", true)];
