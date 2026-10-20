//! Option submenus reached from the options menu, plus helpers they share.

use crate::game::Game;
pub mod eye_candy_options_menu;
pub mod gameplay_options_menu;
pub mod graphics_options_menu;
pub mod powerup_drop_rates_menu;
pub mod powerup_settings_menu;
pub mod projectile_limits_menu;
pub mod projectile_options_menu;
pub mod sound_options_menu;
pub mod stats_menu;
pub mod team_options_menu;

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::globals::*;

/// Builds a select field with `items`, bound to write the selection through `output` and showing its current value.
/// Safety: `output` must stay valid for the field's lifetime.
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

/// Items for a boolean Off/On select field.
pub(crate) const OFF_ON: &[(&str, bool)] = &[("Off", false), ("On", true)];
