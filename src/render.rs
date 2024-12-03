use std::ops::DerefMut;

use embedded_graphics::{mono_font, text::Alignment};
use pixels::{Error, Pixels};
use types::RenderPosition;
use world::SubWorld;

use crate::map::TileType;
use crate::resources::LevelStats;
use crate::ScheduleName;
use crate::{
    components::*,
    draw,
    game::{CoreGame, GameState, SpriteGrid},
    map::{self, Map, MapViewport},
    shapes::{self, FrameBufferTarget},
    SCREEN_HEIGHT, SCREEN_WIDTH,
};
use legion::*;

pub fn do_render(game: &mut CoreGame) -> Result<(), Error> {
    use GameState::*;

    let gamestate = *game.resources.get::<GameState>().unwrap();

    match gamestate {
        Running => {
            let schedule = game
                .schedule_bag
                .schedules
                .get_mut(&ScheduleName::RenderRunning)
                .unwrap();
            schedule.execute(&mut game.world, &mut game.resources);
        }
        PlayerDead => {
            render_finish(game);
        }
        _ => {}
    }

    let pixels = game.resources.get_mut::<Pixels>().unwrap();
    pixels.render()
}

#[system]
pub fn draw_map(
    #[resource] pixels: &mut Pixels,
    #[resource] sprite_grid: &SpriteGrid,
    #[resource] viewport: &MapViewport,
    #[resource] map: &Map,
) {
    use map::TileType::*;

    let frame = pixels.frame_mut();
    frame.iter_mut().for_each(|x| *x = 0);

    let y_offset = 2 * 16;

    for screen_x in -1..viewport.width + 1 {
        for screen_y in -1..viewport.height + 1 {
            if let Some((MapPosition { x: map_x, y: map_y }, (xfrac, yfrac))) =
                viewport.viewport_to_map_pos(screen_x, screen_y, map)
            {
                // draw floor
                let idx = map.idx(map_x, map_y);
                let (six, siy) = (1, 0); // floor
                if map.visible_cells[idx] {
                    let xoff = screen_x * 16 - (xfrac * 16.0) as i32;
                    let yoff = screen_y * 16 - (yfrac * 16.0) as i32;
                    draw::blit_sprite(
                        sprite_grid,
                        six,
                        siy,
                        xoff,
                        yoff + y_offset,
                        frame,
                        SCREEN_WIDTH,
                        SCREEN_HEIGHT,
                        false,
                    );

                    // draw wall
                    let typ = map.cells[idx];
                    if typ == Wall {
                        let (six, siy) = get_wall_tile(map, map_x, map_y);

                        let xoff = screen_x * 16 - (xfrac * 16.0) as i32;
                        let yoff = screen_y * 16 - (yfrac * 16.0) as i32;
                        draw::blit_sprite(
                            sprite_grid,
                            six,
                            siy,
                            xoff,
                            yoff + y_offset,
                            frame,
                            SCREEN_WIDTH,
                            SCREEN_HEIGHT,
                            false,
                        );
                    }
                }
            }
        }
    }
}

fn render_finish(game: &mut CoreGame) {
    let font = mono_font::ascii::FONT_9X18;

    let mut shapes = game.resources.get_mut::<FrameBufferTarget>().unwrap();

    shapes.clear();

    let mut score = 0;
    let mut query = <(&PlayerFlag, &Score)>::query();
    for (_player_flag, &Score(s)) in query.iter(&mut game.world) {
        score = s;
    }

    shapes::draw_text(
        shapes.deref_mut(),
        "OOF!!",
        160,
        60,
        (230, 0, 0),
        Alignment::Center,
        font,
    );
    let mut score_str = "Final Score: ".to_owned();
    score_str.push_str(&score.to_string());
    shapes::draw_text(
        shapes.deref_mut(),
        &score_str,
        160,
        75,
        (230, 0, 0),
        Alignment::Center,
        font,
    );

    let mut pixels = game.resources.get_mut::<Pixels>().unwrap();

    shapes.flush(pixels.frame_mut());
}

#[system]
pub fn new_level_text(
    #[resource] mut shapes: &mut FrameBufferTarget,
    #[resource] level_stats: &LevelStats,
    #[resource] pixels: &mut Pixels,
) {
    shapes.clear();

    let font = mono_font::ascii::FONT_9X18_BOLD;

    let mut score_str = "Level: ".to_owned();
    score_str.push_str(&level_stats.level.to_string());
    shapes::draw_text(
        shapes.deref_mut(),
        &score_str,
        160,
        75,
        (230, 0, 0),
        Alignment::Center,
        font,
    );

    shapes.flush(pixels.frame_mut());
}

