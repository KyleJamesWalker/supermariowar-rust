# Porting conventions

Rules every agent follows when translating `~/work/supermariowar/src` (C++, read only) into this crate. The goal is frame-for-frame identical behavior under differential replay (`REPLAY.md`), so **translate, do not reinterpret**: same names, same order of operations, same numeric types, same quirks and bugs.

## Layout

- `src/common/...` and `src/smw/...` mirror the C++ tree one-to-one. One Rust module per C++ `.cpp`/`.h` pair, named by snake-casing the C++ basename: `map.cpp` → `common/map.rs`, `MapReader15xx.cpp` → `common/map/map_reader15xx.rs`, `GSGameplay.cpp` → `smw/gs_gameplay.rs`, `MO_Fireball.cpp` → `smw/objects/moving/mo_fireball.rs`. Exception: `Coins.h` + `Coin.cpp` → `smw/gamemodes/coin.rs`.
- When a file and a directory share a name (`common/map.cpp` + `common/map/`, `common/gfx.cpp` + `common/gfx/`), the file module declares the directory's children (`common/map.rs` holds `pub mod map_reader;` ...). There is no `mod.rs` for those directories.
- Every module already exists as a stub (generated); fill it in, do not create parallel files. Every module starts with `//! Port of src/<path>` naming the `.cpp` (or the `.h` if header-only).
- `src/lib.rs` is the crate root (so tests and tools can link it); `src/main.rs` only calls `smw::smw::main::main()`.
- `src/globals.rs` holds the global-state infrastructure (`Global<T>`, `Ptr<T>`, `Aliased`) and re-exports every C++ global, so code writes `use crate::globals::*;`.
- Not ported now: `smw/platform/` (ENet), `leveleditor/`, `worldeditor/`, `server/`, `common_netplay/`.

## Names

| C++ | Rust |
|---|---|
| Classes, structs, typedefs, enums (`CMap`, `CPlayer`, `MO_Fireball`, `gfxSprite`) | identical |
| Fields, locals, parameters, globals (`fx`, `iNetworkID`, `game_values`, `g_map`) | identical |
| Free functions and methods (`LoadCurrentMapBackground`, `getInteger`, `GetState`) | snake_case (`load_current_map_background`, `get_integer`, `get_state`) |
| `#define` constants, `static const` | `pub const` with identical name |
| Unscoped `enum` | `pub type Name = i32;` + `pub const enumerator: Name = n;` (identical names). C++ stores these in `short` fields; cast at the use site exactly where C++ converts. |
| `enum class` | `#[repr(i32)] #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)] pub enum` with identical variant names |
| Overloads | keep the most common unsuffixed; suffix the others by what differs (`get_integer`, `get_integer_range`; `convert_path`, `convert_path_pack`) |
| Default arguments | pass every argument explicitly at the call site |

The crate root sets `#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, static_mut_refs, dead_code, ...)]`, so C++ names compile without noise.

## Numeric semantics

