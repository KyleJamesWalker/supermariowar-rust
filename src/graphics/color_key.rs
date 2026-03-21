use bevy::prelude::*;
use bevy::image::TextureFormatPixelInfo;
use bevy::render::render_resource::TextureFormat;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Resource: tracks which images have already been processed
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct ProcessedColorKeyImages {
    processed: HashSet<AssetId<Image>>,
}

// ---------------------------------------------------------------------------
// System: convert magenta color-key pixels to transparent
// ---------------------------------------------------------------------------

/// Listens for newly loaded `Image` assets and converts magenta (255, 0, 255)
/// pixels to fully transparent (0, 0, 0, 0).
///
/// Many Super Mario War assets use RGB PNGs (no alpha channel) with magenta as
/// a color-key for transparency. Bevy does not support color-key transparency
/// natively, so we post-process the raw pixel data after the image is loaded.
pub fn process_color_key_transparency(
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut processed: ResMut<ProcessedColorKeyImages>,
) {
    // Collect the IDs we need to process first, to avoid borrow conflicts.
    let mut ids_to_process: Vec<AssetId<Image>> = Vec::new();

    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id } | AssetEvent::LoadedWithDependencies { id } => {
                if !processed.processed.contains(id) {
                    ids_to_process.push(*id);
                }
            }
            _ => {}
        }
    }

    for id in ids_to_process {
        if processed.processed.contains(&id) {
            continue;
        }

        let Some(image) = images.get_mut(id) else {
            continue;
        };

        let converted = convert_magenta_to_transparent(image);
        if converted {
            processed.processed.insert(id);
        }
    }
}

/// Converts all magenta (R=255, G=0, B=255) pixels in the image to fully
/// transparent (0, 0, 0, 0). Handles both `Rgba8UnormSrgb` and `Rgba8Unorm`
/// formats. For images still in RGB format (no alpha), converts to RGBA first.
///
/// Returns `true` if the image was processed, `false` if it was skipped
/// (unsupported format or no magenta pixels found).
fn convert_magenta_to_transparent(image: &mut Image) -> bool {
    // If the image is in an RGB format without alpha, convert it to RGBA first.
    let needs_rgb_to_rgba = matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Bgra8UnormSrgb
            | TextureFormat::Bgra8Unorm
    );

    let is_rgb_only = matches!(
        image.texture_descriptor.format,
        TextureFormat::R8Unorm | TextureFormat::Rg8Unorm
    );

    // Bevy typically decodes PNGs to Rgba8UnormSrgb. If for some reason the
    // image is in a format we don't handle, skip it.
    if !needs_rgb_to_rgba && !is_rgb_only {
        // Try to convert the image to Rgba8UnormSrgb if possible.
        if let Some(converted) = image.convert(TextureFormat::Rgba8UnormSrgb) {
            *image = converted;
        } else {
            // Cannot convert; skip this image.
            return false;
        }
    }

    let is_bgra = matches!(
        image.texture_descriptor.format,
        TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm
    );

    let pixel_size = image
        .texture_descriptor
        .format
        .pixel_size();

    if pixel_size < 4 {
        // Not enough bytes per pixel to have RGBA; skip.
        return false;
    }

    let data = &mut image.data;
    let mut any_converted = false;

    for chunk in data.chunks_exact_mut(pixel_size) {
        let (r, g, b) = if is_bgra {
            (chunk[2], chunk[1], chunk[0])
        } else {
            (chunk[0], chunk[1], chunk[2])
        };

        if r == 255 && g == 0 && b == 255 {
            // Set to fully transparent black
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
            chunk[3] = 0;
            any_converted = true;
        }
    }

    if any_converted {
        debug!(
            "Color-key: converted magenta pixels to transparent in image ({:?})",
            image.texture_descriptor.format
        );
    }

    true
}
