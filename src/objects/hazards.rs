use bevy::prelude::*;

use crate::audio::sfx::{play_sfx, SfxId, SfxLibrary, SfxVolume};
use crate::core::constants::*;
use crate::objects::components::*;
use crate::physics::collision::aabb_overlap;
use crate::physics::components::{Collider, CollisionLayer};
use crate::player::components::{
    DeathCause, Invincible, Player, PlayerAction, PlayerDeathEvent, SpawnProtection, Velocity,
};

// ---------------------------------------------------------------------------
// Hazard components
// ---------------------------------------------------------------------------

/// Types of environmental hazards.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardType {
    Thwomp,
    FireBar,
    PiranhaPlant,
    BulletCannon,
    Fireball,
    Spikes,
}

/// Hazard behavior data.
#[derive(Component, Debug)]
pub struct Hazard {
    pub hazard_type: HazardType,
    pub active: bool,
    pub cycle_timer: f32,
    pub cycle_duration: f32,
}

impl Hazard {
    pub fn thwomp() -> Self {
        Self {
            hazard_type: HazardType::Thwomp,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 120.0,
        }
    }

    pub fn bullet_cannon() -> Self {
        Self {
            hazard_type: HazardType::BulletCannon,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 200.0,
        }
    }

    pub fn piranha_plant() -> Self {
        Self {
            hazard_type: HazardType::PiranhaPlant,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 180.0,
        }
    }

    pub fn fire_bar() -> Self {
        Self {
            hazard_type: HazardType::FireBar,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 360.0,
        }
    }

    pub fn spikes() -> Self {
        Self {
            hazard_type: HazardType::Spikes,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 0.0, // Static hazard
        }
    }
}

/// Thwomp-specific state machine.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ThwompPhase {
    #[default]
    Waiting,
    Slamming,
    OnGround,
    Rising,
}

/// Thwomp behavior data.
#[derive(Component, Debug)]
pub struct ThwompState {
    pub phase: ThwompPhase,
    pub home_y: f32,
    pub target_y: f32,
    pub slam_speed: f32,
    pub rise_speed: f32,
    pub ground_wait: f32,
    pub ground_timer: f32,
    pub detection_width: f32,
}

impl ThwompState {
    pub fn new(home_y: f32) -> Self {
        Self {
            phase: ThwompPhase::Waiting,
            home_y,
            target_y: -(SCREEN_HEIGHT / 2.0 - TILE_SIZE * 2.0),
            slam_speed: 12.0,
            rise_speed: 2.0,
            ground_wait: 1.0,
            ground_timer: 0.0,
            detection_width: TILE_SIZE * 3.0,
        }
    }
}

/// Piranha plant state machine.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PiranhaPhase {
    #[default]
    Hidden,
    Emerging,
    Visible,
    Retracting,
}

/// Piranha plant behavior data.
#[derive(Component, Debug)]
pub struct PiranhaState {
    pub phase: PiranhaPhase,
    pub base_y: f32,
    pub emerge_height: f32,
    pub visible_duration: f32,
    pub hidden_duration: f32,
    pub phase_timer: f32,
    pub move_speed: f32,
}

impl PiranhaState {
    pub fn new(base_y: f32) -> Self {
        Self {
            phase: PiranhaPhase::Hidden,
            base_y,
            emerge_height: TILE_SIZE * 1.5,
            visible_duration: 2.0,
            hidden_duration: 3.0,
            phase_timer: 0.0,
            move_speed: 2.0,
        }
    }
}

/// Bullet cannon firing timer.
#[derive(Component, Debug)]
pub struct CannonState {
    pub fire_timer: Timer,
    pub facing_left: bool,
    pub bullet_speed: f32,
}

