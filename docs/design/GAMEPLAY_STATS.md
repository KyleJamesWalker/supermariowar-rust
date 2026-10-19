# Gameplay stats

Status: first cut. The game counts per-player stats in every match, shows them on an after-match page, and keeps lifetime totals for this device's human players under Options > Stats. The counters live in the rollback state, so netplay re-runs cannot double count, and nothing in the simulation reads them, so no replay plays differently.

## What it counts

Per player slot, per match (`smw/stats.rs`, `PlayerStats`):

| Group | Counters |
|---|---|
| Combat | kills, deaths, suicides; kills and deaths by kill style (stomp, fireball, shell, hammer, bobomb and the rest of `KillStyle`) |
| Head-to-head | kills of each other slot, per match only |
| Items | powerups collected, items stored and used, item boxes bumped |
| Movement | jumps, distance run, time in the air, star time |
| Streaks | longest kill streak, longest life, double and triple kills |
| Mode | the mode's own score at game over (flags, coins, tag time and so on, as the scoreboard counts them) |

Double and triple kills count kills of other players in one airtime: any kill credited to a player while it is in the air, projectiles included. Each airtime counts once, at its highest tier: two kills make a double, three or more a triple and no double. Landing on ground, a platform, a spring, a throw block or an on/off block ends the airtime, at the same points that reset the game's own air combo (`killsinrowinair`), and so does dying. A kill made on the ground ends it too. The on-screen Double Kill and Triple Kill awards are different: they count kills without dying.

The hooks sit where the game already decides each event: `CPlayer::killed_player` and `player_killed_player` (kills and suicides, only when the victim dies), `CPlayer::die` (deaths), `CPlayer::jump`, `set_powerup`, `set_stored_powerup`, `try_releasing_powerup`, the powerup block's bump, and one `stats::tick` per simulation tick after the players move.

## Where it shows

- **After the match.** At game over, the first Start or Back opens the Match Stats page over the scoreboard; the next one leaves the match as before. With only CPUs, `auto_advance` presses Enter after 3 s on each page, as on the other result screens.
- **Options > Stats.** Lifetime totals, bests, and the most played mode and map with wins. Reset Stats asks first, with No selected.

## Lifetime totals

- One set per device, in `stats.txt` in the settings directory that `Storage::settings_dir` names, followed by `Storage::persist` (IDBFS on the web). Android and muOS use the same directory as `options.bin`.
- The format is text: a `smw-stats 1` line, then tab-separated `total`, `best`, `mode` and `map` rows. A file with another first line reads as empty, so a later format can change everything after it.
- Only human players on this device add to it: a player with no CPU AI whose keys are local. In a net match each device adds its own players only. CPUs show on the after-match page but never in the totals.
- Local, world, tour and net matches count when they reach game over and the player leaves the stats page. A match left through the exit dialog does not count.
- Replays, `SMW_REPLAY`, `SMW_DUMP`, `SMW_LIVE_SCRIPT`, `SMW_FRAMES` and `SMW_SEED` runs (the harness, headless tests and the live-script tests) neither add to the totals nor write the file.

## Determinism and rollback

The match counters are `game_values.stats`. `MatchState` already copies `game_values`, so a GGRS load restores the counters with the rest of the match. A re-simulated tick then counts its events once, and a mispredicted tick's events vanish with the state that held them. The checksum hashes the harness dump, which prints no stats, so the counters cannot cause a desync report.

The alternative, committing only confirmed frames, needs a second copy of the counters and the confirmed-frame bookkeeping in `net_v2.rs`, for the same result.

The lifetime commit happens when the player leaves the stats page after game over. By then the game-over tick is far past GGRS's 8-tick prediction window, so it is confirmed.

`rollback_keeps_match_stats` (`crates/smw-headless/tests/replays.rs`) runs 400 ticks of `cpu_ctf` under the sync test, which reloads and re-runs 7 ticks every tick, and checks that the stats match a plain run of the same ticks.

The page and the Stats screen are menu flow only. The new `stats_page` and `stats_menu` replays cover them; every other replay and sweep map keeps its golden.

## Open

- Per-mode objective counters beyond the mode score, such as flags captured versus flags returned.
- A stats line in the tournament and tour scoreboards.
- Recordings made before this change that press Start at game over now open the stats page instead of leaving the match.
