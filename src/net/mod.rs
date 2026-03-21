use bevy::prelude::*;

/// Network session state.
#[derive(Resource, Debug, Default)]
pub struct NetworkState {
    pub connected: bool,
    pub is_host: bool,
    pub player_count: usize,
}

/// Networking module -- stub for future implementation.
///
/// The original SMW uses ENet for multiplayer networking.
/// This will eventually be replaced with a Bevy-compatible networking solution
/// (e.g., bevy_renet, lightyear, or similar).
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkState>();
    }
}
