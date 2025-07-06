use bevy::prelude::*;
use super::map::{MapTile, HEX_SIZE};

#[derive(Component)]
pub struct Culled;

#[derive(Resource)]
pub struct ViewportCulling {
    pub enabled: bool,
    pub padding: f32, // Extra tiles to render beyond viewport
}

impl Default for ViewportCulling {
    fn default() -> Self {
        Self {
            enabled: true,
            padding: HEX_SIZE * 2.0, // Render 2 extra tiles beyond viewport
        }
    }
}

pub fn viewport_culling_system(
    mut commands: Commands,
    camera_query: Query<&Transform, (With<Camera>, Without<MapTile>)>,
    windows: Query<&Window>,
    tile_query: Query<(Entity, &Transform, &MapTile), Without<Culled>>,
    culled_query: Query<(Entity, &Transform, &MapTile), With<Culled>>,
    culling_settings: Res<ViewportCulling>,
) {
    if !culling_settings.enabled {
        return;
    }

    let Ok(camera_transform) = camera_query.single() else { return };
    let Ok(window) = windows.single() else { return };

    // Calculate viewport bounds in world space
    let camera_pos = camera_transform.translation.truncate();
    let half_width = window.width() / 2.0;
    let half_height = window.height() / 2.0;
    
    let viewport_min = camera_pos - Vec2::new(half_width, half_height) - Vec2::splat(culling_settings.padding);
    let viewport_max = camera_pos + Vec2::new(half_width, half_height) + Vec2::splat(culling_settings.padding);

    // Cull tiles that are outside viewport
    for (entity, transform, _) in tile_query.iter() {
        let tile_pos = transform.translation.truncate();
        
        if tile_pos.x < viewport_min.x || tile_pos.x > viewport_max.x ||
           tile_pos.y < viewport_min.y || tile_pos.y > viewport_max.y {
            commands.entity(entity).insert(Culled);
        }
    }

    // Un-cull tiles that are back in viewport
    for (entity, transform, _) in culled_query.iter() {
        let tile_pos = transform.translation.truncate();
        
        if tile_pos.x >= viewport_min.x && tile_pos.x <= viewport_max.x &&
           tile_pos.y >= viewport_min.y && tile_pos.y <= viewport_max.y {
            commands.entity(entity).remove::<Culled>();
        }
    }
}

// Plugin to easily add culling to your app
pub struct CullingPlugin;

impl Plugin for CullingPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ViewportCulling::default())
            .add_systems(Update, viewport_culling_system);
    }
}