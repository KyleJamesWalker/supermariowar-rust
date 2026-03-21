use bevy::prelude::*;

use crate::graphics::sprites::SpriteSheets;
use crate::objects::blocks::BlockType;
use crate::objects::enemies::{Enemy, EnemyFacing, EnemyType};
use crate::objects::hazards::{Hazard, HazardType};
use crate::objects::powerups::{PowerupType, SpawnedPowerup};
use crate::objects::projectiles::{Projectile, ProjectileType};

// ---------------------------------------------------------------------------
// Marker component
// ---------------------------------------------------------------------------

/// Marker indicating an object entity has had its sprite visuals attached.
#[derive(Component, Debug)]
pub struct ObjectSpriteSetup;

// ---------------------------------------------------------------------------
// Powerup sprite setup
// ---------------------------------------------------------------------------

/// Maps a PowerupType to the key used in `SpriteSheets::powerup_images`.
fn powerup_image_key(pt: PowerupType) -> Option<&'static str> {
    match pt {
        PowerupType::Fire => Some("fireflower"),
        PowerupType::Hammer => Some("hammerpowerup"),
        PowerupType::Boomerang => Some("boomerangpowerup"),
        PowerupType::Feather => Some("featherpowerup"),
        PowerupType::Leaf => Some("leafpowerup"),
        PowerupType::Bomb => Some("bombpowerup"),
        PowerupType::Star => Some("starpowerup"),
        PowerupType::Poison => Some("poisonpowerup"),
        PowerupType::MysteryMushroom => Some("mysterymushroom"),
        PowerupType::OneUp => Some("1uppowerup"),
        PowerupType::TwoUp => Some("2uppowerup"),
        PowerupType::ThreeUp => Some("3uppowerup"),
        PowerupType::FiveUp => Some("5uppowerup"),
        PowerupType::Clock => Some("clockpowerup"),
        PowerupType::BobOmb => Some("bobombpowerup"),
        PowerupType::BulletBill => Some("bulletbillpowerup"),
        PowerupType::Podoboo => Some("podobopowerup"),
        PowerupType::Tanooki => Some("tanooki"),
        PowerupType::IceWand => Some("icewandpowerup"),
        PowerupType::Jail => Some("jailkeypowerup"),
        PowerupType::ExtraGuy => Some("heartpowerup"),
        PowerupType::SlowDown => Some("clockpowerup"),
        // Mushroom uses the powerup sprite sheet rather than an individual image.
        // Coin and shields have no dedicated individual image file.
        PowerupType::Mushroom | PowerupType::Coin | PowerupType::ShieldSoft | PowerupType::ShieldHard => None,
    }
}

