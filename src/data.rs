use std::{collections::HashMap, io::Write};

const HIT1: &[u8] = include_bytes!("../hit1.wav");
const HIT2: &[u8] = include_bytes!("../hit2.wav");
const HIT3: &[u8] = include_bytes!("../hit3.wav");
const FIRE1: &[u8] = include_bytes!("../fire1.wav");
const EXPLODE1: &[u8] = include_bytes!("../explode1.wav");

pub const SPRITES: &[u8] = include_bytes!("../isometric.png");

pub const SCREEN_WIDTH: u32 = 320;
pub const SCREEN_HEIGHT: u32 = 192;

pub const PLAYER_MOVE_TICKS: u32 = 8;

pub const MONSTER_A: &str = "MONSTER_A";

fn convert_sound_to_vec(bytes: &[u8]) -> Vec<u8> {
    let mut hit_sound_data = Vec::new();
    hit_sound_data.write(bytes).unwrap();
    hit_sound_data
}

pub fn create_sound_map() -> HashMap<&'static str, Vec<u8>> {
    let mut map = HashMap::new();
    map.insert("hit1", convert_sound_to_vec(HIT1));
    map.insert("hit2", convert_sound_to_vec(HIT2));
    map.insert("hit3", convert_sound_to_vec(HIT3));
    map.insert("fire1", convert_sound_to_vec(FIRE1));
    map.insert("explode1", convert_sound_to_vec(EXPLODE1));
    map
}
