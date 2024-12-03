use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoFont},
    pixelcolor::{Rgb888, RgbColor},
    prelude::*,
    text::{Alignment, Text},
};

pub struct FrameBufferTarget {
    temp_buffer: Vec<u32>,
    width: u32,
    height: u32,
}

impl FrameBufferTarget {
    pub fn new(width: u32, height: u32) -> Self {
        let temp_buffer = vec![0; (width * height * 4) as usize];

        FrameBufferTarget {
            temp_buffer,
            width,
            height,
        }
    }

    pub fn clear(&mut self) {
        self.temp_buffer.iter_mut().for_each(|x| *x = 0);
    }

    pub fn flush(&self, out: &mut [u8]) {
        out.chunks_exact_mut(4).zip(&self.temp_buffer)
            .for_each(|(output, &input)| {
                if input != 0 {
                    output[0] = ((input >> 24) & 0xff) as u8;
                    output[1] = ((input >> 16) & 0xff) as u8;
                    output[2] = ((input >> 8) & 0xff) as u8;
                    output[3] = (input & 0xff) as u8;
                }
            });
    }
}

impl DrawTarget for FrameBufferTarget {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok(Point{x, y}) = coord.try_into() {
                let idx = (x + y * self.width as i32) as usize;
                if idx < self.temp_buffer.len() {
                    let val = (color.r() as u32) << 24 |
                        (color.g() as u32) << 16 |
                        (color.b() as u32) << 8 |
                        0xff;

                    self.temp_buffer[idx] = val;
                }
            }
        }

        Ok(())
    }
}

impl OriginDimensions for FrameBufferTarget {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

pub fn draw_text(display: &mut FrameBufferTarget, text: &str, x: i32, y: i32, color: (u8, u8, u8), alignment: Alignment, font: MonoFont) {
    /*
     // Create styles used by the drawing operations.
    let thin_stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let thick_stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 3);
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);


    let yoffset = 10;

    // Draw a 3px wide outline around the display.
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)?;

    // Draw a triangle.
    Triangle::new(
        Point::new(16, 16 + yoffset),
        Point::new(16 + 16, 16 + yoffset),
        Point::new(16 + 8, yoffset),
    )
    .into_styled(thin_stroke)
    .draw(&mut display)?;

    // Draw a filled square
    Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
        .into_styled(fill)
        .draw(&mut display)?;

    // Draw a circle with a 3px wide stroke.
    Circle::new(Point::new(88, yoffset), 17)
        .into_styled(thick_stroke)
        .draw(&mut display)?;
    */

    // Draw centered text.
    let (r,g,b) = color;
    let character_style = MonoTextStyle::new(&font, Rgb888::new(r,g,b));
    Text::with_alignment(
        text,
Point::new(x, y),
        character_style,
        alignment,
    )
    .draw(display).unwrap_or_default();
}