#[system]
#[read_component(MapPosition)]
#[read_component(Moving)]
#[read_component(RenderableSprite)]
#[read_component(OnlyVisibleInPlayerFOV)]
#[read_component(ScreenDrawOffset)]
pub fn draw_renderables(
    world: &SubWorld,
    #[resource] viewport: &MapViewport,
    #[resource] map: &Map,
    #[resource] pixels: &mut Pixels,
    #[resource] sprite_grid: &SpriteGrid,
) {
    let y_offset = 2 * 16;

    let mut query = <(
        &MapPosition,
        &RenderableSprite,
        Option<&OnlyVisibleInPlayerFOV>,
        Option<&Moving>,
        Option<&ScreenDrawOffset>,
    )>::query();
    query.for_each(world, |(pos, renderable, only_in_fov, moving, offset)| {
        let &MapPosition { x: map_x, y: map_y } = pos;

        if let Some(RenderPosition { mut x, mut y }) = viewport.checked_map_to_screen_pos(
            *pos,
            16.0,
            RenderPosition {
                x: SCREEN_WIDTH as i32,
                y: SCREEN_HEIGHT as i32,
            },
        ) {
            let &RenderableSprite {
                sprite_x,
                sprite_y,
                facing,
            } = renderable;

            let idx = map.idx(map_x, map_y);
            if only_in_fov.is_some() && !map.visible_cells[idx] {
                return;
            }

            let flip_y = match facing {
                types::Facing::Right => true,
                _ => false,
            };

            // adjust x and y by moving delta
            if let Some(&Moving {
                ticks_left,
                total_ticks,
                delta,
            }) = moving
            {
                x += ((total_ticks - ticks_left) as f32 * delta.x).round() as i32;
                y += ((total_ticks - ticks_left) as f32 * delta.y).round() as i32;
            }

            // adjust by screen draw offset if applicable
            if let Some(&ScreenDrawOffset { x: xoff, y: yoff }) = offset {
                x += xoff;
                y += yoff;
            }

            draw::blit_sprite(
                sprite_grid,
                sprite_x as u32,
                sprite_y as u32,
                x,
                y + y_offset,
                pixels.frame_mut(),
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                flip_y,
            );
        }
    });
}

#[system]
#[read_component(TextBlock)]
#[read_component(Moving)]
#[read_component(MapPosition)]
#[read_component(FixedScreenPos)]
pub fn draw_text(
    world: &SubWorld,
    #[resource] mut shapes: &mut FrameBufferTarget,
    #[resource] viewport: &MapViewport,
    #[resource] pixels: &mut Pixels,
) {
    shapes.clear();

    let mut query = <(&MapPosition, &TextBlock, Option<&Moving>)>::query();
    query.for_each(world, |(pos, text, moving)| {
        //let font = embedded_graphics::mono_font::iso_8859_10::FONT_4X6;
        let font = embedded_graphics::mono_font::iso_8859_10::FONT_6X9;
        //let font = IBM437_8X8_REGULAR;

        if let Some(RenderPosition { x, y }) = viewport.checked_map_to_screen_pos(
            *pos,
            16.0,
            RenderPosition {
                x: SCREEN_WIDTH as i32,
                y: SCREEN_HEIGHT as i32,
            },
        ) {
            let &TextBlock {
                ref text,
                color,
                alignment,
            } = text;

            shapes::draw_text(shapes.deref_mut(), text, x, y, color, alignment, font);
        }
    });

    let mut query = <(&FixedScreenPos, &TextBlock)>::query();
    query.for_each(world, |(pos, text)| {
        //let font = embedded_graphics::mono_font::iso_8859_10::FONT_4X6;
        let font = embedded_graphics::mono_font::iso_8859_10::FONT_6X9;
        //let font = IBM437_8X8_REGULAR;

        let &FixedScreenPos { x, y } = pos;
        let &TextBlock {
            ref text,
            color,
            alignment,
        } = text;

        shapes::draw_text(shapes.deref_mut(), text, x, y, color, alignment, font);
    });

    shapes.flush(pixels.frame_mut());
}

fn get_wall_tile(map: &Map, x: i32, y: i32) -> (u32, u32) {
    macro_rules! aux {
        ($dx:expr, $dy:expr) => {{
            map.cells
                .get(map.idx(x + $dx, y + $dy))
                .map(|t| *t == TileType::Wall)
                .unwrap_or(true)
        }};
    }

    let west = aux!(-1, 0);
    let east = aux!(1, 0);
    let north = aux!(0, -1);
    let south = aux!(0, 1);
    let southeast = aux!(1, 1);

    if south && east && southeast {
        (0, 3)
    } else if !north && !south && !east && !west {
        (0, 0)
    } else if !north && south && !east {
        (0, 6)
    } else if north && !south && !east && !west {
        (0, 1)
    } else if south && east && !southeast {
        (0, 4)
    } else if north && south && !east {
        (0, 7)
    } else if !south && east && !west {
        (0, 2)
    } else if !south && east && west {
        (0, 5)
    } else if north && !south && !east && west {
        (0, 8)
    } else if !north && !south && !east && west {
        (0, 9)
    } else {
        unreachable!()
    }
}
