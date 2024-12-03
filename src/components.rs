use legion::Entity;

pub mod types {
    #[derive(Clone, Copy, PartialEq, Default)]
    pub struct Pos<T> {
        pub x: T,
        pub y: T,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Action {
        Stationary,
        Moving,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Default)]
    pub enum Facing {
        #[default]
        None,
        Right,
        Left,
        Up,
        Down,
    }

    pub type RenderPosition = Pos<i32>;
    pub type DeltaPosition = Pos<f32>;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    MonsterA,
    MonsterB,
    MonsterC,
    Projectile,
    Explosion,
    Health,
    Ammo,
}

pub struct FixedScreenPos {
    pub x: i32,
    pub y: i32,
}
pub struct ScreenDrawOffset {
    pub x: i32,
    pub y: i32,
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct MapPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionState(pub types::Action);

#[derive(Default)]
pub struct RenderableSprite {
    pub sprite_x: u8,
    pub sprite_y: u8,
    pub facing: types::Facing,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct AnimationState {
    pub countdown_timer: i16,
    pub current_frame: usize,
}
pub struct PlayerFlag;

pub struct ProjectileFlag {
    pub origin_entity: Entity,
}

pub struct EnemyFlag;

pub struct PlayerViewportFlag;

pub struct OnlyVisibleInPlayerFOV;

pub struct UpdateViewshedsFlag;

pub struct ActionTimer {
    pub next_action_time: i32,
    pub increment_time: i32,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Moving {
    pub ticks_left: i32,
    pub total_ticks: i32,
    pub delta: types::DeltaPosition,
}

pub struct Viewshed {
    pub visible: Vec<MapPosition>,
    pub range: i32,
}

pub struct TimeToLive(pub i32);

pub struct TextBlock {
    pub text: String,
    pub color: (u8, u8, u8),
    pub alignment: embedded_graphics::text::Alignment,
}

#[derive(Clone, Copy)]
pub struct DealDamage {
    pub target: Entity,
    pub amount: i32,
}

pub struct Score(pub i32);
pub struct Health(pub i32);
pub struct Ammo(pub i32);

pub struct MoveTimer {
    pub time_left: i32,
}

/*
#[derive(Default)]
struct SampleFilter;

impl DynamicFilter for SampleFilter {
    fn prepare(&mut self, world: legion::world::WorldId) {}

    fn matches_archetype<F: legion::Fetch>(&mut self, fetch: &F) -> legion::query::FilterResult {
        let result = fetch
            .find::<EntityType>()
            .map(|t| t.iter().any(|entity| *entity == EntityType::Explosion))
            .unwrap_or(false);

        if result {
            legion::query::FilterResult::Match(true)
        } else {
            legion::query::FilterResult::Match(false)
        }
    }
}
*/
