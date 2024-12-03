use crate::components::*;

use crate::game::TheRng;
use crate::rng::Rng;
use bracket_pathfinding::prelude::{a_star_search, Algorithm2D, BaseMap, Point, SmallVec};
use types::RenderPosition;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Wall,
    Floor,
}

pub struct Map {
    pub width: i32,
    pub height: i32,
    pub cells: Vec<TileType>,
    pub blocks_movement: Vec<bool>,
    pub blocks_visibility: Vec<bool>,
    pub visible_cells: Vec<bool>,
}

pub struct MapViewport {
    pub ulx: f32,
    pub uly: f32,
    pub width: i32,
    pub height: i32,
    pub map_width: i32,
    pub map_height: i32,
}

impl MapViewport {
    pub fn new(width: i32, height: i32, map: &Map) -> Self {
        MapViewport {
            ulx: 0.0,
            uly: 0.0,
            width,
            height,
            map_width: map.width,
            map_height: map.height,
        }
    }

    pub fn viewport_to_map_pos(
        &self,
        x: i32,
        y: i32,
        map: &Map,
    ) -> Option<(MapPosition, (f32, f32))> {
        let xint = self.ulx.floor();
        let xfrac = self.ulx - xint;
        let yint = self.uly.floor();
        let yfrac = self.uly - yint;

        let pos = MapPosition {
            x: x + xint as i32,
            y: y + yint as i32,
        };
        if pos.x < 0 || pos.x >= map.width || pos.y < 0 || pos.y >= map.height {
            None
        } else {
            Some((pos, (xfrac, yfrac)))
        }
    }

    pub fn move_offset(&mut self, x: f32, y: f32) {
        self.ulx += x;
        self.uly += y;

        self.ulx = self.ulx.clamp(0.0, (self.map_width - self.width) as f32);
        self.uly = self.uly.clamp(0.0, (self.map_height - self.height) as f32);
    }

    pub fn checked_map_to_screen_pos(
        &self,
        pos: MapPosition,
        mult: f32,
        screen_bounds: RenderPosition,
    ) -> Option<RenderPosition> {
        let pos @ RenderPosition { x, y } = self.map_to_screen_pos(pos, mult);
        if x < 0 || x >= screen_bounds.x || y < 0 || y > screen_bounds.y {
            None
        } else {
            Some(pos)
        }
    }

    pub fn map_to_screen_pos(&self, pos: MapPosition, mult: f32) -> RenderPosition {
        let x = (pos.x as f32 - self.ulx) * mult;
        let y = (pos.y as f32 - self.uly) * mult;
        RenderPosition {
            x: x as i32,
            y: y as i32,
        }
    }
}

impl Map {
    pub fn new(width: i32, height: i32, rng: &mut pcg_mwc::Mwc256XXA64) -> Map {
        use TileType::*;

        let cells = vec![TileType::Wall; (width * height) as _];
        let blocks_movement = vec![false; cells.len()];
        let blocks_visibility = vec![false; cells.len()];
        let visible_cells = vec![false; cells.len()];

        let mut map = Map {
            width,
            height,
            cells,
            blocks_movement,
            blocks_visibility,
            visible_cells,
        };

        for x in 0..width {
            for y in 0..height {
                let idx = map.idx(x, y);
                if rng.d100() > 10 {
                    map.cells[idx] = Floor;
                }
            }
        }

        for x in 0..width {
            let idx = map.idx(x, 0);
            map.cells[idx] = Wall;
            let idx = map.idx(x, height - 1);
            map.cells[idx] = Wall;
        }

        for y in 0..height {
            let idx = map.idx(0, y);
            map.cells[idx] = Wall;
            let idx = map.idx(width - 1, y);
            map.cells[idx] = Wall;
        }

        map
    }

    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn rev_idx(&self, idx: usize) -> (i32, i32) {
        (idx as i32 % self.width, idx as i32 / self.width)
    }

    pub fn update_blocks_movement(&mut self) {
        self.cells.iter().enumerate().for_each(|(idx, typ)| {
            self.blocks_movement[idx] = match *typ {
                TileType::Wall => true,
                TileType::Floor => false,
            };
        });
    }

    pub fn update_blocks_visibility(&mut self) {
        self.cells.iter().enumerate().for_each(|(idx, typ)| {
            self.blocks_visibility[idx] = match *typ {
                TileType::Wall => true,
                TileType::Floor => false,
            };
        });
    }

    pub fn random_open_spot(&self, rng: &mut TheRng) -> MapPosition {
        // pick random x, y
        let mut x;
        let mut y;
        loop {
            x = rng.range(0..self.width);
            y = rng.range(0..self.height);
            let idx = self.idx(x, y);

            if !self.blocks_movement[idx] {
                break;
            }
        }

        MapPosition { x, y }
    }

    pub fn open_path_a_b(&self, start: usize, end: usize) -> bool {
        let path = a_star_search(start, end, self);
        path.success
    }
}

impl Algorithm2D for Map {
    fn dimensions(&self) -> Point {
        Point::new(self.width, self.height)
    }
}

impl BaseMap for Map {
    fn is_opaque(&self, idx: usize) -> bool {
        self.blocks_visibility[idx]
    }

    fn get_available_exits(&self, idx: usize) -> SmallVec<[(usize, f32); 10]> {
        let (x, y) = self.rev_idx(idx);
        let mut v = SmallVec::new();
        if x > 0 && !self.blocks_movement[self.idx(x - 1, y)] {
            v.push((self.idx(x - 1, y), 1.0));
        }
        if x + 1 < self.width && !self.blocks_movement[self.idx(x + 1, y)] {
            v.push((self.idx(x + 1, y), 1.0));
        }
        if y > 0 && !self.blocks_movement[self.idx(x, y - 1)] {
            v.push((self.idx(x, y - 1), 1.0));
        }
        if y + 1 < self.height && !self.blocks_movement[self.idx(x, y + 1)] {
            v.push((self.idx(x, y + 1), 1.0));
        }
        v
    }

    fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
        let (x1, y1) = self.rev_idx(idx1);
        let (x2, y2) = self.rev_idx(idx2);

        let manhattan_dist = (x1 - x2).abs() + (y1 - y2).abs();
        manhattan_dist as f32
    }
}
