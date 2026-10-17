//! SDL2 joysticks as smw-platform's `Pads`.

use sdl2::sys::{SDL_GameControllerType as T, SDL_GameControllerTypeForIndex, SDL_InitSubSystem, SDL_Joystick, SDL_JoystickEventState, SDL_JoystickFromInstanceID, SDL_JoystickFromPlayerIndex, SDL_JoystickName, SDL_JoystickNameForIndex, SDL_JoystickOpen, SDL_JoystickSetLED, SDL_NumJoysticks, SDL_ENABLE, SDL_INIT_JOYSTICK};
use smw_platform::{Input, PadChange, PadFamily, PadInfo, PadSlot, Pads};
use std::ffi::CStr;

pub struct Sdl2Pads {
    #[cfg(not(target_os = "emscripten"))]
    state: Box<crate::pad::PadState>,
}

impl Sdl2Pads {
    pub fn new() -> Self {
        Sdl2Pads {
            #[cfg(not(target_os = "emscripten"))]
            state: Box::new(crate::pad::PadState::new()),
        }
    }
}

impl Default for Sdl2Pads {
    fn default() -> Self {
        Self::new()
    }
}

impl Pads for Sdl2Pads {
    fn open_all(&mut self, open: bool, translate: bool) -> Vec<Option<PadInfo>> {
        unsafe {
            SDL_InitSubSystem(SDL_INIT_JOYSTICK);
            let joysticks: Vec<*mut SDL_Joystick> = if open { (0..SDL_NumJoysticks().max(0)).map(|i| SDL_JoystickOpen(i)).collect() } else { Vec::new() };
            SDL_JoystickEventState(SDL_ENABLE as i32);
            #[cfg(not(target_os = "emscripten"))]
            return crate::pad::init(&mut self.state, open, translate, &joysticks);
            #[cfg(target_os = "emscripten")]
            {
                let _ = translate;
                joysticks.iter().enumerate().map(|(i, &js)| (!js.is_null()).then(|| info(i as i32, js))).collect()
            }
        }
    }

    fn hotplug(&mut self) -> Vec<PadChange> {
        #[cfg(not(target_os = "emscripten"))]
        return crate::pad::hotplug(&mut self.state);
        #[cfg(target_os = "emscripten")]
        Vec::new()
    }

    fn open(&mut self, device: i32, index: usize) -> Option<PadInfo> {
        #[cfg(not(target_os = "emscripten"))]
        return crate::pad::open(&mut self.state, device, index);
        #[cfg(target_os = "emscripten")]
        {
            let _ = index;
            let js = unsafe { SDL_JoystickOpen(device) };
            (!js.is_null()).then(|| unsafe { info(device, js) })
        }
    }

    fn name(&self, index: usize, pad: Option<PadSlot>) -> Option<String> {
        unsafe {
            let p = match pad {
                Some(pad) => SDL_JoystickName(SDL_JoystickFromInstanceID(pad.0)),
                None => SDL_JoystickNameForIndex(index as i32),
            };
            (!p.is_null()).then(|| CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }

    fn family(&self, index: usize) -> PadFamily {
        match unsafe { SDL_GameControllerTypeForIndex(index as i32) } {
            T::SDL_CONTROLLER_TYPE_XBOX360 | T::SDL_CONTROLLER_TYPE_XBOXONE => PadFamily::Xbox,
            T::SDL_CONTROLLER_TYPE_PS3 | T::SDL_CONTROLLER_TYPE_PS4 | T::SDL_CONTROLLER_TYPE_PS5 => PadFamily::PlayStation,
            T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_PRO
            | T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_JOYCON_LEFT
            | T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_JOYCON_RIGHT
            | T::SDL_CONTROLLER_TYPE_NINTENDO_SWITCH_JOYCON_PAIR => PadFamily::Nintendo,
            _ => PadFamily::Generic,
        }
    }

    fn set_led(&mut self, player: i32, rgb: [u8; 3]) {
        unsafe {
            SDL_JoystickSetLED(SDL_JoystickFromPlayerIndex(player), rgb[0], rgb[1], rgb[2]);
        }
    }

    fn log_input(&mut self, input: &Input) {
        #[cfg(not(target_os = "emscripten"))]
        crate::pad::log_event(&mut self.state, 2, &crate::input::to_sdl(input));
        #[cfg(target_os = "emscripten")]
        let _ = input;
    }
}

#[cfg(target_os = "emscripten")]
unsafe fn info(device: i32, js: *mut SDL_Joystick) -> PadInfo {
    let name = SDL_JoystickNameForIndex(device);
    let name = if name.is_null() { String::new() } else { CStr::from_ptr(name).to_string_lossy().into_owned() };
    let mut guid = [0 as std::ffi::c_char; 33];
    sdl2::sys::SDL_JoystickGetGUIDString(sdl2::sys::SDL_JoystickGetGUID(js), guid.as_mut_ptr(), guid.len() as i32);
    let guid = CStr::from_ptr(guid.as_ptr()).to_string_lossy().into_owned();
    PadInfo { pad: PadSlot(sdl2::sys::SDL_JoystickInstanceID(js)), name, guid, controller: false }
}