impl CannonState {
    pub fn new(facing_left: bool) -> Self {
        Self {
            fire_timer: Timer::from_seconds(3.0, TimerMode::Repeating),
            facing_left,
            bullet_speed: 4.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Hazard spawning
// ---------------------------------------------------------------------------

/// Spawns a hazard entity at the given world position.
pub fn spawn_hazard(commands: &mut Commands, hazard: Hazard, x: f32, y: f32) -> Entity {
    let hazard_type = hazard.hazard_type;

    let mut entity = commands.spawn((
        GameObject {
            object_type: ObjectType::Hazard,
            alive: true,
        },
        hazard,
        Collider::new(TILE_SIZE, TILE_SIZE),
        CollisionLayer::Hazard,
        DamageOnContact {
            kill: true,
            owner_player: None,
        },
        Transform::from_xyz(x, y, crate::graphics::sprites::z_order::OBJECTS + 1.0),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    match hazard_type {
        HazardType::Thwomp => {
            entity.insert(ThwompState::new(y));
            entity.insert(Collider::new(TILE_SIZE * 2.0, TILE_SIZE * 2.0));
        }
        HazardType::PiranhaPlant => {
            entity.insert(PiranhaState::new(y));
        }
        HazardType::BulletCannon => {
            entity.insert(CannonState::new(true));
        }
        _ => {}
    }

    entity.id()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Thwomp behavior: waits at ceiling, slams down when a player is below, then
/// slowly rises back to its home position.
pub fn thwomp_system(
    time: Res<Time>,
    mut query: Query<(&mut ThwompState, &mut Transform, &Collider)>,
    player_query: Query<(&Transform, &PlayerAction), (With<Player>, Without<ThwompState>)>,
) {
    let dt = time.delta_secs();

    for (mut thwomp, mut transform, _collider) in query.iter_mut() {
        match thwomp.phase {
            ThwompPhase::Waiting => {
                // Check if any player is below the thwomp
                let thwomp_x = transform.translation.x;
                let half_detect = thwomp.detection_width * 0.5;

                for (p_transform, p_action) in player_query.iter() {
                    if *p_action == PlayerAction::Dead {
                        continue;
                    }

                    let px = p_transform.translation.x;
                    if px >= thwomp_x - half_detect
                        && px <= thwomp_x + half_detect
                        && p_transform.translation.y < transform.translation.y
                    {
                        thwomp.phase = ThwompPhase::Slamming;
                        break;
                    }
                }
            }
            ThwompPhase::Slamming => {
                // Move downward rapidly (negative Y in Bevy)
                transform.translation.y -= thwomp.slam_speed;

                if transform.translation.y <= thwomp.target_y {
                    transform.translation.y = thwomp.target_y;
                    thwomp.phase = ThwompPhase::OnGround;
                    thwomp.ground_timer = 0.0;
                }
            }
            ThwompPhase::OnGround => {
                thwomp.ground_timer += dt;
                if thwomp.ground_timer >= thwomp.ground_wait {
                    thwomp.phase = ThwompPhase::Rising;
                }
            }
            ThwompPhase::Rising => {
                // Move upward (positive Y in Bevy)
                transform.translation.y += thwomp.rise_speed;

                if transform.translation.y >= thwomp.home_y {
                    transform.translation.y = thwomp.home_y;
                    thwomp.phase = ThwompPhase::Waiting;
                }
            }
        }
    }
}

/// Bullet cannon: fires bullet bill enemies at regular intervals.
pub fn bullet_cannon_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    sfx_vol: Res<SfxVolume>,
    time: Res<Time>,
    mut query: Query<(&mut CannonState, &Transform)>,
) {
    for (mut cannon, transform) in query.iter_mut() {
        if !cannon.fire_timer.tick(time.delta()).just_finished() {
            continue;
        }

        play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Fireball);

        let dir = if cannon.facing_left { -1.0 } else { 1.0 };
        let spawn_x = transform.translation.x + dir * TILE_SIZE;
        let spawn_y = transform.translation.y;

        // Spawn a bullet bill as an enemy entity
        let bullet = crate::objects::enemies::Enemy {
            enemy_type: crate::objects::enemies::EnemyType::BulletBill,
            state: crate::objects::enemies::EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: cannon.bullet_speed,
        };

        let entity = crate::objects::enemies::spawn_enemy(
            &mut commands,
            bullet,
            spawn_x,
            spawn_y,
        );

        // Override facing based on cannon direction
        let facing = if cannon.facing_left {
            crate::objects::enemies::EnemyFacing::Left
        } else {
            crate::objects::enemies::EnemyFacing::Right
        };
        commands.entity(entity).insert(facing);

        // Bullet bills fly straight -- remove gravity
        commands
            .entity(entity)
            .remove::<crate::physics::components::GravityAffected>();
    }
}

/// Piranha plant: emerges from pipe, stays visible briefly, then retracts.
pub fn piranha_plant_system(
    time: Res<Time>,
    mut query: Query<(&mut PiranhaState, &mut Transform, &mut Collider)>,
) {
    let dt = time.delta_secs();

    for (mut piranha, mut transform, mut collider) in query.iter_mut() {
        piranha.phase_timer += dt;

        match piranha.phase {
            PiranhaPhase::Hidden => {
                // Stay hidden at base position
                transform.translation.y = piranha.base_y;
                collider.height = 0.0; // No collision while hidden

                if piranha.phase_timer >= piranha.hidden_duration {
                    piranha.phase = PiranhaPhase::Emerging;
                    piranha.phase_timer = 0.0;
                }
            }
            PiranhaPhase::Emerging => {
                // Rise upward from pipe (positive Y in Bevy)
                transform.translation.y += piranha.move_speed;
                let emerged = transform.translation.y - piranha.base_y;
                collider.height = emerged.max(1.0);

                if emerged >= piranha.emerge_height {
                    transform.translation.y = piranha.base_y + piranha.emerge_height;
                    collider.height = piranha.emerge_height;
                    piranha.phase = PiranhaPhase::Visible;
                    piranha.phase_timer = 0.0;
                }
            }
            PiranhaPhase::Visible => {
                // Stay visible and dangerous
                if piranha.phase_timer >= piranha.visible_duration {
                    piranha.phase = PiranhaPhase::Retracting;
                    piranha.phase_timer = 0.0;
                }
            }
            PiranhaPhase::Retracting => {
                // Sink back into pipe (negative Y in Bevy)
                transform.translation.y -= piranha.move_speed;
                let emerged = transform.translation.y - piranha.base_y;
                collider.height = emerged.max(0.0);

                if transform.translation.y <= piranha.base_y {
                    transform.translation.y = piranha.base_y;
                    collider.height = 0.0;
                    piranha.phase = PiranhaPhase::Hidden;
                    piranha.phase_timer = 0.0;
                }
            }
        }
    }
}

/// Kills players on contact with any active hazard.
pub fn hazard_collision_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    sfx_vol: Res<SfxVolume>,
    hazard_query: Query<(&Hazard, &Collider, &Transform, &GameObject)>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut PlayerAction,
        &mut Velocity,
        &Transform,
        &Collider,
        Option<&Invincible>,
        Option<&SpawnProtection>,
    ), With<Player>>,
    mut contact_events: EventWriter<HazardContactEvent>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
    for (hazard, h_collider, h_transform, game_obj) in hazard_query.iter() {
        if !game_obj.alive || !hazard.active {
            continue;
        }

        // Skip hazards with zero-height colliders (hidden piranha plants)
        if h_collider.height <= 0.0 {
            continue;
        }

        for (p_entity, player, mut p_action, _p_vel, p_transform, p_collider, invincible, spawn_prot) in
            player_query.iter_mut()
        {
            if *p_action == PlayerAction::Dead || *p_action == PlayerAction::Spawning {
                continue;
            }

            if !aabb_overlap(
                &h_transform.translation,
                h_collider,
                &p_transform.translation,
                p_collider,
            ) {
                continue;
            }

            contact_events.send(HazardContactEvent {
                hazard_entity: Entity::PLACEHOLDER,
                player_entity: p_entity,
            });

            if invincible.is_some() || spawn_prot.is_some() {
                // Invincible or spawn-protected players survive hazard contact
                continue;
            }

            play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PlayerDeath);
            *p_action = PlayerAction::Dead;
            death_events.send(PlayerDeathEvent {
                player_entity: p_entity,
                player_index: player.index,
                killer_index: None,
                cause: DeathCause::Environment,
            });
        }
    }
}

/// Updates hazard cycle timers for hazards that use frame-based cycling
/// (fire bars, rotating fireballs, etc.).
pub fn hazard_cycle_system(
    time: Res<Time>,
    mut query: Query<&mut Hazard>,
) {
    let dt = time.delta_secs();

    for mut hazard in query.iter_mut() {
        if hazard.cycle_duration <= 0.0 {
            continue;
        }

        hazard.cycle_timer += dt * 60.0; // Frame-based timer
        if hazard.cycle_timer >= hazard.cycle_duration {
            hazard.cycle_timer -= hazard.cycle_duration;
        }
    }
}
