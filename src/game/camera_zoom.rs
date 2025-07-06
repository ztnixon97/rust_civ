use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;

/// System to zoom the camera in and out with the mouse wheel.
pub fn camera_zoom_system(
    mut scroll_evr: EventReader<MouseWheel>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut zoom_delta = 0.0;
    for ev in scroll_evr.read() {
        zoom_delta += ev.y;
    }
    if zoom_delta.abs() > 0.0f32 {
        if let Ok(mut transform) = query.single_mut() {
            // Clamp scale to avoid flipping or disappearing
            let min_scale = 0.2;
            let max_scale = 4.0;
            let scale_change = 1.0 - zoom_delta * 0.1;
            let new_scale = (transform.scale.x * scale_change).clamp(min_scale, max_scale);
            transform.scale = Vec3::splat(new_scale);
        }
    }
}
