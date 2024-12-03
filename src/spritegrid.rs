use std::{ops::Deref, marker::PhantomData, slice::Iter};

use image::{ImageBuffer, Pixel};
pub struct SpriteGrid<P, Container>
where
    P: Pixel + 'static,
    Container: Deref<Target = [P::Subpixel]>,
{
    buffer: ImageBuffer<P, Container>,
    width: u32,
    height: u32,
    num_cols: u32,
    grid_data: Vec<Vec<P>>,
}

impl <'a, P: Pixel, Container> SpriteGrid<P, Container>
where
    Container: Deref<Target = [P::Subpixel]>
{
    pub fn new(buffer: ImageBuffer<P, Container>, width: u32, height: u32, num_rows: u32, num_cols: u32) -> Self {
        let grid_data = Vec::new();
        let mut grid = SpriteGrid {
            buffer,
            width,
            height,
            grid_data,
            num_cols,
        };

        for y in 0..num_rows {
            for x in 0..num_cols {
                grid.copy_cell(x, y);
            }
        }

        grid
    }

    pub fn idx(&self, grid_x: u32, grid_y: u32) -> usize {
        (grid_y * self.num_cols + grid_x) as usize
    }

    fn copy_cell(&mut self, grid_x: u32, grid_y: u32) {
        let mut data = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let pix = self.buffer.get_pixel(grid_x * self.width + x, grid_y * self.height + y);
                data.push(*pix);
            }
        }

        self.grid_data.push(data);
    }

    pub fn enumerate_pixels(
        &self,
        grid_x: u32,
        grid_y: u32,
        flip_y: bool,
    ) -> EnumerateGridPixels<'_, P, Iter<'_, P>>
    {
        EnumerateGridPixels {
            pixels: self.grid_data[self.idx(grid_x, grid_y)].iter(),
            x: 0,
            y: 0,
            width: self.width,
            flip_y,
            _phantom: PhantomData,
        }
    }
}

pub struct EnumerateGridPixels<'a, P, I>
where
    P: Pixel + 'a,
    I: Iterator<Item = &'a P>
{
    pixels: I,
    x: u32,
    y: u32,
    width: u32,
    flip_y: bool,
    _phantom: PhantomData<&'a P>,
}

impl<'a, P: Pixel + 'a, I> Iterator for EnumerateGridPixels<'a, P, I>
where
    P::Subpixel: 'a,
    I: Iterator<Item = &'a P>,
{
    type Item = (u32, u32, &'a P);

    #[inline(always)]
    fn next(&mut self) -> Option<(u32, u32, &'a P)> {
        use num_traits::Zero;
        let pixel_zero = image::Rgb([P::Subpixel::zero(); 3]);

        if self.x >= self.width {
            self.x = 0;
            self.y += 1;
        }
        let (x, y) = (self.x, self.y);
        self.x += 1;
        let result = self.pixels.next().map(|p| match self.flip_y {
            false => (x, y, p),
            true => (self.width - x - 1, y, p),
        });
        match result {
            None => None,
            r@Some((_, _, &p)) if p.to_rgb() != pixel_zero => r,
            Some(_) => self.next()
        }
    }
}
