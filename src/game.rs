use std::rc::Rc;

use crate::components::*;
use crate::data::*;

use crate::resources::*;
use crate::shapes::FrameBufferTarget;
use crate::spritegrid;
use crate::ScheduleBag;
use embedded_graphics::text::Alignment;
use legion::*;
use pixels::{Pixels, SurfaceTexture};
use rand::SeedableRng;
use winit::window::Window;
use winit_input_helper::WinitInputHelper;

pub(crate) type TheRng = pcg_mwc::Mwc256XXA64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    Init,
    GenerateNewMap(i32),
    Running,
    PlayerDead,
}

pub struct EntityContainer {
    pub player: Entity,
    pub score: Entity,
    pub level: Entity,
    pub remaining: Entity,
    pub health: Entity,
    pub ammo: Entity,
}

pub type SpriteGrid = spritegrid::SpriteGrid<image::Rgb<u8>, Vec<u8>>;

pub struct CoreGame {
    pub input: WinitInputHelper,
    sprite_state: bool,

    pub world: World,
    pub resources: Resources,

    pub schedule_bag: ScheduleBag,

    pub entities: EntityContainer,

    // need to hold on to this value so it doesn't drop
    pub astream: rodio::OutputStream,
}

pub async fn init(window: Rc<Window>) -> CoreGame {
    use image::ImageReader;

    let (astream, astream_handle) = rodio::OutputStream::try_default().unwrap();
    let hit_sounds = create_sound_map();

    let img = ImageReader::new(std::io::Cursor::new(SPRITES))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();
    let sprite_grid = SpriteGrid::new(img, 16, 16, 32, 32 * 3);

    let shapes = FrameBufferTarget::new(SCREEN_WIDTH, SCREEN_HEIGHT);

    let pixels = {
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.as_ref());
        Pixels::new_async(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)
            .await
            .expect("Pixels error")
    };

    let input = WinitInputHelper::new();

    let sprite_state = true;

    let rng = TheRng::from_entropy();
    let json = serde_json::to_string(&rng).unwrap();
    log::info!("RNG Seed: {}", json);

    let mut world = World::default();
    let mut resources = Resources::default();

    let player_entity = world.push((
        PlayerFlag,
        AnimationState::default(),
        RenderableSprite::default(),
        ActionState(types::Action::Stationary),
        MapPosition { x: 1, y: 1 },
        Viewshed {
            visible: Vec::new(),
            range: 4,
        },
        Health(10),
        Score(0),
    ));

    world
        .entry(player_entity)
        .unwrap()
        .add_component(EntityType::Player);

    world.entry(player_entity).unwrap().add_component(Ammo(10));

    world.push((PlayerViewportFlag,));
    world.push((UpdateViewshedsFlag,));

    let level_info_entity = world.push((
        TextBlock {
            text: "Level: 1".to_owned(),
            color: (0, 210, 0),
            alignment: Alignment::Left,
        },
        FixedScreenPos { x: 5, y: 5 },
    ));

    let score_info_entity = world.push((
        TextBlock {
            text: "Score: 0".to_owned(),
            color: (0, 210, 0),
            alignment: Alignment::Left,
        },
        FixedScreenPos { x: 5, y: 5 + 8 },
    ));

    let remaining_info_entity = world.push((
        TextBlock {
            text: "Remaining: ??".to_owned(),
            color: (0, 210, 0),
            alignment: Alignment::Left,
        },
        FixedScreenPos { x: 5 + 100, y: 5 },
    ));

    let health_info_entity = world.push((
        TextBlock {
            text: "Health:".to_owned(),
            color: (0, 210, 0),
            alignment: Alignment::Left,
        },
        FixedScreenPos {
            x: 5 + 100 + 100,
            y: 5,
        },
    ));

    let ammo_info_entity = world.push((
        TextBlock {
            text: "Ammo:".to_owned(),
            color: (0, 210, 0),
            alignment: Alignment::Left,
        },
        FixedScreenPos {
            x: 5 + 100 + 100,
            y: 5 + 8,
        },
    ));

    let entities = EntityContainer {
        player: player_entity,
        level: level_info_entity,
        health: health_info_entity,
        ammo: ammo_info_entity,
        remaining: remaining_info_entity,
        score: score_info_entity,
    };

    let level_stats = LevelStats {
        level: 1,
        ..Default::default()
    };

    let audio = AudioHandler {
        astream_handle,
        hit_sounds,
    };

    resources.insert(PlayerEntity(player_entity));
    resources.insert(PlayerPosition::default());
    resources.insert(shapes);
    resources.insert(pixels);
    resources.insert(sprite_grid);
    resources.insert(level_stats);
    resources.insert(rng);
    resources.insert(GameState::Init);
    resources.insert(MobPositions::default());
    resources.insert(audio);

    CoreGame {
        input,
        sprite_state,
        world,
        resources,
        entities,
        schedule_bag: ScheduleBag::default(),
        astream,
    }
}

pub fn play_sound(sound: &str, audio: &AudioHandler) {
    let hit_sound = audio.hit_sounds.get(sound).unwrap().clone();
    let file = std::io::Cursor::new(hit_sound);
    let sink = audio.astream_handle.play_once(file).unwrap();
    sink.set_volume(0.8);
    sink.detach();
}
