use std::collections::HashMap;

use crate::components::*;
use crate::data::*;
use crate::game::play_sound;
use crate::game::CoreGame;
use crate::game::TheRng;
use crate::map::Map;
use crate::map::MapViewport;
use crate::render::*;
use crate::resources::*;
use crate::rng::CoinFlip;
use crate::rng::Rng;
use crate::spawn::get_move_speed_for_mob;
use crate::sprites::get_renderable;
use crate::utils::delta_to_next_map_position;
use crate::utils::effective_moving_map_position;

use bracket_pathfinding::prelude::{a_star_search, field_of_view, Point};
use legion::*;
use num_traits::Signed;
use systems::CommandBuffer;
use types::DeltaPosition;
use world::SubWorld;

#[derive(Hash, PartialEq, Eq)]
pub enum ScheduleName {
    RenderRunning,
    RenderNewMapWait,
    UpdateRunning,
}

pub struct ScheduleBag {
    pub schedules: HashMap<ScheduleName, Schedule>,
}

#[cfg(target_arch = "wasm32")]
impl Default for ScheduleBag {
    fn default() -> Self {
        use ScheduleName::*;

        let mut schedules = HashMap::new();

        schedules.insert(
            RenderRunning,
            Schedule::builder()
                .add_thread_local(draw_map_system())
                .add_thread_local(draw_renderables_system())
                .add_thread_local(draw_text_system())
                .build(),
        );

        schedules.insert(
            RenderNewMapWait,
            Schedule::builder()
                .add_thread_local(new_level_text_system())
                .build(),
        );

        schedules.insert(
            UpdateRunning,
            Schedule::builder()
                .add_thread_local(update_positions_system())
                .flush()
                .add_thread_local(update_viewsheds_system())
                .add_thread_local(update_player_viewport_system())
                .flush()
                .add_thread_local(gather_mob_positions_system())
                .add_thread_local(update_renderable_system())
                .add_thread_local(update_enemy_movement_system())
                .flush()
                .add_thread_local(projectile_collision_system())
                // keyboard input should go here
                .add_thread_local(deal_damage_system())
                .add_thread_local(update_time_to_live_system())
                .build(),
        );

        ScheduleBag { schedules }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Default for ScheduleBag {
    fn default() -> Self {
        use ScheduleName::*;

        let mut schedules = HashMap::new();

        schedules.insert(
            RenderRunning,
            Schedule::builder()
                .add_system(draw_map_system())
                .add_system(draw_renderables_system())
                .add_system(draw_text_system())
                .build(),
        );

        schedules.insert(
            RenderNewMapWait,
            Schedule::builder()
                .add_system(new_level_text_system())
                .build(),
        );

        schedules.insert(
            UpdateRunning,
            Schedule::builder()
                .add_system(update_positions_system())
                .flush()
                .add_system(update_viewsheds_system())
                .add_system(update_player_viewport_system())
                .flush()
                .add_system(gather_mob_positions_system())
                .add_system(update_renderable_system())
                .add_system(update_enemy_movement_system())
                .flush()
                .add_system(projectile_collision_system())
                // keyboard input should go here
                .add_system(deal_damage_system())
                .add_system(update_time_to_live_system())
                .build(),
        );

        ScheduleBag { schedules }
    }
}

#[system]
#[read_component(Entity)]
#[read_component(MapPosition)]
#[write_component(Viewshed)]
#[write_component(UpdateViewshedsFlag)]
pub fn update_viewsheds(world: &mut SubWorld, buffer: &mut CommandBuffer, #[resource] map: &Map) {
    let mut update_viewsheds = false;
    let mut query = <Entity>::query().filter(component::<UpdateViewshedsFlag>());
    query.for_each(world, |&entity| {
        update_viewsheds = true;
        buffer.remove(entity);
    });

    // early exit
    if !update_viewsheds {
        return;
    }

    // update all viewsheds
    let mut query = <(&MapPosition, &mut Viewshed)>::query();
    query.for_each_mut(world, |(&MapPosition { x, y }, viewshed)| {
        let fov = field_of_view(Point::new(x, y), viewshed.range, map);
        // fov.iter().for_each(|p| { println!("{:?}", p) });

        viewshed.visible = fov
            .iter()
            .filter(|p| p.x >= 0 && p.x < map.width && p.y >= 0 && p.y < map.height)
            .map(|p| MapPosition { x: p.x, y: p.y })
            .collect();
    });
}

#[system]
#[read_component(Entity)]
#[read_component(Viewshed)]
#[read_component(MapPosition)]
#[read_component(PlayerViewportFlag)]
pub fn update_player_viewport(
    world: &mut SubWorld,
    buffer: &mut CommandBuffer,
    #[resource] map: &mut Map,
    #[resource] &PlayerEntity(player_entity): &PlayerEntity,
    #[resource] viewport: &mut MapViewport,
    #[resource] player_world_pos: &PlayerPosition,
) {
    let mut update_player_viewport = false;
    let mut query = <Entity>::query().filter(component::<PlayerViewportFlag>());
    query.for_each(world, |&entity| {
        update_player_viewport = true;
        buffer.remove(entity);
    });

    // early exit
    if !update_player_viewport {
        return;
    }

    // update visible cells on map based on player viewshed
    //map.visible_cells.iter_mut().for_each(|v| *v = false);

    let player_entry = world.entry_mut(player_entity).unwrap();
    let viewshed = player_entry.get_component::<Viewshed>().unwrap();

    for &MapPosition { x, y } in viewshed.visible.iter() {
        let idx = map.idx(x, y);
        map.visible_cells[idx] = true;
    }

    // map update viewport
    let max_x = viewport.map_width - viewport.width;
    let max_y = viewport.map_height - viewport.height;
    viewport.ulx = (player_world_pos.0.x as f32 - (viewport.width as f32 / 2.0) + 1.0)
        .clamp(0.0, max_x as f32);
    viewport.uly = (player_world_pos.0.y as f32 - (viewport.height as f32 / 2.0) + 1.0)
        .clamp(0.0, max_y as f32);
}

#[system]
#[read_component(Entity)]
#[read_component(EntityType)]
#[write_component(MapPosition)]
#[write_component(Moving)]
#[write_component(ActionState)]
#[write_component(PlayerViewportFlag)]
#[write_component(UpdateViewshedsFlag)]
fn update_positions(
    world: &mut SubWorld,
    buffer: &mut CommandBuffer,
    #[resource] &PlayerEntity(player_entity): &PlayerEntity,
    #[resource] player_pos: &mut PlayerPosition,
    #[resource] map: &Map,
) {
    use EntityType::*;
    // gather items
    let mut query = <(Entity, &EntityType, &MapPosition)>::query();
    let item_positions = query
        .iter(world)
        .filter(|&(_, et, _)| *et == Ammo || *et == Health)
        .map(|(e, et, mp)| (map.idx(mp.x, mp.y), (*e, *et)))
        .collect::<HashMap<_, _>>();

    // update positions
    let mut query = <(
        Entity,
        &mut MapPosition,
        &mut Moving,
        Option<&mut ActionState>,
    )>::query();
    query.for_each_mut(
        world,
        |(&entity, MapPosition { x, y }, moving, action_state)| {
            let Moving {
                ticks_left, delta, ..
            } = moving;

            *ticks_left -= 1;

            if *ticks_left <= 0 {
                let dx = (delta.x.abs().ceil() * delta.x.signum()).clamp(-1.0, 1.0) as i32;
                let dy = (delta.y.abs().ceil() * delta.y.signum()).clamp(-1.0, 1.0) as i32;

                *x += dx;
                *y += dy;
                buffer.remove_component::<Moving>(entity);
                buffer.push((UpdateViewshedsFlag,));
                action_state.map(|s| s.0 = types::Action::Stationary);
            }

            if entity == player_entity {
                if *ticks_left <= 0 {
                    player_pos.0.x = *x;
                    player_pos.0.y = *y;

                    player_pos.1 = None;

                    if let Some(item) = item_positions.get(&map.idx(player_pos.0.x, player_pos.0.y))
                    {
                        let entity_type = item.1;
                        buffer.exec_mut(move |world, resources| {
                            let player_entity = resources.get::<PlayerEntity>().unwrap();
                            let mut player_entry = world.entry(player_entity.0).unwrap();
                            match entity_type {
                                Ammo => {
                                    let ammo = player_entry
                                        .get_component_mut::<crate::components::Ammo>()
                                        .unwrap();
                                    ammo.0 = 10.min(ammo.0 + 1);
                                }
                                Health => {
                                    let health = player_entry
                                        .get_component_mut::<crate::components::Health>()
                                        .unwrap();
                                    health.0 = 10.min(health.0 + 1);
                                }
                                _ => unreachable!(),
                            }
                        });

                        buffer.remove(item.0);
                    }

                    buffer.push((PlayerViewportFlag,));
                } else {
                    player_pos.1 = Some(delta_to_next_map_position(&player_pos.0, delta));
                }
            }
        },
    );
}

#[system]
#[read_component(EntityType)]
#[read_component(ActionState)]
#[write_component(AnimationState)]
#[write_component(RenderableSprite)]
fn update_renderable(world: &mut SubWorld) {
    let mut query = <(
        &EntityType,
        &ActionState,
        &mut AnimationState,
        &mut RenderableSprite,
    )>::query();
    query.for_each_mut(world, |(etype, act, anim, r)| {
        let rend = get_renderable(etype, *act, anim, r.facing);

        r.sprite_x = rend.sprite_x;
        r.sprite_y = rend.sprite_y;
    });
}

#[system]
#[read_component(Entity)]
#[write_component(TimeToLive)]
pub fn update_time_to_live(world: &mut SubWorld, buffer: &mut CommandBuffer) {
    let mut query = <(Entity, &mut TimeToLive)>::query();
    query.for_each_mut(world, |(&entity, TimeToLive(ticks_left))| {
        *ticks_left -= 1;

        if *ticks_left <= 0 {
            buffer.remove(entity);
        }
    });
}

#[system]
#[read_component(Entity)]
#[read_component(Viewshed)]
#[read_component(MapPosition)]
#[read_component(EnemyFlag)]
#[read_component(EntityType)]
#[write_component(MoveTimer)]
#[write_component(DealDamage)]
pub fn update_enemy_movement(
    #[resource] map: &Map,
    #[resource] rng: &mut TheRng,
    #[resource] PlayerEntity(player_entity): &PlayerEntity,
    #[resource] player_position: &PlayerPosition,
    world: &mut SubWorld,
    buffer: &mut CommandBuffer,
) {
    let player_world_pos = player_position.0;
    let player_pos_idx = map.idx(player_world_pos.x, player_world_pos.y);
    let player_world_pos_next = player_position.1;
    let player_pos_next_idx = player_world_pos_next.map(|p| map.idx(p.x, p.y));
    let mut query = <(
        Entity,
        &Viewshed,
        &MapPosition,
        &mut MoveTimer,
        &EntityType,
        &EnemyFlag,
    )>::query();
    let mut to_move = Vec::new();
    let mut to_attack = Vec::new();
    let mut mob_positions = Vec::new();
    query.for_each_mut(
        world,
        |(&entity, viewshed, &MapPosition { x, y }, move_timer, entity_type, _enemy)| {
            mob_positions.push(map.idx(x, y));
            move_timer.time_left -= 1;
            if move_timer.time_left > 0 {
                return;
            }

            move_timer.time_left = get_move_speed_for_mob(*entity_type, rng);

            if viewshed.visible.contains(&MapPosition {
                x: player_world_pos.x,
                y: player_world_pos.y,
            }) {
                let start = map.idx(x, y);
                let path = a_star_search(start, player_pos_idx, map);
                if path.success {
                    if path.steps.len() < 10 && path.steps.len() > 2 {
                        // move
                        let next_step_idx = path.steps[1];
                        // make sure not moving into next player move position
                        if player_pos_next_idx.is_none_or(|idx| idx != next_step_idx) {
                            let delta = match next_step_idx as i32 - start as i32 {
                                1 => MapPosition { x: 1, y: 0 },
                                -1 => MapPosition { x: -1, y: 0 },
                                a if a < 0 => MapPosition { x: 0, y: -1 },
                                _ => MapPosition { x: 0, y: 1 },
                            };

                            let delta_f = DeltaPosition {
                                x: delta.x as f32 / PLAYER_MOVE_TICKS as f32,
                                y: delta.y as f32 / PLAYER_MOVE_TICKS as f32,
                            };
                            let mover = Moving {
                                ticks_left: PLAYER_MOVE_TICKS as i32,
                                total_ticks: PLAYER_MOVE_TICKS as i32,
                                delta: delta_f,
                            };
                            to_move.push((entity, mover, next_step_idx));
                        }
                    } else if path.steps.len() == 2 {
                        // attack
                        to_attack.push(entity);
                    }
                }
            }
        },
    );

    let mut taken_spots = Vec::new();
    for (entity, mover, dest_idx) in to_move {
        // check if there's an existing mob there
        if !mob_positions.contains(&dest_idx) && !taken_spots.contains(&dest_idx) {
            taken_spots.push(dest_idx);
            buffer.add_component(entity, mover);
            //let anim = enemy_state_to_renderable(MONSTER_A, EnemyState::Moving, Facing::Right);
            //entry.add_component(anim);
        }
    }

    for _entity in to_attack {
        let damage = DealDamage {
            target: *player_entity,
            amount: 1,
        };

        buffer.push((damage,));
    }
}

#[system]
#[read_component(Entity)]
#[read_component(DealDamage)]
#[write_component(Health)]
pub fn deal_damage(
    world: &mut SubWorld,
    buffer: &mut CommandBuffer,
    #[resource] audio: &mut AudioHandler,
) {
    let mut dmg = Vec::new();

    let mut query = <(Entity, &DealDamage)>::query();
    for (&entity, d) in query.iter_mut(world) {
        dmg.push(*d);
        buffer.remove(entity);
    }

    for DealDamage { target, amount } in dmg {
        if let Ok(mut entry) = world.entry_mut(target) {
            let health = entry.get_component_mut::<Health>().unwrap();
            health.0 -= amount;
            health.0 = health.0.max(0);
            let current_health = health.0;

            play_sound("hit1", audio);

            if current_health >= 0 {
                //let pos = entry.get_component::<PosF>().unwrap();
                //let target_pos = (pos.x.round(), pos.y.round());
                //game.world.push((
                //    TextBlock {
                //        text: current_health.to_string(),
                //        color: (200, 200, 0),
                //        alignment: Alignment::Center,
                //    },
                //    Moving {
                //        ticks_left: 64,
                //        delta: PosF {
                //            x: 0.0,
                //            y: 1.0 / 64.0,
                //        },
                //    },
                //    PosF {
                //        x: target_pos.0 + 0.5,
                //        y: target_pos.1 + 0.5,
                //    },
                //    TimeToLive(32),
                //));
            }
        }
    }
}

fn spawn_item(world: &mut World, entity_type: EntityType, pos: MapPosition) {
    world.push((
        entity_type,
        OnlyVisibleInPlayerFOV,
        RenderableSprite::default(),
        AnimationState::default(),
        ActionState(types::Action::Stationary),
        pos,
    ));
}

pub fn dead_ememy_remover_system(game: &mut CoreGame) -> bool {
    let mut query = <(Entity, &Health, &MapPosition)>::query();
    let mut removers = Vec::new();
    let mut spawn = None;
    let mut score_mod = 0;
    let mut player_dead = false;
    for (entity, &Health(h), pos) in query.iter_mut(&mut game.world) {
        if h <= 0 {
            if *entity == game.entities.player {
                // player dead
                player_dead = true;
            } else {
                removers.push((*entity, *pos));

                score_mod += 1;

                // random drop
                let mut rng = game.resources.get_mut::<TheRng>().unwrap();
                if rng.d100() <= 75 {
                    if rng.coin_flip() == CoinFlip::Heads {
                        spawn = Some((EntityType::Ammo, pos.clone()));
                    } else {
                        spawn = Some((EntityType::Health, pos.clone()));
                    }
                }
            }
        }
    }

    if let Some((entity_type, pos)) = spawn {
        spawn_item(&mut game.world, entity_type, pos);
    }

    for (entity, pos) in removers {
        game.world.remove(entity);

        play_sound("explode1", &game.resources.get::<AudioHandler>().unwrap());

        let explosion = (
            pos,
            EntityType::Explosion,
            ActionState(types::Action::Stationary),
            AnimationState {
                countdown_timer: 10,
                ..Default::default()
            },
            RenderableSprite {
                sprite_x: 5,
                sprite_y: 2,
                facing: types::Facing::None,
            },
            TimeToLive(30),
        );
        game.world.push(explosion);
    }

    if score_mod > 0 {
        let mut query = <(&PlayerFlag, &mut Score, &Health, &mut Viewshed)>::query();
        for (_player_flag, Score(score), &Health(health), vs) in query.iter_mut(&mut game.world) {
            *score += score_mod;
            vs.range = health.clamp(2, 8);
        }
    }

    player_dead
}

pub fn update_text_info_system(game: &mut CoreGame) {
    let mut query = <(&PlayerFlag, &Score, &Health, &Ammo)>::query();
    let mut score = 0;
    let mut health = 0;
    let mut ammo = 0;
    for (_player_flag, &Score(a), &Health(b), &Ammo(c)) in query.iter(&game.world) {
        score = a;
        health = b;
        ammo = c;
    }

    let mut query = <(&EnemyFlag,)>::query();
    let enemy_count = query.iter(&game.world).count();

    let mut entry = game.world.entry(game.entities.score).unwrap();
    let text = entry.get_component_mut::<TextBlock>().unwrap();
    text.text = "Score: ".to_owned();
    text.text.push_str(&score.to_string());

    let mut entry = game.world.entry(game.entities.remaining).unwrap();
    let text = entry.get_component_mut::<TextBlock>().unwrap();
    text.text = "Remaining: ".to_owned();
    text.text.push_str(&enemy_count.to_string());

    let mut entry = game.world.entry(game.entities.health).unwrap();
    let text = entry.get_component_mut::<TextBlock>().unwrap();
    text.text = "Health: ".to_owned();
    for _ in 0..health {
        text.text.push('O');
    }

    let mut entry = game.world.entry(game.entities.ammo).unwrap();
    let text = entry.get_component_mut::<TextBlock>().unwrap();
    text.text = "Ammo:   ".to_owned();
    for _ in 0..ammo {
        text.text.push('O');
    }

    let level = game.resources.get::<LevelStats>().unwrap().level;

    let mut entry = game.world.entry(game.entities.level).unwrap();
    let text = entry.get_component_mut::<TextBlock>().unwrap();
    text.text = "Level: ".to_owned();
    text.text.push_str(&level.to_string());
}

pub fn check_map_complete_system(game: &mut CoreGame) -> bool {
    let mut query = <(&EntityType, Option<&EnemyFlag>)>::query();
    query
        .iter(&game.world)
        .filter(|&(et, ef)| *et == EntityType::Explosion || ef.is_some())
        .count()
        == 0
}

#[system]
#[read_component(Entity)]
#[read_component(EnemyFlag)]
#[read_component(MapPosition)]
fn gather_mob_positions(world: &mut SubWorld, #[resource] mob_positions: &mut MobPositions) {
    mob_positions.positions.clear();

    let query = <(Entity, &MapPosition)>::query();
    query
        .filter(component::<EnemyFlag>())
        .for_each(world, |(entity, pos)| {
            mob_positions.positions.push((*pos, *entity));
        });
}

#[system]
#[read_component(ProjectileFlag)]
#[read_component(MapPosition)]
#[read_component(Moving)]
#[read_component(ScreenDrawOffset)]
fn projectile_collision(
    #[resource] mob_positions: &MobPositions,
    #[resource] map: &Map,
    #[resource] audio: &AudioHandler,
    world: &mut SubWorld,
    buffer: &mut CommandBuffer,
) {
    let mut query = <(Entity, &MapPosition, &Moving, Option<&ScreenDrawOffset>)>::query()
        .filter(component::<ProjectileFlag>());
    query.for_each(world, |(entity, pos, moving, offset)| {
        let projectile_position = effective_moving_map_position(pos, moving, offset);
        // check for collision with wall
        let idx = map.idx(projectile_position.x, projectile_position.y);
        if map.blocks_movement[idx] {
            buffer.remove(*entity);
            return;
        }

        // check for collision with mob
        mob_positions.positions.iter().for_each(|(p, mentity)| {
            if projectile_position == *p {
                buffer.remove(*entity);

                let damage = DealDamage {
                    target: *mentity,
                    amount: 1,
                };

                play_sound("hit2", audio);

                buffer.push((damage,));
            }
        });
    });
}
