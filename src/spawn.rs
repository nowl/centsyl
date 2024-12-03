use legion::*;
use num_traits::Signed;

use crate::rng::Rng;

use super::{components::*, game::TheRng, map::Map};

pub enum EnemyState {
    Stationary,
    Moving,
}

fn mob_positions(world: &mut World) -> Vec<MapPosition> {
    let mut query = <(&MapPosition, &EnemyFlag, Option<&Moving>)>::query();
    query
        .iter(world)
        .flat_map(|(&MapPosition { x, y }, _, moving)| {
            if let Some(&Moving { delta, .. }) = moving {
                let dx = (delta.x.abs().ceil() * delta.x.signum()).clamp(-1.0, 1.0) as i32;
                let dy = (delta.y.abs().ceil() * delta.y.signum()).clamp(-1.0, 1.0) as i32;
                Vec::from([
                    MapPosition { x, y },
                    MapPosition {
                        x: x + dx,
                        y: y + dy,
                    },
                ])
            } else {
                Vec::from([MapPosition { x, y }])
            }
        })
        .collect()
}

fn item_positions(world: &mut World) -> Vec<MapPosition> {
    let mut query = <(&MapPosition, &EntityType)>::query();
    query
        .iter(world)
        .filter(|&(_, et)| *et == EntityType::Health || *et == EntityType::Ammo)
        .map(|(pos, _)| pos.clone())
        .collect()
}

pub fn spawn_items(
    world: &mut World,
    map: &Map,
    rng: &mut TheRng,
    player_pos: &MapPosition,
    entity_type: EntityType,
) {
    let mob_positions = mob_positions(world);
    let item_positions = item_positions(world);
    let player_pos_idx = map.idx(player_pos.x, player_pos.y);
    let pos: MapPosition;
    loop {
        let p = map.random_open_spot(rng);
        if !mob_positions.contains(&p)
            && !item_positions.contains(&p)
            && map.open_path_a_b(map.idx(p.x, p.y), player_pos_idx)
        {
            pos = p;
            break;
        }
    }

    world.push((
        entity_type,
        OnlyVisibleInPlayerFOV,
        RenderableSprite::default(),
        AnimationState::default(),
        ActionState(types::Action::Stationary),
        pos,
    ));
}

pub fn spawn_enemy(
    world: &mut World,
    map: &Map,
    rng: &mut TheRng,
    player_pos: &MapPosition,
    level: i32,
) {
    let current_positions = mob_positions(world);
    let item_positions = item_positions(world);
    let player_pos_idx = map.idx(player_pos.x, player_pos.y);
    let pos: MapPosition;
    loop {
        let p = map.random_open_spot(rng);
        if !current_positions.contains(&p)
            && p != *player_pos
            && !item_positions.contains(&p)
            && map.open_path_a_b(map.idx(p.x, p.y), player_pos_idx)
        {
            pos = p;
            break;
        }
    }

    let roll = rng.d100();
    let monter_type = if roll > 50 {
        EntityType::MonsterA
    } else if roll > 20 {
        EntityType::MonsterB
    } else {
        EntityType::MonsterC
    };

    let health = match monter_type {
        EntityType::MonsterA => 1,
        EntityType::MonsterB => 2,
        EntityType::MonsterC => 3,
        _ => unreachable!(),
    };

    let entity = world.push((
        EnemyFlag,
        OnlyVisibleInPlayerFOV,
        ActionState(types::Action::Stationary),
        RenderableSprite::default(),
        AnimationState::default(),
        pos,
        Viewshed {
            visible: Vec::new(),
            range: level.clamp(8, 50),
        },
        Health(health),
    ));

    world.entry(entity).unwrap().add_component(MoveTimer {
        time_left: get_move_speed_for_mob(monter_type, rng),
    });

    world.entry(entity).unwrap().add_component(monter_type);
}

pub fn get_move_speed_for_mob(entity_type: EntityType, rng: &mut TheRng) -> i32 {
    match entity_type {
        EntityType::MonsterA => rng.d10() as i32 * 10,
        EntityType::MonsterB => rng.d10() as i32 * 25,
        EntityType::MonsterC => rng.d10() as i32 * 40,
        _ => unreachable!(),
    }
}