/// Attaches a Sprite to newly-spawned powerup entities that lack one.
pub fn setup_powerup_sprites(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    query: Query<
        (Entity, &SpawnedPowerup),
        (Without<ObjectSpriteSetup>, With<SpawnedPowerup>),
    >,
) {
    for (entity, spawned) in query.iter() {
        // Try individual powerup image first
        if let Some(key) = powerup_image_key(spawned.powerup_type) {
            if let Some(handle) = sprite_sheets.powerup_images.get(key) {
                commands.entity(entity).insert((
                    Sprite {
                        image: handle.clone(),
                        ..default()
                    },
                    ObjectSpriteSetup,
                ));
                continue;
            }
        }

        // Fallback: use the powerup sprite sheet (large.png) with an atlas index.
        if let Some(ref sheet) = sprite_sheets.powerup_sheet {
            let atlas_index = powerup_sheet_index(spawned.powerup_type);
            commands.entity(entity).insert((
                Sprite {
                    image: sheet.texture.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: sheet.atlas_layout.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                ObjectSpriteSetup,
            ));
        }
    }
}

/// Returns the atlas frame index in the large powerup sheet for a given type.
/// The large.png sheet is 28 frames of 32x32. These indices are approximate
/// mappings based on the original SMW powerup ordering.
fn powerup_sheet_index(pt: PowerupType) -> usize {
    match pt {
        PowerupType::Mushroom => 0,
        PowerupType::Fire => 1,
        PowerupType::Hammer => 2,
        PowerupType::Boomerang => 3,
        PowerupType::Feather => 4,
        PowerupType::Leaf => 5,
        PowerupType::Bomb => 6,
        PowerupType::Star => 7,
        PowerupType::Poison => 8,
        PowerupType::MysteryMushroom => 9,
        PowerupType::OneUp => 10,
        PowerupType::TwoUp => 11,
        PowerupType::ThreeUp => 12,
        PowerupType::FiveUp => 13,
        PowerupType::Clock => 14,
        PowerupType::BobOmb => 15,
        PowerupType::BulletBill => 16,
        PowerupType::Podoboo => 17,
        PowerupType::Tanooki => 18,
        PowerupType::ExtraGuy => 19,
        PowerupType::Jail => 20,
        PowerupType::Coin => 21,
        PowerupType::ShieldSoft => 22,
        PowerupType::ShieldHard => 23,
        PowerupType::SlowDown => 24,
        PowerupType::IceWand => 25,
    }
}

// ---------------------------------------------------------------------------
// Enemy sprite setup
// ---------------------------------------------------------------------------

/// Maps an EnemyType to the key used in `SpriteSheets::hazard_images` and
/// returns the default atlas frame index. Enemies in SMW share the hazard
/// sprite sheets.
fn enemy_sprite_info(et: EnemyType) -> (&'static str, usize) {
    match et {
        // Goomba/Koopa variants use the flame sheet as a placeholder since
        // the original SMW stores these in the tileset. We pick a recognizable
        // frame so they are at least visible.
        EnemyType::Goomba | EnemyType::Paragoomba => ("fireball", 0),
        EnemyType::Koopa | EnemyType::Parakoopa => ("fireball", 1),
        EnemyType::KoopaRed => ("fireball", 2),
        EnemyType::Spiny => ("fireball", 3),
        EnemyType::BuzzyBeetle => ("fireball", 4),
        EnemyType::CheepCheep => ("fireball", 5),
        EnemyType::BulletBill => ("bulletbill", 0),
        EnemyType::Podoboo => ("fireball", 6),
    }
}

/// Attaches a Sprite to newly-spawned enemy entities that lack one.
pub fn setup_enemy_sprites(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    query: Query<
        (Entity, &Enemy),
        (Without<ObjectSpriteSetup>, With<Enemy>),
    >,
) {
    for (entity, enemy) in query.iter() {
        let (sheet_key, atlas_index) = enemy_sprite_info(enemy.enemy_type);

        if let Some(sheet) = sprite_sheets.hazard_images.get(sheet_key) {
            commands.entity(entity).insert((
                Sprite {
                    image: sheet.texture.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: sheet.atlas_layout.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                ObjectSpriteSetup,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Projectile sprite setup
// ---------------------------------------------------------------------------

/// Maps a ProjectileType to the key used in `SpriteSheets::projectile_images`
/// and the default atlas frame.
fn projectile_sprite_info(pt: ProjectileType) -> (&'static str, usize) {
    match pt {
        ProjectileType::Fireball => ("fireball", 0),
        ProjectileType::Hammer => ("hammer", 0),
        ProjectileType::Boomerang => ("boomerang", 0),
        ProjectileType::IceBall => ("wandblast", 0),
        ProjectileType::Bomb => ("bomb", 0),
        ProjectileType::Shell => ("shell", 0),
        ProjectileType::ThrowBlock => ("throwblock", 0),
        ProjectileType::BulletBill => ("bulletbill", 0),
        // Feather and TailSwipe don't have dedicated projectile sprites;
        // fall back to fireball as a visible placeholder.
        ProjectileType::Feather | ProjectileType::TailSwipe => ("fireball", 0),
    }
}

/// Attaches a Sprite to newly-spawned projectile entities that lack one.
pub fn setup_projectile_sprites(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    query: Query<
        (Entity, &Projectile),
        (Without<ObjectSpriteSetup>, With<Projectile>),
    >,
) {
    for (entity, projectile) in query.iter() {
        let (sheet_key, atlas_index) = projectile_sprite_info(projectile.projectile_type);

        if let Some(sheet) = sprite_sheets.projectile_images.get(sheet_key) {
            commands.entity(entity).insert((
                Sprite {
                    image: sheet.texture.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: sheet.atlas_layout.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                ObjectSpriteSetup,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Block sprite setup
// ---------------------------------------------------------------------------

/// Maps a BlockType to the key used in `SpriteSheets::block_images` and the
/// default atlas frame.
fn block_sprite_info(bt: BlockType) -> (&'static str, usize) {
    match bt {
        BlockType::Breakable => ("breakableblock", 0),
        BlockType::PowerupBlock => ("powerupblock", 0),
        BlockType::DonutBlock => ("donutblock", 0),
        BlockType::FlipBlock => ("flipblock", 0),
        BlockType::BounceBlock => ("bounceblock", 0),
        BlockType::NoteBlock => ("noteblock", 0),
        BlockType::OnOffSwitch => ("switchblock", 0),
        BlockType::OnOffBlock => ("switchblock", 1),
        BlockType::WeaponBreakable => ("weaponbreakableblock", 0),
        BlockType::ViewBlock => ("viewblock", 0),
        BlockType::ThrowBlock => ("throwblock", 0),
    }
}

/// Attaches a Sprite to newly-spawned block entities that lack one.
pub fn setup_block_sprites(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    query: Query<
        (Entity, &BlockType),
        (Without<ObjectSpriteSetup>, With<BlockType>),
    >,
) {
    for (entity, block_type) in query.iter() {
        let (sheet_key, atlas_index) = block_sprite_info(*block_type);

        if let Some(sheet) = sprite_sheets.block_images.get(sheet_key) {
            commands.entity(entity).insert((
                Sprite {
                    image: sheet.texture.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: sheet.atlas_layout.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                ObjectSpriteSetup,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Hazard sprite setup
// ---------------------------------------------------------------------------

/// Maps a HazardType to the key used in `SpriteSheets::hazard_images` and the
/// default atlas frame.
fn hazard_sprite_info(ht: HazardType) -> (&'static str, usize) {
    match ht {
        HazardType::Fireball => ("fireball", 0),
        HazardType::FireBar => ("rotodisc", 0),
        HazardType::PiranhaPlant => ("pirhanaplant", 0),
        HazardType::BulletCannon => ("bulletbill", 0),
        HazardType::Thwomp => ("flame", 0),
        HazardType::Spikes => ("flame", 1),
    }
}

/// Attaches a Sprite to newly-spawned hazard entities that lack one.
pub fn setup_hazard_sprites(
    mut commands: Commands,
    sprite_sheets: Res<SpriteSheets>,
    query: Query<
        (Entity, &Hazard),
        (Without<ObjectSpriteSetup>, With<Hazard>),
    >,
) {
    for (entity, hazard) in query.iter() {
        let (sheet_key, atlas_index) = hazard_sprite_info(hazard.hazard_type);

        if let Some(sheet) = sprite_sheets.hazard_images.get(sheet_key) {
            commands.entity(entity).insert((
                Sprite {
                    image: sheet.texture.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: sheet.atlas_layout.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                ObjectSpriteSetup,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Enemy facing → sprite flip
// ---------------------------------------------------------------------------

/// Syncs enemy facing direction to sprite horizontal flip.
pub fn sync_enemy_facing(
    mut query: Query<
        (&EnemyFacing, &mut Sprite),
        (With<Enemy>, With<ObjectSpriteSetup>, Changed<EnemyFacing>),
    >,
) {
    for (facing, mut sprite) in query.iter_mut() {
        sprite.flip_x = *facing == EnemyFacing::Left;
    }
}
