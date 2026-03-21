use bevy::prelude::*;

use crate::player::components::{
    CarriedObject, CapeState, Facing, Player, PlayerAction, PlayerSkinIndex,
    PowerupState, ShieldState, SpawnProtection, TailState,
};
use crate::graphics::animation::{
    AnimationController, AnimationBank, AnimationState, AnimatedEntityType, BlinkEffect,
    DeathAnimation,
};
use crate::graphics::sprites::SpriteSheets;

// ---------------------------------------------------------------------------
// Player sprite marker
// ---------------------------------------------------------------------------

/// Marker component indicating this entity has been set up with player sprite visuals.
#[derive(Component, Debug)]
pub struct PlayerSpriteSetup;

// ---------------------------------------------------------------------------
// Visual overlay components
// ---------------------------------------------------------------------------

/// Marker for the shield overlay child entity.
#[derive(Component, Debug)]
pub struct ShieldOverlay;

/// Marker for the cape/tail attachment child entity.
#[derive(Component, Debug)]
pub struct AttackAttachment;

/// Marker for carried object visual child entity.
#[derive(Component, Debug)]
pub struct CarriedObjectVisual;

// ---------------------------------------------------------------------------
// Powerup visual tint timer
// ---------------------------------------------------------------------------

/// Component that cycles colors for star power visual effect.
#[derive(Component, Debug)]
pub struct StarPowerVisual {
    pub elapsed: f32,
    pub cycle_speed: f32,
}

