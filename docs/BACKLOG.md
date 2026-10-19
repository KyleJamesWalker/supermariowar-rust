# Backlog

Work we intend to do but have not started, newest ideas last in each section. Bugs in the C++ original are tracked separately in `upstream/UPSTREAM_BACKLOG.md`. When an item starts, it moves to a design doc or a PR and leaves this list.

## Features

| Item | Notes |
|---|---|
| Split built-in controller (muOS) | Two players on one handheld: a Controls option "Built-in controller: single / split". Player 1 gets the D-pad, L1 jump, L2 run and Select item; player 2 gets Y/A/B as left/right/down, X jump, R1 run and R2 item; Start is shared. An optional `--split-pad` flag for the launcher. |
| Gameplay stats | Scope not decided. Open questions: which stats (stomps, deaths, kills by weapon, items used, mode objectives), per match or lifetime, where they show (scoreboard, a menu screen), and whether they persist. They must stay out of the simulated state, or be derived from replays, so goldens do not change. |

## CPU AI

Steps 4, 6 and 9 of `design/CPU_AI.md` have a first cut. Still open from its Risks and open questions:

- Riding a moving platform along its path: the navigation graph sees a platform only where it is. Planning rides along the whole path was mixed in the benchmark.
- Springs and breakable blocks in the navigation graph.
- Note-block super jumps: they always rise to full height, so CPUs aiming at a lower ledge overshoot. Planning them as longer jump links, and letting go of jump above a lower target, were both mixed in the benchmark.

## Maintenance

- `soft_surface::color_key_with_blend_matches_sdl` aborted once on a misaligned SDL surface pointer under a full workspace test run; it passed 40 of 40 runs alone.
