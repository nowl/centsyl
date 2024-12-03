use crate::rng::CoinFlip;
use pixels::{Error, Pixels};
use types::DeltaPosition;

use crate::{
    components::*,
    data::*,
    game::{play_sound, CoreGame, GameState, TheRng},
    map::{self, Map},
    resources::{AudioHandler, LevelStats, PlayerEntity, PlayerPosition},
    rng::Rng,
    spawn,
    systems::*,
    utils::delta_to_next_map_position,
};
use legion::*;

pub enum UpdateResult {
    None,
    Exit,
}

pub fn do_update(game: &mut CoreGame) -> Result<UpdateResult, Error> {
    use crate::game::GameState::*;
    use winit::event::VirtualKeyCode;

    // handle resize
    if let Some(size) = game.input.window_resized() {
        let mut pixels = game.resources.get_mut::<Pixels>().unwrap();
        pixels.resize_surface(size.width, size.height)?;
    }

    let gamestate = *game.resources.get::<GameState>().unwrap();

    let mut new_gamestate = gamestate;

    match gamestate {
        Init => {
            new_gamestate = GenerateNewMap(1);
        }
        GenerateNewMap(level) => {
            game.resources.get_mut::<LevelStats>().unwrap().level = level;
            let mut rng = game.resources.get_mut::<TheRng>().unwrap();

            game.world.push((PlayerViewportFlag,));
            game.world.push((UpdateViewshedsFlag,));

            // clear enemies
            let mut query = <(Entity, &EnemyFlag)>::query();
            let entities = query
                .iter(&game.world)
                .map(|(e, ..)| *e)
                .collect::<Vec<_>>();
            for e in entities {
                game.world.remove(e);
            }

            // clear items
            let mut query = <(Entity, &EntityType)>::query();
            let entities = query
                .iter(&game.world)
                .filter(|&(_, et)| *et == EntityType::Ammo || *et == EntityType::Health)
                .map(|(e, ..)| *e)
                .collect::<Vec<_>>();
            for e in entities {
                game.world.remove(e);
            }

            let mut map = map::Map::new(10 + level, 6 + level, &mut rng);
            map.update_blocks_visibility();
            map.update_blocks_movement();

            // restore health and ammo
            let mut query = <(&PlayerFlag, &mut Health, &mut Ammo)>::query();
            for (_, health, ammo) in query.iter_mut(&mut game.world) {
                health.0 = 10;
                ammo.0 = 10;
            }

            // put player in random spot
            let open_spot = map.random_open_spot(&mut rng);
            let mut query = <(&PlayerFlag, &mut MapPosition)>::query();
            for (_, MapPosition { x, y }) in query.iter_mut(&mut game.world) {
                *x = open_spot.x;
                *y = open_spot.y;
            }
            {
                // update playerposition resource as well
                let mut p = game.resources.get_mut::<PlayerPosition>().unwrap();
                p.0 = open_spot;
            }

            //let viewport = map::MapViewport::new(20, 12, &map);
            let viewport = map::MapViewport::new(map.width.min(20), map.height.min(10), &map);

            let player_pos = game.resources.get::<PlayerPosition>().unwrap().0;
            for _ in 0..level * 2 {
                spawn::spawn_enemy(&mut game.world, &map, &mut rng, &player_pos, level);
            }

            for _ in 0..level {
                use CoinFlip::*;
                use EntityType::*;
                let etype = match rng.coin_flip() {
                    Heads => Ammo,
                    Tails => Health,
                };
                spawn::spawn_items(&mut game.world, &map, &mut rng, &player_pos, etype);
            }

            drop(rng);

            game.resources.insert(map);
            game.resources.insert(viewport);

            //update_viewshed_system(game);
            //update_map_visibilty_from_viewshed_system(game);
            update_text_info_system(game);

            new_gamestate = Running;
        }
        Running => {
            use types::Facing;

            let schedule = game
                .schedule_bag
                .schedules
                .get_mut(&ScheduleName::UpdateRunning)
                .unwrap();

            schedule.execute(&mut game.world, &mut game.resources);

            if game.input.key_pressed(VirtualKeyCode::Escape)
                || game.input.close_requested()
                || game.input.destroyed()
            {
                return Ok(UpdateResult::Exit);
            }

            if game.input.key_held(VirtualKeyCode::W) {
                try_move_player(game, MapPosition { x: 0, y: -1 }, Facing::Up);
            }

            if game.input.key_held(VirtualKeyCode::S) {
                try_move_player(game, MapPosition { x: 0, y: 1 }, Facing::Down);
            }

            if game.input.key_held(VirtualKeyCode::A) {
                try_move_player(game, MapPosition { x: -1, y: 0 }, Facing::Left);
            }

            if game.input.key_held(VirtualKeyCode::D) {
                try_move_player(game, MapPosition { x: 1, y: 0 }, Facing::Right);
            }

            if game.input.key_pressed(VirtualKeyCode::Up) {
                change_facing(game, Facing::Up);
                fire_projectile(game);
            }

            if game.input.key_pressed(VirtualKeyCode::Down) {
                change_facing(game, Facing::Down);
                fire_projectile(game);
            }

            if game.input.key_pressed(VirtualKeyCode::Left) {
                change_facing(game, Facing::Left);
                fire_projectile(game);
            }

            if game.input.key_pressed(VirtualKeyCode::Right) {
                change_facing(game, Facing::Right);
                fire_projectile(game);
            }

            if game.input.key_pressed(VirtualKeyCode::Space) {
                fire_projectile(game);
            }

            if dead_ememy_remover_system(game) {
                new_gamestate = PlayerDead;
            } else if check_map_complete_system(game) {
                let level = game.resources.get::<LevelStats>().unwrap().level;
                new_gamestate = GenerateNewMap(level + 1);
            }
            update_text_info_system(game);
        }
        PlayerDead => {
            if game.input.key_pressed(VirtualKeyCode::Space) {
                new_gamestate = Init;
            }
        }
    };

    game.resources.insert(new_gamestate);

    Ok(UpdateResult::None)
}

