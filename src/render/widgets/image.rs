use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Color;
use tui::widgets::{Block, Widget};

pub struct Image<'a> {
    block: Option<Block<'a>>,
    img_buffer: &'a [u8],
}

impl<'a> Image<'a> {
    pub fn new(img_buffer: &'a [u8]) -> Image<'a> {
        Image {
            block: None,
            img_buffer,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Image<'a> {
        self.block = Some(block);
        self
    }
}

impl<'a> Widget for Image<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let img_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        self.img_buffer
            .chunks(8)
            .into_iter()
            .map(|pixels| {
                [
                    Color::Rgb(pixels[0], pixels[1], pixels[2]),
                    Color::Rgb(pixels[4], pixels[5], pixels[6]),
                ]
            })
            .enumerate()
            .for_each(|(idx, colors)| {
                let x_char = idx as u16 / img_area.height;
                let y_char = img_area.height - (idx as u16 % img_area.height) - 1;

                buf.get_mut(
                    x_char as u16 + img_area.left(),
                    y_char as u16 + img_area.top(),
                )
                .set_char('▄')
                .set_bg(colors[0])
                .set_fg(colors[1]);
            });
    }
}