- Types: `short`→`i16`, `unsigned short`→`u16`, `int`→`i32`, `unsigned`/`unsigned int`→`u32`, `long`→`i64`, `char`→`i8`, `unsigned char`/`Uint8`→`u8`, `Uint32`→`u32`, `size_t`→`usize`, `float`→`f32`, `double`→`f64`, `bool`→`bool`.
- **Integer promotion.** C++ computes `short`/`char` arithmetic in `int`. Write `(a as i32 + b as i32) as i16` wherever an intermediate can leave the narrow range or feeds a division, comparison or shift. Integer `/` and `%` truncate toward zero in both languages; keep them as-is.
- **Double literals.** An unsuffixed C++ literal (`0.5`, `1.0`) is a `double` and drags the whole expression into `f64`: `fx = fx + 0.5;` is `fx = (fx as f64 + 0.5) as f32;`. `0.5f` is `0.5f32`. Getting this wrong breaks bit-exact replay. Same for `<cmath>`: `sin(float)` with `using namespace std` resolves to the `float` overload, `::sin` / `sin` of a double to `f64`; check which one the C++ calls.
- **Float→int.** `static_cast<short>(f)` truncates toward zero; Rust `f as i16` matches in range (Rust saturates out of range; C++ is UB). Keep the C++ cast target: `(short)f` is `f as i16`, not `f as i32 as i16`, unless the C++ goes through `int`.
- **Overflow.** `overflow-checks = false` in every profile, so arithmetic wraps like the compiled C++. Use explicit `wrapping_*` where C++ relies on unsigned wraparound (RNG, hashes), as documentation.
- **Evaluation order.** C++ argument evaluation order is unspecified; the reference build is clang, which evaluates left to right. When arguments have side effects (e.g. two `RANDOM_INT` calls), hoist them into locals in left-to-right order.
- **Randomness.** All game randomness goes through `RANDOM_INT` / `RANDOM_BOOL` (`common::random_number_generator`). Never use another RNG.
- **Exceptions.** `throw` is a panic carrying the C++ payload (`std::panic::panic_any(String)`, see `file_io::throw_runtime_error`); `try { } catch` is `std::panic::catch_unwind(AssertUnwindSafe(|| ...))` at the same site. Signatures do not change to `Result`.
- **Strings.** `std::string` and `char[N]` fields become `String`; where C++ truncates into a fixed buffer, truncate the same way at the write site.
- Array indexing panics where C++ would read out of bounds; that is intended (it surfaces bugs) unless the C++ out-of-bounds read is load-bearing, in which case reproduce the value it reads and comment why.

## Global state

The game is single-threaded. C++ globals are `pub static mut` items with the C++ name, **declared in the Rust module that mirrors the C++ file defining them** (`common/global.rs` for `game_values`, `g_map`, `rm`; `smw/main.rs` for `screen`, `blitdest`, `list_players`, `objectcontainer`...), and re-exported from `globals.rs`. Code touches them inside `unsafe { }`; functions keep their safe signatures and open one `unsafe` block around the body when needed.

| C++ global | Rust |
|---|---|
| Scalar, POD array, const-initializable value (`short x_shake = 0;`, `short g_iSwirlSpawnLocations[4][2][25];`) | `pub static mut x_shake: i16 = 0;` |
| Class instance constructed at startup (`CGameValues game_values;`, `CObjectContainer objectcontainer[3];`) | `pub static mut game_values: Global<CGameValues> = Global::uninit();` initialized by `game_values.init(CGameValues::new())` in `globals::init_globals()` in C++ definition order. `Global<T>` derefs to `T`, so call sites read `game_values.gamemode` exactly like C++. |
| (trap) `Global<T>` has its own `init(v)`, `is_initialized()`, `as_ptr()`; call a same-named method of `T` as `CGameValues::init(&mut game_values)`. | |
| Pointer global (`CMap* g_map;`, `CResourceManager* rm;`) | `pub static mut g_map: Ptr<CMap> = Ptr::null();` assigned with `g_map = Ptr::new_box(CMap::new())`; `Ptr<T>` derefs to `T`, so `g_map.map(x, y)` mirrors `g_map->map(x, y)`. |
| Function-local `static` | `static mut` inside the function, same name |

## Pointers and ownership

- **Non-owning pointers** (`CPlayer* owner`, `IO_Block*`, `CObject*` in lists, `this` passed to another object) are `Ptr<T>` (`globals::Ptr`, a nullable `Copy` raw pointer that derefs to `T`, works for `dyn Trait`, compares by address). `NULL` is `Ptr::null()`, `p == NULL` is `p.is_null()`, `this` is `Ptr::from_mut(self)`. One convention everywhere: never indices, never `Rc`, never references stored in structs.
- **Owning pointers** that C++ `new`s and `delete`s: `Box<T>` / `Vec<Box<T>>` when nothing else stores a pointer to the object, otherwise `Ptr::new_box(v)` (leaked `Box`) and `p.delete()` at the C++ `delete` site. Objects never move once other code may point at them, so never hold such objects by value in a `Vec<T>`.
- **Aliasing.** C++ freely mutates an object through a global or another object's pointer while one of its own methods is running. In Rust that violates the `noalias` that `&mut self`/`&self` carry and LLVM *will* miscompile it. Every class struct that is reachable through a `Ptr` or a global therefore embeds `pub _alias: Aliased` (a zero-sized `UnsafeCell<()>` + `PhantomPinned`), which makes the type `!Freeze + !Unpin`; rustc then emits no `noalias` for references to it. Put it in base structs (`CObject`, `CPlayer`, `CMap`, `CGameValues`, `CGameMode`, menu items...); derived structs inherit it through their base field.
- Never keep a `&mut` to global state alive across a call that may touch the same object through another path; re-derive it from the `Ptr` / global after the call, as the C++ effectively does.

