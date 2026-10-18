# CPU AI overhaul

Status: steps 1 to 3, 5, 7 and 8 are done, and steps 4, 6 and 9 have a first cut (2026-10-18).

## Summary

The CPU players chase stomps and ignore the objective, so most game modes are close to unwinnable with three CPUs.
They also get stuck on walls and ignore height.
This proposal replaces the fixed priority ladder in `crates/smw-core/src/smw/ai.rs` with mode-aware scoring.
It also adds wall and wrap awareness, and an AI-vs-AI benchmark to measure each change.

## Background

`ai.rs` is a line-for-line port of the C++ `CPlayerAI`.
Each frame, `get_nearest_objects` finds the nearest goal, player, threat, stomp target and teammate.
`cplayer_ai_think` then picks one action and steers toward it.

The C++ build is no longer the golden reference.
The invariant is that every Rust build produces the same frames, so the AI may change freely as long as it stays deterministic.

## Problem

**The objective almost never wins**.
The action ladder (`ai.rs:1029`) checks, in order: a threat within 120 px, a stomp target within 120 px, a teammate, then the goal.
Without teammates, any player within 90 px beats the goal.
With three CPUs on one map, a player is almost always that close.
The game mode never changes the order.

**The AI gives up on goals**.
An attention timer (`ai.rs:975`) makes the AI ignore a goal it has chased for too long.
Lower difficulties shorten the timer.

**The AI aims through screen edges on maps without wrap**.
`distance_to_object` (`ai.rs:824`) always treats the screen as wrapping.
If the target is closer across the edge, the AI walks into the side wall and stays there.

**The AI does not see walls**.
The only tile look-ahead is fall and death danger (`ai.rs:1420-1530`).
The AI walks into any solid wall and jumps only when the target is above it.

**The AI ignores height**.
Distance is a plain squared distance.
Nothing rewards standing above an opponent or penalizes standing below one, which is the core of stomp combat.

**Difficulty only adds noise**.
Lower difficulty makes the AI skip a share of its decisions at random (`ai.rs:950`).
It does not change how well the AI plans or times jumps.

**CPUs dogpile**.
Every CPU targets the nearest player, so several can chase the same target while the leader runs free.

## Proposed design

The steps run from most payoff per effort to least.

1. **AI-vs-AI benchmark**.
   A headless tool runs seeded matches per mode and map, and reports objective score, win spread, and frames spent stuck before game over (`tools/ai_bench.py`).
   Every later step must improve its target metric without making another mode worse.
2. **Wrap-aware distance**.
   `distance_to_object` and its siblings consider the wrapped path only when the CPU's rows or the target's rows have no solid tile in either edge column.
3. **Wall look-ahead**.
   If a solid tile blocks the move direction, the AI jumps (`wall_ahead`).
   If the wall rises four tiles above the CPU's head, or the CPU has jumped at it for 45 frames, the AI ignores its goal or opponent for 90 frames, unless that opponent is the only one.
4. **Mode-aware utility scoring**.
   Each candidate action (goal, stomp, flee, regroup) gets a score from mode weights, distance, height difference, and game state.
   The AI picks the highest score.
   Examples: a Capture the Flag carrier scores "go home" above any stomp, and a Star holder scores "flee" above everything.
   First cut: in coins, eggs, Capture the Flag, domination, King of the Hill, race and collection, a stomp target beats the goal only within 60 px and the nearest player only within 45 px.
5. **Height advantage**.
   The score rewards positions above an opponent and penalizes positions below a non-invincible one.
   Weighting target choice by height (below at three quarters, above at five quarters) was mixed in the benchmark, so this needs positioning rather than target choice.
   Done as positioning: a CPU on the ground jumps when the horizontal gap to its target equals the distance it closes while its jump carries its feet up to the target's head and back down (`stomp_lead_jump`), and it steps out from under an opponent falling onto it (`dodge_stomp`). Both need Moderate or harder.
6. **Per-mode behaviors**.
   Flag defense, Chicken and Tag evasion, Jail rescue, and Race checkpoints get explicit behaviors, not only target filters.
   First cut: a flag carrier ignores stomps and nearby players and runs home, and other CPUs count a flag carrier's distance at half.
   Thwomps and podobos count their vertical distance at a quarter, so one anywhere in the CPU's column is near, and the CPU runs sideways out of the column instead of ducking or jumping.
   A fleeing CPU (the chicken, a player near the tagged one, any CPU facing an invincible opponent) steps to the neighbouring tile farthest from the threat, and when cornered jumps over it.
   Focusing untagged players on the tagged one in Tag left them pinned in corners, so Tag keeps the nearest-player target.
   In Star mode the holder of the star worth keeping counts at half distance and the holder of the star to pass on is avoided; before, a CPU without the star that picked a player to attack stood still.
7. **Target spreading**.
   A target that another CPU already chases scores lower.
   The current leader scores higher.
   Done as a distance weight: each other CPU chasing an opponent adds its distance once more, and the score leader counts at three quarters.
8. **Navigation graph and A\*.**
   Build a graph from the tile map: walkable tiles, jump and fall links, wrap edges, and warp pipes.
   Steer along the path, not straight at the target.
   Done in `ai_nav.rs`: the tiles a player can stand on, linked by walks, falls, drops through solid-on-top ledges, jumps of up to three tiles up and four across, screen wraps and pipes, searched cheapest-first on each think.
   Goals always follow the path. Chasing a player, or a goal a player carries, the AI follows it only once the straight line is blocked or the target is three rows away, because the straight line wins fights.
   Moving platforms count as tiles where they are when the graph is built, and the graph is rebuilt every eight thinks on maps that have them.
   The graph has no springs or breakable blocks, so an unreachable route keeps the straight-line steering, and step 3's wall jump stays as a fallback.
9. **Skill-based difficulty and personalities**.
   Difficulty sets planning depth, jump precision, and objective focus.
   Personalities (aggressive, objective, cautious) are weight presets for step 4.
   First cut: each CPU draws a style at match start from the game RNG (balanced, aggressive, objective or cautious), which sets the distances that pull it off its goal, the threat distance it runs from, and how close an opponent above it must be before it backs away (`style_ranges`). Very Easy CPUs steer in straight lines, Easy ones plan routes only to goals, and the lead jump and stomp dodge need Moderate or harder.

All randomness stays on the game RNG, and no logic depends on wall-clock time or hash map iteration order.

## Rollout

Ship each step as its own PR, after the handles refactor lands.
Steps 1 and 2 go first: the benchmark gives every later PR a baseline, and the wrap fix is small.
Each PR posts the benchmark before and after, and refreshes the replay goldens that include CPU players.

## Alternatives considered

- **Tune the existing ladder thresholds**.
  This is cheap, but a fixed order cannot express "the flag carrier goes home" next to "the chaser hunts the carrier".
- **Machine-learned policy**.
  This needs a training pipeline, makes behavior hard to debug, and puts float determinism across platforms at risk.
- **Behavior trees per mode**.
  These are workable, but every mode then needs a full tree.
  Utility scoring shares one engine and varies by weights.

## Risks and open questions

- The handles refactor rewrites most of the `Ptr` access in `ai.rs`.
  Starting before it lands means large conflicts, so this work waits for it.
- The navigation graph treats moving platforms as still for eight thinks at a time. Riding a platform along its path, springs and breakable blocks are open.
- The benchmark needs a stuck metric that does not flag deliberate waiting, such as a CPU guarding a flag. It already stops at game over, so the winner idling on the results screen does not count.
