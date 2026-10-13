//! Not in upstream: the family of a connected pad, for Options > Controls (and later button prompts).

use sdl2::sys::{SDL_GameControllerType as T, SDL_GameControllerTypeForIndex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerType {
    Generic,
    Xbox,
    PlayStation,
    Nintendo,
}

impl ControllerType {
    pub fn label(self) -> &'static str {
        match self {
            ControllerType::Generic => "Controller",
            ControllerType::Xbox => "Xbox controller",
            ControllerType::PlayStation => "PlayStation controller",
            ControllerType::Nintendo => "Nintendo controller",
        }
    }

    pub fn from_vendor(vendor: u16) -> ControllerType {
        match vendor {
            0x045e => ControllerType::Xbox,
            0x054c => ControllerType::PlayStation,
            0x057e => ControllerType::Nintendo,
            _ => ControllerType::Generic,
        }
    }
}

/// The vendor ID in a browser's `gamepad.id`: Chrome's "... Vendor: 045e Product: 028e)" or Firefox's "045e-028e-...".
pub fn vendor_from_web_id(id: &str) -> Option<u16> {
    if let Some(i) = id.find("Vendor: ") {
        return u16::from_str_radix(id.get(i + 8..i + 12)?, 16).ok();
    }
    let (vendor, rest) = id.split_once('-')?;
    (vendor.len() == 4 && rest.split_once('-').is_some_and(|(p, _)| p.len() == 4)).then(|| u16::from_str_radix(vendor, 16).ok())?
}

/// The family of the joystick at SDL device index `device`, whose SDL name is `name`.
pub fn controller_type(device: i32, name: &str) -> ControllerType {
    let sdl = match unsafe { SDL_GameControllerTypeForIndex(device) } {
        T::SDL_CONTROLLER_TYPE_XBOX360 | T::SDL_CONTROLLER_TYPE_XBOXONE => ControllerType::Xbox,
        T::SDL_CONTROLLER_TYPE_PS3 | T::SDL_CONTROLLER_TYPE_PS4 | T::SDL_CONTROLLER_TYPE_PS5 => ControllerType::PlayStation,
        T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_PRO
        | T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_JOYCON_LEFT
        | T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_JOYCON_RIGHT
        | T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_JOYCON_PAIR => ControllerType::Nintendo,
        _ => ControllerType::Generic,
    };
    #[cfg(target_os = "emscripten")]
    if let Some(vendor) = vendor_from_web_id(name) {
        return ControllerType::from_vendor(vendor);
    }
    let _ = name;
    sdl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_ids() {
        assert_eq!(vendor_from_web_id("Xbox 360 Controller (STANDARD GAMEPAD Vendor: 045e Product: 028e)"), Some(0x045e));
        assert_eq!(vendor_from_web_id("054c-09cc-Wireless Controller"), Some(0x054c));
        assert_eq!(vendor_from_web_id("Some pad"), None);
        assert_eq!(ControllerType::from_vendor(0x057e), ControllerType::Nintendo);
        assert_eq!(ControllerType::from_vendor(0x1234).label(), "Controller");
    }
}
