# Super Mario War on muOS

`SuperMarioWar-<version>.muxapp` installs the game on Anbernic handhelds running [muOS](https://muos.dev) through Archive Manager. The release workflow (`.github/workflows/release.yml`) builds it for aarch64 and attaches it to each `v*` release.

## Install

1. Download `SuperMarioWar-<version>.muxapp` from the GitHub release and copy it to `ARCHIVE/` on either SD card (`/mnt/mmc/ARCHIVE` or `/mnt/sdcard/ARCHIVE`).
2. On the device, open **Applications > Archive Manager** and select it. muOS extracts it to `<card>/MUOS/application/SuperMarioWar/`.
3. Launch **Super Mario War** from **Applications**.

To update, install the newer `.muxapp` the same way; it replaces the files it ships and leaves `home/` and `log.txt` alone. To uninstall, delete `MUOS/application/SuperMarioWar/`.

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

The built-in controls are a joystick to the game, and the first joystick is player 1. The game's joystick defaults are by button number, so they may not match the labels on the device:

| | Game | Menus |
|---|---|---|
| Move | D-pad or left stick | D-pad or left stick |
| Button 0 | Jump | Select |
| Button 1 | Turbo | Back (on the main menu: exit the game) |
| Button 2 | Use item | Random |
| Button 3 | Pause | |
| Button 4 | Exit (pause menu, then quit the match) | |

Rebind them under **Controls** in the main menu if they land on the wrong buttons. `docs/GAME_MANUAL.md` covers the menus and modes.

At every launch, players after the connected joysticks are set to the two keyboard sets as human players. A handheld has no keyboard, so before starting a match set players 2 and 3 to **Bot** or **None** in the main menu's **Players** row, or the team selection screen waits for them.

To quit, choose **Exit** on the main menu or press Back there.

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
- **Buttons do the wrong thing.** Rebind them under **Controls**; the bindings are saved in `home/`.
- **Stuck on the team selection screen.** Set the players without a controller to Bot or None (see Controls).

## Building

`build-muxapp.sh` runs on aarch64 Linux (the release workflow uses GitHub's `ubuntu-22.04-arm` runner):

```sh
sudo apt-get install libsdl2-dev cmake libclang-dev patchelf zip
device/muos/build-sdl-libs.sh             # SDL2_image and SDL2_mixer into /usr/local
device/muos/build-muxapp.sh               # builds smw, writes dist/SuperMarioWar.muxapp
device/muos/build-muxapp.sh path/to/smw   # or packages a prebuilt binary
```

It copies `smw`, every library `smw` needs except those the device provides (glibc, libSDL2, GL/EGL/GLES, DRM, ALSA and PulseAudio), `data/`, the launcher and the glyph, then prints the newest `GLIBC_` symbol version the package requires. `MUXAPP_OUT` sets the output path.
