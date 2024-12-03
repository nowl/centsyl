use std::collections::HashMap;

use legion::*;

use crate::components::*;

pub struct PlayerEntity(pub Entity);

#[derive(Default)]
pub struct LevelStats {
    pub level: i32,
}

#[derive(Default)]
pub struct PlayerPosition(pub MapPosition, pub Option<MapPosition>);

#[derive(Default)]
pub struct MobPositions {
    pub positions: Vec<(MapPosition, Entity)>,
}

pub struct AudioHandler {
    pub astream_handle: rodio::OutputStreamHandle,
    pub hit_sounds: HashMap<&'static str, Vec<u8>>,
}
