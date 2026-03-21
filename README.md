# Super Mario War (Rust/Bevy)

A Rust rewrite of [Super Mario War](https://github.com/mmatyas/supermariowar) using the [Bevy](https://bevyengine.org/) game engine.

Super Mario War is a fan-made multiplayer Super Mario Bros. style deathmatch game in which players try to beat one another in a variety of gameplay modes.

- [About](#about)
- [Building](#building)
  - [Requirements](#requirements)
  - [Build & Run](#build--run)
  - [Release Build](#release-build)
- [Game Data](#game-data)
- [Project Structure](#project-structure)
- [Original Project](#original-project)

## About

Super Mario War is a Super Mario multiplayer game. The goal is to stomp as many other Mario's as possible to win the game. There is a range of different gameplay modes in the game, like Stomp, Survival, and more. Up to four players can play simultaneously.

The game uses artwork and sounds from Nintendo games. We hope that this noncommercial fangame qualifies as fair use work. We just wanted to create this game to show how much we adore Nintendo's characters and games.

## Building

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition or later)
- Cargo (included with Rust)

Bevy's dependencies vary by platform:

- **macOS**: Xcode command line tools (`xcode-select --install`)
- **Linux**: `sudo apt install g++ pkg-config libx11-dev libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev` (Debian/Ubuntu) or equivalent for your distro
- **Windows**: VS2019+ Build Tools or MSVC toolchain

### Build & Run

```sh
cargo run
```

### Release Build

For optimized performance:

```sh
cargo run --release
```

## Game Data

The game assets are in the `data/` directory and include:

- `gfx/` - Graphics and sprites
- `maps/` - Level maps
- `music/` - Background music
- `sfx/` - Sound effects
- `filters/` - Graphics filters
- `tours/` - Tour definitions
- `worlds/` - World definitions

## Project Structure

```
src/
├── main.rs          # App entry point and Bevy plugin setup
├── lib.rs           # Library root
├── ai/              # CPU player AI
├── audio/           # Music and sound effects
├── core/            # Constants, game state, and shared types
├── game_modes/      # Game mode implementations (Stomp, Survival, etc.)
├── graphics/        # Sprite loading and rendering
├── map/             # Map loading and tile systems
├── menu/            # Main menu and UI
├── net/             # Networking
├── objects/         # Game objects (items, powerups, blocks)
├── physics/         # Movement and collision systems
└── player/          # Player components and systems
```

## Original Project

This is a Rust/Bevy port based on the original C++ Super Mario War project by [mmatyas](https://github.com/mmatyas/supermariowar).