## Inheritance and virtual dispatch

- **Data**: the derived struct embeds its base as its first field, named after the base type in snake case (`pub io_moving_object: IO_MovingObject`), and implements `Deref`/`DerefMut` to it, so `self.fx`, `self.velx`, and non-virtual base methods resolve as in C++ (`impl_base!(MO_Fireball => io_moving_object: IO_MovingObject)` generates this).
- **Virtuals**: each polymorphic root gets a trait named `<Class>Trait` (`CObjectTrait`, `IO_MovingObjectTrait: CObjectTrait`, `CGameModeTrait`, `UI_ControlTrait`) holding every virtual method. A virtual with a body in class `C` is written once as a free function generic over the trait (`pub fn io_moving_object_update<T: IO_MovingObjectTrait + ?Sized>(this: &mut T)`); the trait's default method calls it, and an override that does `IO_MovingObject::update()` calls the same function. This keeps virtual calls made from base-class code dispatching to the most derived override.
- Each trait exposes `fn base(&mut self) -> &mut <BaseStruct>` style accessors so default methods can reach fields.
- **Containers** hold `Ptr<dyn CObjectTrait>` (or `Box<dyn CObjectTrait>` when the container owns), mirroring `CObject*`.
- **`dynamic_cast`**: the root trait has `fn as_any(&mut self) -> &mut dyn Any` for concrete types, plus `fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> { None }`-style hooks for intermediate bases. `getObjectType()` / `movingObjectType` checks stay as in C++.
- Pure-virtual → trait method without default. Destructors with side effects → `Drop` only if C++ relies on them at `delete`; otherwise do the work at the `delete` site.

## SDL and graphics

- One global SDL context (owned by the gfx port). Drawing is software: the C++ blits onto the 640x480 `SDL_Surface* screen` / `blitdest`, then presents. The port does the same with `sdl2::sys` FFI on `*mut SDL_Surface` (`SDL_UpperBlit`, `SDL_FillRect`, `SDL_SetColorKey`...), calling the same SDL functions in the same order, so screenshots match pixel for pixel. Safe `sdl2` wrappers are fine for init, window, events and audio.
- `screen`, `blitdest`, `x_shake`, `y_shake` live in `smw/main.rs` (`pub static mut screen: *mut SDL_Surface`).
- Until gfx lands, drawing code in non-gfx modules leaves `// TODO(gfx)` at each unported draw call; logic must not depend on it.

## Data root

`RootDataDirectory` (`common/global.rs`) is computed like C++: `GetRootDirectory()` (directory of the executable, `SDL_GetBasePath`) + `"data"`, overridden by `--datadir <DIR>` (`common/cmd_args.rs`). Tests use the repository's `data/` (`env!("CARGO_MANIFEST_DIR")/data`); running the binaries from `target/` needs `--datadir data`. Every file path goes through `convert_path` / `convert_path_pack` (`common/path.rs`), as in C++.

## Tests and commits

- Unit tests live in `#[cfg(test)] mod tests` at the bottom of the module. Reference values come from compiling the original C++ (clang++) in `tools/`, never from the Rust code itself.
- `cargo build` must stay error-free at every commit; warnings are tolerated.
- Commit only your own paths (`git add <paths> && git commit -m "feat(port): ..."`). If `.git/index.lock` exists, wait and retry. Never push, amend, rebase or stash.
- Dependencies come from crates.io only; check `Cargo.lock` for private-registry URLs before committing it.
- Record finished modules in `PROGRESS.md`.
