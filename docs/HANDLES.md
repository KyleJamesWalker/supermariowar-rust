# Typed handles replace the `Ptr` object graph

Status: accepted. This is decision 9 of `ARCHITECTURE_V2.md`, which it left to a separate document.

## Summary

The port links its objects with `Ptr<T>`, a copyable raw pointer that the C++ translation needed. This proposal replaces the pointers with typed handles into tables that `Game` owns: players, game objects, map blocks, sprites and menu controls. The work is one category per PR, and each PR is behavior-preserving under the replay gates. After the last PR, `Game` can derive `Clone`, which in-process replay tests and rollback netplay both need.

## Background

`docs/ARCHITECTURE.md` (Pointers and ownership) sets the current convention. Non-owning C++ pointers are `Ptr<T>`, owning ones are `Ptr::new_box` plus `delete()`, and every struct reachable through a `Ptr` embeds `Aliased` so rustc emits no `noalias`. Phase 5 removed the global `Game`, so every function now reaches state through `g`. The object graph inside `Game` is still pointers.

Counted on the branch of #153, smw-core holds 1,328 `Ptr<...>` uses in 207 files:

| Pointee | Uses | Owner today |
|---|---|---|
| `CPlayer` | 388 | `Game.players: Vec<Ptr<CPlayer>>`, created per match, deleted at the match end |
| `gfxSprite` | 180 | Fields of the resource manager, alive for the whole run |
| `dyn` traits (objects, blocks, game modes, controls) | 153 | Object containers, the map's block grid, `Game.gamemodes`, menus |
| Menu controls (`MI_*`, `UI_Menu`) | about 450 | The menu that created them |
| Other (`CPlayerInput`, `MovingPlatform`, `Spotlight`, ...) | about 150 | Various |

## Problem

- **Nothing can clone or snapshot `Game`.** A copy of `Game` would share every pointee with the original. Running two games in one process, for parallel replay tests or a rollback netplay prediction, needs a deep copy that pointers cannot give.
- **Dangling pointers are silent.** The `owner_throw` fix in `PROGRESS.md` (a thrown flag kept pointing at a removed player) is one known case. A stale `Ptr` reads freed memory and does not fail.
- **`unsafe` and `Aliased` stay everywhere.** Nothing checks a dereference, and the `Aliased` marker turns off optimizations the compiler could otherwise make.

## Proposed design

Each category gets a table in `Game` and a `Copy` handle type. Code that holds a `Ptr<T>` today holds the handle and looks the value up through `g`.

1. **Players.** `PlayerId(u8)` indexes `Game.players: Vec<CPlayer>`. A match creates the players and ends by clearing the table. A handle carries the match number, so a lookup after the match ends returns `None` instead of reading freed memory.
2. **Game objects.** Each `CObjectContainer` becomes a slot table of `Box<dyn CObjectTrait>` plus a `Vec<ObjectId>` in insertion order. `ObjectId` is container, slot and generation. `clean_dead_objects` frees slots and bumps their generation. Iteration walks the order vector, which keeps C++'s list order and therefore the RNG draw order.
3. **Map blocks.** The block grid holds `Option<ObjectId>` into a block table that the map owns.
4. **Sprites.** `SpriteId(u16)` indexes a table in the resource manager. Sprites live for the whole run, so these handles never go stale.
5. **Menu controls.** Each menu owns `Vec<Box<dyn UI_ControlTrait>>`, and its fields hold `ControlId` indices into it. Navigation links (up, down, left, right) become `Option<ControlId>`.

When a category no longer has `Ptr` uses, its types drop `Aliased`. When the last one goes, `Game` derives `Clone`. `Box<dyn Trait>` members clone through a `clone_box` method on each trait.

## Alternatives considered

- **Keep `Ptr` and write a custom deep copy.** A copy would have to rebuild every pointer into the new allocation. That is the same table work, without the safety the handles give.
- **`Rc<RefCell<T>>`.** This adds reference counts and run-time borrow checks to every access. It still cannot clone a graph without aliasing the copy and the original.
- **An ECS (`bevy_ecs`).** Entities and components would fit the objects well, and the repository name points that way. It is a much larger rewrite of game logic that is a faithful C++ translation today. Handles are a smaller step that does not block an ECS later.
- **Index into the existing `Vec`s with no generation.** This is simpler, but it turns today's silent use-after-free into a silent wrong object. The generation turns it into a `None` the code has to handle.

## Risks and open questions

- **Borrow conflicts.** Some code holds `&mut self` on one object while it mutates another through a `Ptr`. That code has to split the borrow: take the value out of the table, update it, put it back. The phase 5 tools (`thread.py`, borrow fix-ups) cover the mechanical part. Some call sites need restructuring by hand.
- **Order and determinism.** Every container must keep its current iteration order, or the RNG draw order changes. The replays, the sweep and the `R calls=` count catch a change on the first frame it happens.
- **Checkpoints.** `checkpoint.rs` saves and restores game state for segment replays. Handles are plain integers, so a checkpoint can store them directly. The checkpoint format version changes, and `tools/checkpoint_fixtures` needs regenerating once.
- **Performance.** A table lookup replaces a pointer dereference. Generation checks are one compare. The frame time on the RG35XX is the number to watch.
- **Open question:** whether player handles need a generation, or whether clearing the table at match end is enough. The `owner_throw` case says a generation is worth one byte.
- **Open question: what a copy of `Game` shares.** Step 5 removes the pointers between parts of the game state, but `Game` also holds things a copy cannot sensibly duplicate. Each needs a decision before `Game` can implement `Clone`:
  - **Platform services** (`services`: video, audio, events, storage, pads). Only one game can own the window and audio device. Options: the copy gets none and runs headless (`smw-headless`), or the copy shares the original's.
  - **Loaded assets** (`rm` sprites and sounds, the file lists, `g_tilesetmanager`, fonts). Nothing in a match writes them, except that platforms paint their own sprites. Options: share them behind a reference count, or copy them.
  - **Surfaces** (`screen`, `blitdest`, a platform's `sprites`). The copy needs its own, or must not draw.
  - **Menus and editors** (`ms`, `controls`, `level_editor`, `world_editor`). A rollback copy only needs the match. Options: copy them too, or clone only the match state into a game that has no menus.

  `checkpoint.rs` already saves and restores the match state for segment replays, so a checkpoint loaded into a fresh headless `Game` is a second way to copy a match, with no `Clone` at all.

  A copy for rollback netplay or an in-process replay test needs the match state, a headless service set and the shared assets. The proposal for step 6 is to clone exactly that: `Game::clone_match`, which shares assets, gives the copy headless services, and leaves menus and editors out.

## Rollout

1. Sprites (the simplest, never stale).
2. Players.
3. Game objects and map blocks, one container kind per PR.
4. Menu controls, one menu per PR or a few small menus together.
5. Drop `Aliased` from types with no `Ptr` left, then derive `Clone` for `Game`.
6. Add an in-process test that clones a `Game` mid-replay and checks both copies reach the same dump.

Every PR runs the 51 replays, the 290-replay sweep, the editor scripts, the net games and the segment checks. Each step leaves the game working, so the work can pause between any two PRs.
