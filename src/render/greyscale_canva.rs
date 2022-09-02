use tui::style::Color;
use tui::widgets::canvas::{Painter, Shape};
pub struct TransposedGreyScaleCanva<'a> {
    img_buffer: &'a [u8],
    width: usize,
    height: usize,
}

impl<'a> TransposedGreyScaleCanva<'a> {
    pub fn new(img_buffer: &'a [u8], width: usize, height: usize) -> TransposedGreyScaleCanva<'a> {
        TransposedGreyScaleCanva {
            img_buffer,
            width,
            height,
        }
    }
}

impl<'a> Shape for TransposedGreyScaleCanva<'a> {
    fn draw(&self, painter: &mut Painter) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = x + y * self.width;
                let value = self.img_buffer[idx];
                let color = Color::Rgb(value, value, value);

                // painter.paint(x, y , color);
                painter.paint(y, self.width - 1 - x, color);
            }
        }
    }
}
