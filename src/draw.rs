use image::Rgb;

use super::spritegrid::SpriteGrid;

pub fn blit_sprite(
    sprite_grid: &SpriteGrid<Rgb<u8>, Vec<u8>>,
    sprite_pos_x: u32,
    sprite_pos_y: u32,
    x: i32,
    y: i32,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    flip_y: bool,
) {
    for (spritex, spritey, rgb) in sprite_grid.enumerate_pixels(sprite_pos_x, sprite_pos_y, flip_y)
    {
        let abs_y = y + spritey as i32;
        let abs_x = x + spritex as i32;
        if abs_x >= 0 && abs_x < frame_width as i32 && abs_y >= 0 && abs_y < frame_height as i32 {
            let &image::Rgb([r, g, b]) = rgb;
            let s = [r, g, b, 0xff];
            let idx = (abs_y * (frame_width as i32) * 4 + abs_x * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&s);
        }
    }
}