fn try_move_player(game: &mut CoreGame, delta: MapPosition, new_facing: types::Facing) {
    // check if currently moving
    {
        let player_entry = game.world.entry(game.entities.player).unwrap();
        if player_entry.get_component::<Moving>().is_ok() {
            return;
        }
    }

    let map = game.resources.get::<Map>().unwrap();
    let mut rng = game.resources.get_mut::<TheRng>().unwrap();

    // gather enemy locations
    let mut query = <(Entity, &MapPosition, Option<&Moving>, &EnemyFlag)>::query();
    let mut mob_positions = Vec::new();
    for (&entity, mpos @ &MapPosition { x, y }, moving, _enemy) in query.iter_mut(&mut game.world) {
        mob_positions.push((entity, map.idx(x, y)));
        if let Some(idx) = moving
            .map(|m| delta_to_next_map_position(mpos, &m.delta))
            .map(|p| map.idx(p.x, p.y))
        {
            mob_positions.push((entity, idx))
        }
    }

    let mut entry = game.world.entry(game.entities.player).unwrap();
    let current_facing = entry
        .get_component_mut::<RenderableSprite>()
        .unwrap()
        .facing;

    let player_pos = entry.get_component::<MapPosition>().unwrap();
    let target_pos = (player_pos.x + delta.x, player_pos.y + delta.y);
    let target_idx = map.idx(target_pos.0, target_pos.1);

    let map = game.resources.get::<Map>().unwrap();

    // adjust facing
    if current_facing != new_facing {
        let rend = entry.get_component_mut::<RenderableSprite>().unwrap();
        rend.facing = new_facing;
    }

    if map.blocks_movement[target_idx] {
        play_sound("hit3", &game.resources.get::<AudioHandler>().unwrap());
    } else if let Some((mentity, _midx)) = mob_positions.iter().find(|(_e, idx)| *idx == target_idx)
    {
        let &Health(strength) = entry.get_component::<Health>().unwrap();
        game.world.push((DealDamage {
            target: *mentity,
            amount: strength,
        },));

        if rng.d100() <= 55 {
            game.world.push((DealDamage {
                target: game.entities.player,
                amount: 1,
            },));
        }
    } else {
        let delta_f = DeltaPosition {
            x: delta.x as f32 / PLAYER_MOVE_TICKS as f32,
            y: delta.y as f32 / PLAYER_MOVE_TICKS as f32,
        };
        let mover = Moving {
            ticks_left: PLAYER_MOVE_TICKS as i32,
            total_ticks: PLAYER_MOVE_TICKS as i32,
            delta: delta_f,
        };
        entry.add_component(mover);

        let action_state = entry.get_component_mut::<ActionState>().unwrap();
        action_state.0 = types::Action::Moving;
        let anim = entry.get_component_mut::<AnimationState>().unwrap();
        anim.countdown_timer = 0;
        anim.current_frame = 0;
        let rend = entry.get_component_mut::<RenderableSprite>().unwrap();
        rend.facing = new_facing;
    }
}

fn change_facing(game: &mut CoreGame, new_facing: types::Facing) {
    // check if currently moving
    {
        let player_entry = game.world.entry(game.entities.player).unwrap();
        if player_entry.get_component::<Moving>().is_ok() {
            return;
        }
    }

    let mut entry = game.world.entry(game.entities.player).unwrap();

    let rend = entry.get_component_mut::<RenderableSprite>().unwrap();
    rend.facing = new_facing;
}

fn fire_projectile(game: &mut CoreGame) {
    use types::Facing::*;

    let player_pos = game.resources.get::<PlayerPosition>().unwrap();
    let player_entity = game.resources.get::<PlayerEntity>().unwrap();
    let mut player_entry = game.world.entry(player_entity.0).unwrap();

    let ammo = player_entry.get_component_mut::<Ammo>().unwrap();
    if ammo.0 == 0 {
        return;
    } else {
        ammo.0 -= 1;
    }

    let facing = player_entry
        .get_component::<RenderableSprite>()
        .unwrap()
        .facing;

    let pos = player_pos.0;

    let screen_draw_offset = match facing {
        Right => ScreenDrawOffset { x: 6, y: 0 },
        None | Left => ScreenDrawOffset { x: -6, y: 0 },
        Up => ScreenDrawOffset { x: 0, y: -8 },
        Down => ScreenDrawOffset { x: 0, y: 3 },
    };

    let speed = 2.0;
    let delta = match facing {
        Right => DeltaPosition { x: speed, y: 0.0 },
        None | Left => DeltaPosition { x: -speed, y: 0.0 },
        Up => DeltaPosition { x: 0.0, y: -speed },
        Down => DeltaPosition { x: 0.0, y: speed },
    };

    let duration = 50;
    let moving = Moving {
        ticks_left: duration,
        total_ticks: duration,
        delta,
    };

    play_sound("fire1", &game.resources.get::<AudioHandler>().unwrap());

    let rend = RenderableSprite {
        sprite_x: 5,
        sprite_y: 1,
        facing: types::Facing::None,
    };

    let components = (
        ProjectileFlag {
            origin_entity: player_entity.0,
        },
        EntityType::Projectile,
        moving,
        pos,
        ActionState(types::Action::Moving),
        AnimationState::default(),
        rend,
        TimeToLive(duration),
    );

    let entity = game.world.push(components);

    game.world
        .entry(entity)
        .unwrap()
        .add_component(screen_draw_offset);
}
