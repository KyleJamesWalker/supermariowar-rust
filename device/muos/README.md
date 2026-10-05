# Super Mario War on muOS

`SuperMarioWar-<version>.muxapp` installs the game on Anbernic handhelds running [muOS](https://muos.dev) through Archive Manager. The release workflow (`.github/workflows/release.yml`) builds it for aarch64 and attaches it to each `v*` release.

It targets the 64-bit H700 handhelds such as the RG35XX Plus, H, SP and 2024, whose 640x480 screen matches the game's resolution. The original 2023 RG35XX is 32-bit ARM and is not supported.

## Install

1. Download `SuperMarioWar-<version>.muxapp` from the GitHub release and copy it to `ARCHIVE/` on either SD card (`/mnt/mmc/ARCHIVE` or `/mnt/sdcard/ARCHIVE`).
2. On the device, open **Applications > Archive Manager** and select it. muOS extracts it to `<card>/MUOS/application/SuperMarioWar/`.
3. Launch **Super Mario War** from **Applications**.

To update, install the newer `.muxapp` the same way; it replaces the files it ships and leaves `home/` and `log.txt` alone. If the menus then show the Retro graphics instead of Classic, move `home/` aside, delete the folder, reinstall and put `home/` back: the game takes its default graphics pack from directory order, which an install over an older one keeps. To uninstall, delete `MUOS/application/SuperMarioWar/`.

## What is in the package

```
SuperMarioWar/
├── mux_launch.sh    muOS entry point
├── smw              the game (aarch64, glibc)
├── lib/             libSDL2_image-2.0.so.0, libSDL2_mixer-2.0.so.0
├── data/            game data (supermariowar-data)
└── glyph/smw.png    24x24 menu icon, named by the launcher's "# ICON: smw"
```

`smw` and the bundled libraries require glibc 2.34 or newer (the newest `GLIBC_` symbol they use) and use the device's own `libSDL2`, which carries its GPU driver. `smw` finds `lib/` through its RUNPATH (`$ORIGIN/lib`); the launcher also puts it on `LD_LIBRARY_PATH`. The bundled SDL2_image and SDL2_mixer are built with only their built-in PNG and OGG decoders, so they need nothing else from the device.

## Controls

The built-in controls are player 1's gamepad. The game reads them through muOS's controller mapping, so muOS's controller layout setting decides which physical button is A: with the retro layout A is the button labelled A, with the modern layout A and B (and X and Y) swap places. The table names the mapped buttons:

| Button | In a match | In menus |
|---|---|---|
| D-pad or left stick | Move, Down to duck | Move the cursor |
| A | Jump | Choose |
| B | Run and pick up | Back (on the main menu: quit) |
| X | Use the stored item | Random choice |
| Start | Pause | Scroll fast |
| Select | Exit the match (asks first) | |
| Menu | Quit the game | Quit the game |

Y, the shoulder buttons and the triggers are free to bind under **Controls** in the main menu. `docs/GAME_MANUAL.md` covers the menus and modes.

With one controller, the other players that were human become **Bot** at launch, so a match starts against the computer. Set them to **None** in the main menu's **Players** row for fewer opponents. Each extra controller (a USB or Bluetooth pad) joins as the next human player.

To quit, press **Menu** or choose **Exit** on the main menu. B on the main menu also quits, as Back does there in the original game, so press A to start. Settings are saved on the way out.

## Launcher settings

`mux_launch.sh` sets these variables, which change the game's behaviour on muOS only:

| Variable | Effect |
|---|---|
| `SMW_PAD_TRANSLATE=1` | Reads every pad SDL has a controller mapping for (muOS provides one for the built-in controls) through that mapping, in the button layout above. Mapped pads get players first; Menu quits. |
| `SMW_NO_KEYBOARD=1` | Leaves the keyboard out of the launch-time player assignment and turns the players without a pad that were human into bots. |

Other builds set neither, so desktop pads keep their raw button numbering there.

`SMW_DEBUG_INPUT=1` is off; uncomment its line in `mux_launch.sh` to log a heartbeat every 120 frames, the first 2,000 input events (as SDL queues them, after translation and as the game reads them), and a watchdog line naming the main loop's phase when it stalls for 3 s.

The `AUDIO_DRIVER` line sets `SDL_AUDIODRIVER`, a comma-separated list SDL tries in order. It defaults to `alsa`, the driver muOS's SDL 2.28.5 opens on the RG35XX (it has no PipeWire backend; ALSA's default device forwards to PipeWire). `dummy` runs without sound.

It also sets `SDL_JOYSTICK_DEVICE` (unless already set) to the `/dev/input/eventN` node of `muOS-Keys`, or of every device the kernel gives a joystick (`jsN`) handler, so SDL opens the pad even where its udev enumeration finds none.

## Files on the device

Everything the game writes stays in its folder, `<card>/MUOS/application/SuperMarioWar/`:

| Path | Contents |
|---|---|
| `home/Library/Preferences/.smw/options.bin`, `controls.sdl2.bin` | settings and control bindings |
| `home/Library/Preferences/.smw/replays/` | session recordings, newest 10 (see `docs/REPLAY.md`) |
| `log.txt` | the last session's output, overwritten at each launch |

A recording copied to a computer plays with `smw --replay <file>`.

## Troubleshooting

- **The app does not start or returns straight to the menu.** Read `log.txt`. `error while loading shared libraries` names a library the device lacks; `GLIBC_2.xx not found` means the firmware's glibc is older than 2.34.
- **No icon in the Applications list.** Apps on the second card (`/mnt/sdcard`) need muOS 2508.3 or newer to show their glyph.
- **No sound.** The log's `[sfx]` lines show whether SDL_mixer opened the audio device; the game keeps running without one.
- **Buttons do nothing or the wrong thing.** Turn on `SMW_DEBUG_INPUT` (see Launcher settings). `log.txt` starts with the launcher's view of the input devices (`/proc/bus/input/devices`, `/dev/input`, the `SDL_` variables, whether udevd runs, the muOS version). The game's `[pad]` lines that follow give the joystick count SDL reports and list each joystick and whether it has a controller mapping ("translated"). Rebind under **Controls**; the bindings are saved in `home/`. Bindings saved by a build before controller support point at raw button numbers: delete `home/Library/Preferences/.smw/controls.sdl2.bin` to get the defaults above.

## Building

`build-muxapp.sh` runs on aarch64 Linux (the release workflow uses GitHub's `ubuntu-22.04-arm` runner):

```sh
sudo apt-get install libsdl2-dev cmake libclang-dev patchelf zip
device/muos/build-sdl-libs.sh             # SDL2_image and SDL2_mixer into /usr/local
device/muos/build-muxapp.sh               # builds smw, writes dist/SuperMarioWar.muxapp
device/muos/build-muxapp.sh path/to/smw   # or packages a prebuilt binary
```

It copies `smw`, every library `smw` needs except those the device provides (glibc, libSDL2, GL/EGL/GLES, DRM, ALSA and PulseAudio), `data/`, the launcher and the glyph, then prints the newest `GLIBC_` symbol version the package requires. `MUXAPP_OUT` sets the output path.
