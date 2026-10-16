# Pads and the recorder move into the backend

Status: proposed. It finishes phase 3 of `ARCHITECTURE_V2.md` and unblocks phase 6 (`smw-sdl3`).

## Summary

After #153, smw-core still uses SDL in three places. They are the game-controller code in `pad.rs`, the SDL virtual joysticks that replays attach, and the recorder's SDL event filter. This proposal moves the recorder onto normalized `Input`, replaces virtual joysticks with a core device table, and moves `pad.rs` into `smw-sdl2` behind a `Pads` trait. Four PRs do it. The last one removes `sdl2` from smw-core's dependencies, and the pad PR needs testing on real devices before it merges.

## Background

Since #153, the game polls, waits for and pushes input through `Services.events`, and asks `Services.keyboard` for key state. The replay harness, the editor harness and the touch overlay push `Input` values, not SDL events. Pixels and image files no longer touch SDL.

What remains, by file:

| Code | SDL use | What it does |
|---|---|---|
| `smw/pad.rs` (770 lines) | `SDL_Joystick*`, `SDL_GameController*`, an SDL event filter | Opens pads, reads controller mappings, translates raw joystick events through the mapping (`SMW_PAD_TRANSLATE`), adds the muOS mappings, handles hotplug |
| `Game.joysticks`, `main.rs::init_joysticks` | `SDL_JoystickOpen`, `SDL_NumJoysticks` | The table of open joysticks, indexed by the device number bindings use |
| `smw/harness.rs` | `SDL_JoystickAttachVirtual`, `SDL_SetHint`, the recorder filter | Gives replays virtual joysticks. While recording, the filter holds raw input until the frame start turns it into replay lines |
| `mi_input_control_container.rs`, `mi_team_select.rs`, `gfx.rs` | Joystick names, instance IDs and LEDs | The controls menu and team colors |

The web build already runs replays without virtual joysticks. There, a replay's devices exist only as the device numbers its events carry (`web_replay_device`, `main_loop.web_removed`).

## Problem

These three uses keep `sdl2` in smw-core's `Cargo.toml`, which blocks two planned steps:

- **An SDL3 backend.** `smw-sdl3` cannot replace `smw-sdl2` while smw-core calls SDL2 directly. SDL3 also renamed the game-controller API (`SDL_Gamepad`), so `pad.rs` cannot simply compile against SDL3.
- **Core tests without SDL.** `cargo test -p smw-core` still links SDL2 and needs the dummy video and audio drivers.

The recorder filter is also the last place where game logic runs inside an SDL callback. It calls `pad::translate` and `touch::filter` from SDL's event thread context.

## Proposed design

### 1. The recorder reads `Input`

A core input queue sits between `Services.events` and the game:

- `input::poll(g)` replaces direct calls to `services(g).events.poll()`. It is the only reader of the backend queue.
- While recording, `input::poll` moves every live `Input` from the backend into the recorder's pending list. The frame start turns pending inputs into replay lines and pushes those, exactly as the filter does today.
- `touch::filter` runs on the `Input` in `input::poll`, not in an SDL filter.

The recorder filter, `INJECTING` and `to_replay_event`'s SDL argument go away. `to_replay_event` takes an `Input`.

### 2. A core device table replaces virtual joysticks

`Game.joysticks` becomes `Game.devices: Vec<Option<Device>>`, indexed by the device number in bindings and replay lines. A `Device` is either a backend pad (with its backend handle) or a replay device (a number only, as on the web today).

- A replay opens no backend pads. Its `jadd`/`jremove` lines add and remove replay devices in the table.
- Replays stop attaching SDL virtual joysticks and stop setting the `SDL_JOYSTICK_*` hints.
- The native and web replay paths become one path.

### 3. `pad.rs` moves into `smw-sdl2` behind `Pads`

`smw-platform` gains:

```rust
pub trait Pads {
    /// Opens every connected pad. `translate` turns on mapping translation (`SMW_PAD_TRANSLATE`).
    fn open_all(&mut self, translate: bool) -> Vec<PadInfo>;
    /// Pads connected or removed since the last call.
    fn hotplug(&mut self) -> Vec<PadChange>;
    fn set_led(&mut self, pad: PadSlot, rgb: [u8; 3]);
}
```

`PadInfo` carries the name, GUID and whether the pad is a translated controller. The mapping tables, `read_binds`, `translate_*` and the muOS mappings move to `smw-sdl2` unchanged. The backend applies translation before it hands out `Input`, so the core never sees untranslated controller events. The core keeps the policy: `player_pads`, `assign_inputs` and hotplug assignment.

### 4. smw-core drops `sdl2`

With 1-3 done, `smw-core/Cargo.toml` loses `sdl2`. The SDL comparison tests in `soft_surface.rs`, `soft_image.rs` and `editor_event.rs` move to `smw-sdl2`, or keep `sdl2` as a dev-dependency.

## Alternatives considered

- **A thin trait that mirrors SDL's joystick API**, with `pad.rs` kept in core. This is the smallest change, but it puts SDL2's controller concepts into `smw-platform`. SDL3 renamed and changed that API, so `smw-sdl3` would have to emulate SDL2 behind it.
- **Keep SDL virtual joysticks for native replays.** This keeps today's replay behavior exactly. It also keeps native and web replays on different device paths, and leaves smw-core depending on SDL.
- **One large PR.** Existing CI gates cover the recorder and device table changes. The pad move needs hardware testing. Splitting the PRs keeps the hardware-only risk in one PR.

## Risks and open questions

- **Hardware-only paths.** The muOS mappings, Android pads and macOS hotplug are not covered by CI. The pad PR needs a manual pass on an RG35XX, a Pixel, and an Xbox or PlayStation pad on macOS.
- **Event order within a frame.** Today the SDL filter sees events in SDL's queue order, and translation can drop or rewrite them in place. Moving translation into the backend's poll must keep that order. Replays with `jadd`/`jremove` and the net games check this.
- **Replays that relied on SDL virtual joystick behavior.** For example, an instance ID that differs from the device index. The 51 replays, the sweep and the segment checks cover the recorded cases. Native and web replays already agree on the determinism report, which suggests the device-number path is enough.
- **Open question:** whether live recording round trips need a new CI test before step 1. Nothing in CI records live input today. A test that drives `SMW_LIVE_SCRIPT` and compares the recording's replay dump with the live dump would cover it.

## Rollout

1. Add a CI test for live recording round trips (`SMW_LIVE_SCRIPT` dump equals replay dump).
2. PR: the recorder reads `Input` (design step 1). Gates: replays, sweep, editors, net games, the new round-trip test.
3. PR: the core device table replaces virtual joysticks (step 2). Gates: the same, plus every replay with `jadd`/`jremove` lines.
4. PR: `pad.rs` into `smw-sdl2` (step 3). Gates: the same, plus the device pass under Risks. This PR waits for that pass before it merges.
5. PR: smw-core drops `sdl2` (step 4).

A revert of any one PR leaves the others working. Steps 2 and 3 are independent of each other, so either can go first.
