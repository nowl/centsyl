use num_traits::Signed;
use types::DeltaPosition;

use crate::components::*;

// determines map position for a moving entity that doesn't update it's own
// MapPosition and has an option draw offset (Projectiles)
pub fn effective_moving_map_position(
    &MapPosition { x, y }: &MapPosition,
    &Moving {
        ticks_left,
        total_ticks,
        delta,
    }: &Moving,
    offset: Option<&ScreenDrawOffset>,
) -> MapPosition {
    let mut move_offset_x = ((total_ticks - ticks_left) as f32 * delta.x).round() as i32;
    let mut move_offset_y = ((total_ticks - ticks_left) as f32 * delta.y).round() as i32;
    if let Some(offset) = offset {
        move_offset_x += offset.x;
        move_offset_y += offset.y;
    };

    MapPosition {
        x: x + move_offset_x / 16,
        y: y + move_offset_y / 16,
    }
}

// determines next map position based on a delta
pub fn delta_to_next_map_position(
    &MapPosition { x, y }: &MapPosition,
    delta: &DeltaPosition,
) -> MapPosition {
    let dx = (delta.x.abs().ceil() * delta.x.signum()).clamp(-1.0, 1.0) as i32;
    let dy = (delta.y.abs().ceil() * delta.y.signum()).clamp(-1.0, 1.0) as i32;

    MapPosition {
        x: x + dx,
        y: y + dy,
    }
}
