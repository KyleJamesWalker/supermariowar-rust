pub mod classic;
pub mod coins;
pub mod ctf;
pub mod domination;
pub mod frenzy;
pub mod king_of_the_hill;
pub mod stomp;
pub mod survival;
pub mod tag;
pub mod timelimit;
pub mod traits;

use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::core::game_state::{GameState, GameplayState, MatchResult};
use crate::player::components::{DeathCause, PlayerDeathEvent, PlayerStompEvent};
use traits::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GameModesPlugin;

impl Plugin for GameModesPlugin {
    fn build(&self, app: &mut App) {
        // --- Resources ---
        app.init_resource::<GameModeConfig>()
            .init_resource::<ScoreBoard>()
            .init_resource::<GameTimer>()
            .init_resource::<RoundManager>()
            .init_resource::<RoundCountdown>()
            .init_resource::<RoundOverTimer>()
            // Per-mode state
            .init_resource::<classic::ClassicModeState>()
            .init_resource::<coins::CoinModeState>()
            .init_resource::<ctf::CtfModeState>()
            .init_resource::<stomp::StompModeState>()
            .init_resource::<timelimit::TimeLimitModeState>()
            .init_resource::<survival::SurvivalModeState>()
            .init_resource::<domination::DominationModeState>()
            .init_resource::<king_of_the_hill::KothModeState>()
            .init_resource::<tag::TagModeState>()
            .init_resource::<frenzy::FrenzyModeState>()
            .init_resource::<PendingRoundReset>();

        // --- Events ---
        // NOTE: PlayerDeathEvent is registered by PlayerPlugin (player/mod.rs).
        app.add_event::<PlayerKillEvent>()
            .add_event::<ItemCollectEvent>()
            .add_event::<ObjectiveEvent>()
            .add_event::<ScoreEvent>()
            .add_event::<RoundOverEvent>()
            .add_event::<CountdownFinishedEvent>();

        // --- Countdown systems (run during Countdown sub-state) ---
        app.add_systems(
            Update,
            countdown_system
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::Countdown)),
        );

        let playing_condition = in_state(GameState::Gameplay)
            .and(in_state(GameplayState::Playing));

        // --- Bridge system: convert death/stomp events into kill events ---
        // Must run BEFORE any game mode handler so that PlayerKillEvent is
        // available in the same frame.
        app.add_systems(
            PreUpdate,
            bridge_death_to_kill_events
                .run_if(playing_condition.clone()),
        );

        // --- Gameplay systems: Group 1 (Classic + Coins) ---
        app.add_systems(
            Update,
            (
                classic::handle_classic_kills,
                classic::handle_classic_score,
                classic::check_classic_win_condition,
                coins::coin_spawn_system,
                coins::handle_coin_collect,
                coins::coins_drop_on_kill,
                coins::handle_coin_score,
                coins::check_coin_win_condition,
            )
                .run_if(playing_condition.clone()),
        );

        // --- Gameplay systems: Group 2 (CTF) ---
        app.add_systems(
            Update,
            (
                ctf::spawn_flags,
                ctf::flag_pickup_system,
                ctf::flag_follow_carrier,
                ctf::flag_capture_system,
                ctf::flag_drop_on_kill,
                ctf::flag_auto_return_system,
                ctf::handle_ctf_score,
                ctf::check_ctf_win_condition,
            )
                .run_if(playing_condition.clone()),
        );

        // --- Gameplay systems: Group 3 (Stomp + TimeLimit + Survival) ---
        app.add_systems(
            Update,
            (
                stomp::handle_stomp_kills,
                stomp::reset_stomp_combo_on_death,
                stomp::check_stomp_win_condition,
                timelimit::timelimit_start_timer,
                timelimit::timelimit_tick,
                timelimit::handle_timelimit_kills,
                survival::survival_init,
                survival::handle_survival_kills,
                survival::handle_survival_death,
            )
                .run_if(playing_condition.clone()),
        );

        // --- Gameplay systems: Group 4 (Domination + KotH) ---
        app.add_systems(
            Update,
            (
                domination::spawn_domination_zones,
                domination::domination_claim_system,
                domination::domination_score_tick,
                domination::check_domination_win_condition,
                king_of_the_hill::spawn_hill,
                king_of_the_hill::hill_occupancy_system,
                king_of_the_hill::hill_score_tick,
                king_of_the_hill::hill_relocation_system,
                king_of_the_hill::check_koth_win_condition,
            )
                .run_if(playing_condition.clone()),
        );

        // --- Gameplay systems: Group 5 (Tag + Frenzy + round-over handler) ---
        app.add_systems(
            Update,
            (
                tag::tag_init,
                tag::tag_score_tick,
                tag::tag_transfer_system,
                tag::tag_transfer_on_kill,
                tag::check_tag_win_condition,
                frenzy::frenzy_spawn_powerups,
                frenzy::handle_frenzy_kills,
                frenzy::frenzy_chaos_check,
                frenzy::check_frenzy_win_condition,
                handle_round_over,
            )
                .run_if(playing_condition),
        );

        // --- RoundOver systems ---
        app.add_systems(
            Update,
            (
                round_over_timer_system,
                reset_mode_state_on_new_round,
                cleanup_mode_entities_on_new_round,
            )
                .chain()
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::RoundOver)),
        );

        // --- HUD system (runs any time we are in Gameplay) ---
        app.add_systems(
            Update,
            hud_display_system
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

// ---------------------------------------------------------------------------
// Bridge: convert PlayerDeathEvent + PlayerStompEvent -> PlayerKillEvent
// ---------------------------------------------------------------------------

/// Reads `PlayerDeathEvent` and `PlayerStompEvent` each frame and emits
/// corresponding `PlayerKillEvent`s so that all game modes can react uniformly.
fn bridge_death_to_kill_events(
    mut death_events: EventReader<PlayerDeathEvent>,
    mut stomp_events: EventReader<PlayerStompEvent>,
    mut kill_events: EventWriter<PlayerKillEvent>,
) {
    // Collect stomp events first so we can check if a kill was a stomp.
    // Each stomp event records (stomper_index, stomped_index).
    let stomps: Vec<(usize, usize)> = stomp_events
        .read()
        .map(|e| (e.stomper_index, e.stomped_index))
        .collect();

    for event in death_events.read() {
        let Some(killer) = event.killer_index else {
            // No killer -- environment death, no PlayerKillEvent needed
            continue;
        };

        // Determine the kill method by checking if a matching stomp event exists
        let is_stomp = stomps
            .iter()
            .any(|&(stomper, stomped)| stomper == killer && stomped == event.player_index);

        let method = if is_stomp {
            KillMethod::Stomp
        } else {
            match event.cause {
                DeathCause::Environment => KillMethod::Environment,
                _ => KillMethod::Other,
            }
        };

        kill_events.send(PlayerKillEvent {
            killer,
            victim: event.player_index,
            method,
        });
    }
}

// ---------------------------------------------------------------------------
// Countdown system (3-2-1-GO!)
// ---------------------------------------------------------------------------

fn countdown_system(
    time: Res<Time>,
    mut countdown: ResMut<RoundCountdown>,
    mut next_gameplay_state: ResMut<NextState<GameplayState>>,
    mut countdown_finished: EventWriter<CountdownFinishedEvent>,
) {
    countdown.timer.tick(time.delta());

    if countdown.timer.just_finished() {
        match countdown.phase {
            CountdownPhase::Three => {
                countdown.phase = CountdownPhase::Two;
                countdown.timer = Timer::from_seconds(1.0, TimerMode::Once);
            }
            CountdownPhase::Two => {
                countdown.phase = CountdownPhase::One;
                countdown.timer = Timer::from_seconds(1.0, TimerMode::Once);
            }
            CountdownPhase::One => {
                countdown.phase = CountdownPhase::Go;
                countdown.timer = Timer::from_seconds(0.5, TimerMode::Once);
            }
            CountdownPhase::Go => {
                countdown.phase = CountdownPhase::Done;
                countdown_finished.send(CountdownFinishedEvent);
                next_gameplay_state.set(GameplayState::Playing);
            }
            CountdownPhase::Done => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Round-over handling
// ---------------------------------------------------------------------------

/// Listens for RoundOverEvent and transitions the gameplay state to RoundOver.
/// Records the winning player/team in MatchResult and updates the RoundManager.
fn handle_round_over(
    mut round_over_events: EventReader<RoundOverEvent>,
    mut next_gameplay_state: ResMut<NextState<GameplayState>>,
    mut match_result: ResMut<MatchResult>,
    mut round_manager: ResMut<RoundManager>,
    mut round_over_timer: ResMut<RoundOverTimer>,
) {
    for event in round_over_events.read() {
        // Store result
        match_result.winning_player = event.winning_player;
        match_result.winning_team = event.winning_team;

        // Update round wins
        if let Some(player) = event.winning_player {
            if player < MAX_PLAYERS {
                round_manager.round_wins[player] += 1;
            }
        }
        if let Some(team) = event.winning_team {
            let team_idx = team as usize;
            if team_idx < MAX_PLAYERS {
                round_manager.team_round_wins[team_idx] += 1;
            }
        }

        // Reset the round-over display timer
        round_over_timer.timer.reset();

        // Transition to RoundOver
        next_gameplay_state.set(GameplayState::RoundOver);

        // Only process the first event per frame
        return;
    }
}

/// Resource flag set by `round_over_timer_system` (or pause-menu restart) to
/// signal that mode state should be reset. This avoids exceeding Bevy's
/// 16-parameter system limit.
#[derive(Resource, Debug, Default)]
pub struct PendingRoundReset(pub bool);

/// During RoundOver state, wait for the display timer to expire, then either
/// start the next round or end the match.
fn round_over_timer_system(
    time: Res<Time>,
    mut round_over_timer: ResMut<RoundOverTimer>,
    mut round_manager: ResMut<RoundManager>,
    mut next_gameplay_state: ResMut<NextState<GameplayState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut countdown: ResMut<RoundCountdown>,
    mut pending_reset: ResMut<PendingRoundReset>,
) {
    round_over_timer.timer.tick(time.delta());

    if !round_over_timer.timer.just_finished() {
        return;
    }

    // Advance to next round first, then check if match is over
    round_manager.current_round += 1;

    if round_manager.current_round > round_manager.total_rounds {
        // Match is over
        round_manager.match_finished = true;
        next_gameplay_state.set(GameplayState::MatchOver);
        next_game_state.set(GameState::GameOver);
    } else {
        // Signal reset
        pending_reset.0 = true;

        // Reset countdown
        *countdown = RoundCountdown::default();

        // Go back to countdown phase for the next round
        next_gameplay_state.set(GameplayState::Countdown);
    }
}

/// Resets all per-mode state when transitioning to a new round.
fn reset_mode_state_on_new_round(
    mut pending_reset: ResMut<PendingRoundReset>,
    mut classic_state: ResMut<classic::ClassicModeState>,
    mut coin_state: ResMut<coins::CoinModeState>,
    mut ctf_state: ResMut<ctf::CtfModeState>,
    mut stomp_state: ResMut<stomp::StompModeState>,
    mut timelimit_state: ResMut<timelimit::TimeLimitModeState>,
    mut survival_state: ResMut<survival::SurvivalModeState>,
    mut domination_state: ResMut<domination::DominationModeState>,
    mut koth_state: ResMut<king_of_the_hill::KothModeState>,
    mut tag_state: ResMut<tag::TagModeState>,
    mut frenzy_state: ResMut<frenzy::FrenzyModeState>,
    mut game_timer: ResMut<GameTimer>,
    mut scoreboard: ResMut<ScoreBoard>,
) {
    if !pending_reset.0 {
        return;
    }
    pending_reset.0 = false;

    *classic_state = classic::ClassicModeState::default();
    *coin_state = coins::CoinModeState::default();
    *ctf_state = ctf::CtfModeState::default();
    *stomp_state = stomp::StompModeState::default();
    *timelimit_state = timelimit::TimeLimitModeState::default();
    *survival_state = survival::SurvivalModeState::default();
    *domination_state = domination::DominationModeState::default();
    *koth_state = king_of_the_hill::KothModeState::default();
    *tag_state = tag::TagModeState::default();
    *frenzy_state = frenzy::FrenzyModeState::default();
    *game_timer = GameTimer::default();
    *scoreboard = ScoreBoard::default();
}

/// Despawns game-mode entities (flags, zones, coins) between rounds so they
/// can be re-spawned fresh. Runs after `reset_mode_state_on_new_round` which
/// resets the `*_spawned` flags that guard the spawn systems.
fn cleanup_mode_entities_on_new_round(
    mut commands: Commands,
    pending_reset: Res<PendingRoundReset>,
    flag_query: Query<Entity, With<ctf::Flag>>,
    indicator_query: Query<Entity, With<ctf::FlagCarrierIndicator>>,
    zone_query: Query<Entity, With<domination::DominationZone>>,
    hill_query: Query<Entity, With<king_of_the_hill::HillZone>>,
    coin_query: Query<Entity, With<coins::CoinEntity>>,
    tagged_query: Query<Entity, With<tag::TaggedIt>>,
) {
    // Only run when reset_mode_state_on_new_round just consumed the flag
    // (changed it from true to false) in the same frame.
    if !pending_reset.is_changed() || pending_reset.0 {
        return;
    }

    for entity in flag_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in indicator_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in zone_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in hill_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in coin_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in tagged_query.iter() {
        commands.entity(entity).remove::<tag::TaggedIt>();
    }
}

// ---------------------------------------------------------------------------
// HUD display system (placeholder -- logs state to console for now)
// ---------------------------------------------------------------------------

/// Displays mode-relevant information. In a full game this would render UI
/// elements; here it writes to the Bevy log when values change.
fn hud_display_system(
    config: Res<GameModeConfig>,
    scoreboard: Res<ScoreBoard>,
    game_timer: Res<GameTimer>,
    round_manager: Res<RoundManager>,
    gameplay_state: Res<State<GameplayState>>,
    countdown: Res<RoundCountdown>,
) {
    // Only log on resource changes to avoid spam
    if !scoreboard.is_changed()
        && !game_timer.is_changed()
        && !round_manager.is_changed()
        && !countdown.is_changed()
    {
        return;
    }

    // During countdown, show the countdown phase
    if *gameplay_state.get() == GameplayState::Countdown {
        match countdown.phase {
            CountdownPhase::Three => info!("[HUD] 3..."),
            CountdownPhase::Two => info!("[HUD] 2..."),
            CountdownPhase::One => info!("[HUD] 1..."),
            CountdownPhase::Go => info!("[HUD] GO!"),
            CountdownPhase::Done => {}
        }
        return;
    }

    // Show round info
    if round_manager.is_changed() {
        info!(
            "[HUD] Round {}/{}",
            round_manager.current_round, round_manager.total_rounds
        );
    }

    // For time-based modes, show remaining time
    if config.mode_type == GameModeType::TimeLimit && game_timer.is_changed() {
        let remaining = game_timer.remaining_secs();
        let mins = (remaining / 60.0) as u32;
        let secs = (remaining % 60.0) as u32;
        info!("[HUD] Time: {:02}:{:02}", mins, secs);
        if game_timer.expired {
            info!("[HUD] SUDDEN DEATH!");
        }
    }

    // Show scores
    if scoreboard.is_changed() {
        for i in 0..MAX_PLAYERS {
            if scoreboard.score[i] != 0 || scoreboard.kills[i] != 0 {
                info!(
                    "[HUD] P{}: score={} kills={} deaths={}",
                    i + 1,
                    scoreboard.score[i],
                    scoreboard.kills[i],
                    scoreboard.deaths[i],
                );
            }
        }
    }
}
