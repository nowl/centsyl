use super::components::*;

macro_rules! sprite_xy {
    ($x:expr, $y:expr) => {
        RenderableSprite {
            sprite_x: $x,
            sprite_y: $y,
            facing: types::Facing::None,
        }
    };
}

pub fn get_renderable(
    typ: &EntityType,
    s: ActionState,
    anim: &mut AnimationState,
    facing: types::Facing,
) -> RenderableSprite {
    macro_rules! anim_countdown {
        ($durations:expr) => {
            anim.countdown_timer -= 1;
            if anim.countdown_timer <= 0 {
                anim.countdown_timer += $durations[anim.current_frame];
                anim.current_frame = (anim.current_frame + 1) % $durations.len();
            }
        };
    }

    use types::Action::*;
    use types::Facing::*;
    use EntityType::*;
    match typ {
        Player => match facing {
            None | Left | Right => match s.0 {
                Moving => {
                    anim_countdown!([75; 2]);

                    match anim.current_frame {
                        0 => sprite_xy!(6, 1),
                        1 => sprite_xy!(6 + 32, 1),
                        _ => unreachable!(),
                    }
                }
                Stationary => sprite_xy!(6, 1),
            },
            Up => sprite_xy!(6, 0),
            Down => sprite_xy!(6, 2),
        },
        MonsterA => match s.0 {
            Moving => sprite_xy!(8, 1),
            Stationary => {
                anim_countdown!([10, 10, 10, 100]);

                match anim.current_frame {
                    0 => sprite_xy!(8, 1),
                    1 => sprite_xy!(8 + 32, 1),
                    2 => sprite_xy!(8, 1),
                    3 => sprite_xy!(8 + 32 * 2, 1),
                    _ => unreachable!(),
                }
            }
        },
        MonsterB => match s.0 {
            Moving => sprite_xy!(9, 1),
            Stationary => {
                anim_countdown!([10, 10, 10, 100]);

                match anim.current_frame {
                    0 => sprite_xy!(9, 1),
                    1 => sprite_xy!(9 + 32, 1),
                    2 => sprite_xy!(9, 1),
                    3 => sprite_xy!(9 + 32 * 2, 1),
                    _ => unreachable!(),
                }
            }
        },
        MonsterC => match s.0 {
            Moving => sprite_xy!(9, 1),
            Stationary => {
                anim_countdown!([20, 20]);

                match anim.current_frame {
                    0 => sprite_xy!(9, 1),
                    1 => sprite_xy!(11, 2),
                    _ => unreachable!(),
                }
            }
        },
        Projectile => {
            anim_countdown!([5; 2]);

            match anim.current_frame {
                0 => sprite_xy!(5, 1),
                1 => sprite_xy!(5 + 32, 1),
                _ => unreachable!(),
            }
        }
        Explosion => {
            anim_countdown!([10; 3]);

            match anim.current_frame {
                0 => sprite_xy!(5, 2),
                1 => sprite_xy!(5 + 32, 2),
                2 => sprite_xy!(5 + 2 * 32, 2),
                _ => unreachable!(),
            }
        }
        Health => sprite_xy!(1, 1),
        Ammo => sprite_xy!(1, 2),
    }
}
