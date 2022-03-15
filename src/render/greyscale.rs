use tui::widgets::canvas::{Shape, Painter};
use tui::style::Color;
pub struct GreyScaleCanva<'a> {
    img_buffer: &'a [u8],
    width: usize,
    height: usize,
}

impl<'a> GreyScaleCanva<'a> {
    pub fn new(img_buffer : &'a [u8], width: usize, height: usize) -> GreyScaleCanva<'a> {
        GreyScaleCanva {
            img_buffer, 
            width, 
            height
        }
    }
}

impl<'a> Shape for GreyScaleCanva<'a> {
    fn draw(&self, painter: &mut Painter) {
        for x in 0..self.width {
            for y in 0..self.height {
                let idx = x + y * self.width;
                let value = self.img_buffer[idx];
                let color = Color::Rgb(value, value, value);

                // let (px, py) = painter.get_point(x as f64, (self.height - y) as f64).unwrap();
                // painter.paint(px, py, color);

                painter.paint(x, y , color);
            }
        }
    }
}