impl Default for StarPowerVisual {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            cycle_speed: 8.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Team colors
// ---------------------------------------------------------------------------

/// Returns a tint color for a given team index.
pub fn team_color(team: u8) -> Color {
    match team {
        0 => Color::srgb(1.0, 0.4, 0.4),   // Red
        1 => Color::srgb(0.4, 0.6, 1.0),   // Blue
        2 => Color::srgb(0.4, 1.0, 0.4),   // Green
        3 => Color::srgb(1.0, 1.0, 0.4),   // Yellow
        _ => Color::WHITE,
    }
}

/// Rainbow cycle colors for star power.
fn star_power_color(elapsed: f32) -> Color {
    let r = (elapsed * 3.0).sin() * 0.5 + 0.5;
    let g = (elapsed * 3.0 + 2.094).sin() * 0.5 + 0.5; // offset by 2pi/3
    let b = (elapsed * 3.0 + 4.189).sin() * 0.5 + 0.5; // offset by 4pi/3
    Color::srgb(r, g, b)
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Attaches sprite visuals to newly spawned Player entities that have a
/// PlayerSkinIndex but no PlayerSpriteSetup marker yet.
pub fn setup_player_sprites(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    query: Query<
        (Entity, &Player, &PlayerSkinIndex),
        (Without<PlayerSpriteSetup>, With<Player>),
    >,
) {
    for (entity, player, skin_index) in query.iter() {
        let skin_idx = skin_index.0.min(sprite_sheets.player_skins.len().saturating_sub(1));

        if sprite_sheets.player_skins.is_empty() {
            // Assets not loaded yet; skip
            continue;
        }

        let skin = &sprite_sheets.player_skins[skin_idx];

        // Insert Sprite with texture atlas on the player entity
        commands.entity(entity).insert((
            Sprite {
                image: skin.texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: skin.atlas_layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            AnimationController::new(vec![0], 1.0, false),
            PlayerSpriteSetup,
        ));

        info!(
            "Set up player {} sprite with skin '{}'",
            player.index, skin.name
        );
    }
}

/// Syncs the player sprite frame with the current PlayerAction.
/// Frame mapping for 6-frame skin strips:
///   Standing: frame 0
///   Running: frames 0-3 looping
///   Jumping: frame 4
///   Falling: frame 4
///   Skidding: frame 3
///   Dead: frame 5 (spinning via DeathAnimation)
///   Spawning: frame 0 (with blink effect)
pub fn sync_player_sprite_action(
    animation_bank: Res<AnimationBank>,
    mut query: Query<
        (&PlayerAction, &mut AnimationController),
        (With<Player>, With<PlayerSpriteSetup>, Changed<PlayerAction>),
    >,
) {
    for (action, mut anim_ctrl) in query.iter_mut() {
        let anim_state = AnimationState::from_player_action(action);

        if let Some(frame_data) =
            animation_bank.get(AnimatedEntityType::Player, anim_state)
        {
            anim_ctrl.set_animation(anim_state, frame_data);
        }
    }
}

/// Applies facing direction as sprite flip.
pub fn sync_player_facing(
    mut query: Query<
        (&Facing, &mut Sprite),
        (With<Player>, With<PlayerSpriteSetup>, Changed<Facing>),
    >,
) {
    for (facing, mut sprite) in query.iter_mut() {
        sprite.flip_x = *facing == Facing::Left;
    }
}

/// Applies team color tint to player sprites.
pub fn apply_team_tint(
    mut query: Query<
        (&Player, &PowerupState, &mut Sprite, Option<&StarPowerVisual>),
        With<PlayerSpriteSetup>,
    >,
) {
    for (player, powerup, mut sprite, star_visual) in query.iter_mut() {
        match powerup {
            PowerupState::Star => {
                // Star power uses rainbow cycling (handled by star_power_visual_system)
                // Don't override here if StarPowerVisual is present
                if star_visual.is_none() {
                    sprite.color = Color::WHITE;
                }
            }
            PowerupState::Fire => {
                // Fire powerup: warm red-orange tint
                sprite.color = Color::srgb(1.0, 0.7, 0.5);
            }
            _ => {
                // Normal: apply team color
                sprite.color = team_color(player.team);
            }
        }
    }
}

/// Cycles rainbow colors for star power visual effect.
pub fn star_power_visual_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &PowerupState,
        Option<&mut StarPowerVisual>,
        &mut Sprite,
    ), With<PlayerSpriteSetup>>,
) {
    let dt = time.delta_secs();
    for (entity, powerup, star_visual, mut sprite) in query.iter_mut() {
        match powerup {
            PowerupState::Star => {
                if let Some(mut star) = star_visual {
                    star.elapsed += dt;
                    sprite.color = star_power_color(star.elapsed * star.cycle_speed);
                } else {
                    commands.entity(entity).insert(StarPowerVisual::default());
                }
            }
            _ => {
                if star_visual.is_some() {
                    commands.entity(entity).remove::<StarPowerVisual>();
                }
            }
        }
    }
}

/// Adds/removes BlinkEffect based on SpawnProtection presence.
pub fn manage_spawn_blink(
    mut commands: Commands,
    query: Query<
        (Entity, &PlayerAction, Option<&SpawnProtection>, Option<&BlinkEffect>),
        With<PlayerSpriteSetup>,
    >,
) {
    for (entity, action, spawn_prot, blink) in query.iter() {
        if *action == PlayerAction::Spawning && spawn_prot.is_some() && blink.is_none() {
            commands.entity(entity).insert(BlinkEffect::default());
        } else if *action != PlayerAction::Spawning && blink.is_some() {
            commands.entity(entity).remove::<BlinkEffect>();
        }
    }
}

/// Adds/removes DeathAnimation based on PlayerAction::Dead.
pub fn manage_death_animation(
    mut commands: Commands,
    query: Query<
        (Entity, &PlayerAction, Option<&DeathAnimation>),
        (With<PlayerSpriteSetup>, Changed<PlayerAction>),
    >,
) {
    for (entity, action, death_anim) in query.iter() {
        if *action == PlayerAction::Dead && death_anim.is_none() {
            commands.entity(entity).insert(DeathAnimation::default());
        }
        // DeathAnimation removal is handled by death_animation_system when action changes
    }
}

/// Updates shield overlay visibility and appearance based on ShieldState.
pub fn shield_visual_system(
    query: Query<(&ShieldState, &Children), (With<Player>, With<PlayerSpriteSetup>)>,
    mut overlay_query: Query<(&mut Sprite, &mut Visibility), With<ShieldOverlay>>,
) {
    for (shield_state, children) in query.iter() {
        for child in children.iter() {
            if let Ok((mut sprite, mut visibility)) = overlay_query.get_mut(*child) {
                match shield_state {
                    ShieldState::None => {
                        *visibility = Visibility::Hidden;
                    }
                    ShieldState::Soft => {
                        *visibility = Visibility::Inherited;
                        sprite.color = Color::srgba(0.5, 0.8, 1.0, 0.3);
                    }
                    ShieldState::Hard => {
                        *visibility = Visibility::Inherited;
                        sprite.color = Color::srgba(0.8, 0.8, 0.2, 0.4);
                    }
                }
            }
        }
    }
}

/// Updates cape/tail attack attachment visibility.
pub fn attack_attachment_system(
    query: Query<
        (&Facing, Option<&CapeState>, Option<&TailState>, &Children),
        (With<Player>, With<PlayerSpriteSetup>),
    >,
    mut attachment_query: Query<
        (&mut Transform, &mut Visibility),
        With<AttackAttachment>,
    >,
) {
    for (facing, cape, tail, children) in query.iter() {
        let active = cape.map_or(false, |c| c.active) || tail.map_or(false, |t| t.active);
        let dir = match facing {
            Facing::Right => 1.0,
            Facing::Left => -1.0,
        };

        for child in children.iter() {
            if let Ok((mut transform, mut visibility)) = attachment_query.get_mut(*child) {
                if active {
                    *visibility = Visibility::Inherited;
                    transform.translation.x = dir * 16.0;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

/// Updates carried object visual position relative to player.
pub fn carried_object_visual_system(
    query: Query<
        (&Facing, &CarriedObject, &Children),
        (With<Player>, With<PlayerSpriteSetup>),
    >,
    mut visual_query: Query<
        (&mut Transform, &mut Visibility),
        With<CarriedObjectVisual>,
    >,
) {
    for (facing, carried, children) in query.iter() {
        let has_object = carried.entity.is_some();
        let dir = match facing {
            Facing::Right => 1.0,
            Facing::Left => -1.0,
        };

        for child in children.iter() {
            if let Ok((mut transform, mut visibility)) = visual_query.get_mut(*child) {
                if has_object {
                    *visibility = Visibility::Inherited;
                    // Position the visual in front of the player based on facing
                    transform.translation.x = dir * 12.0;
                    transform.translation.y = 4.0;
                    // Z is slightly in front/behind based on facing
                    transform.translation.z = if dir > 0.0 { 0.1 } else { -0.1 };
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

/// Spawns child overlay entities (shield, attack attachment, carried visual)
/// for players that need them.
pub fn spawn_player_overlays(
    mut commands: Commands,
    query: Query<Entity, (With<Player>, With<PlayerSpriteSetup>, Without<ShieldOverlay>)>,
    has_children: Query<&Children>,
) {
    for entity in query.iter() {
        // Check if this entity already has overlay children
        let has_overlays = has_children
            .get(entity)
            .map(|children| !children.is_empty())
            .unwrap_or(false);

        if has_overlays {
            continue;
        }

        // Spawn shield overlay as a child
        let shield_child = commands
            .spawn((
                Sprite {
                    color: Color::srgba(0.5, 0.8, 1.0, 0.0),
                    custom_size: Some(Vec2::new(40.0, 40.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 0.05),
                Visibility::Hidden,
                ShieldOverlay,
            ))
            .id();

        // Spawn attack attachment as a child
        let attack_child = commands
            .spawn((
                Sprite {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                    custom_size: Some(Vec2::new(12.0, 20.0)),
                    ..default()
                },
                Transform::from_xyz(16.0, 0.0, 0.05),
                Visibility::Hidden,
                AttackAttachment,
            ))
            .id();

        // Spawn carried object visual as a child
        let carried_child = commands
            .spawn((
                Sprite {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                    custom_size: Some(Vec2::new(16.0, 16.0)),
                    ..default()
                },
                Transform::from_xyz(12.0, 4.0, 0.1),
                Visibility::Hidden,
                CarriedObjectVisual,
            ))
            .id();

        commands
            .entity(entity)
            .add_children(&[shield_child, attack_child, carried_child]);
    }
}
