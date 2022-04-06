use super::Renderer;
use super::greyscale_canva::TransposedGreyScaleCanva;
use super::AsyncRendererData;
use super::draw_loading;
use core::panic;
extern crate sndfile;
// use crate::dsp::AudioRepresentationData;
use crate::sndfile::{SndFileIO, SndFile};
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Receiver, TryRecvError, Sender};
use std::convert::{TryFrom, TryInto};
use fr::Image;
use rand::Fill;
use tui::Frame;
use tui::backend::Backend;
use tui::layout::{Rect, Alignment};
use tui::text::Spans;
use tui::widgets::{Paragraph, Wrap};
use tui::widgets::canvas::Painter;
use tui::{
    widgets::{
        Chart, Dataset, GraphType, Block, Borders, Axis,
        canvas::{Points, Shape, Canvas, Map, Rectangle},
    },
    symbols::Marker,
    style::{Style, Color, Modifier},
    text::Span
};
// use viuer::{print_from_file, Config};

use crate::utils::filled_rectangle::FilledRectangle;
use crate::dsp::spectrogram::Spectrogram;

use std::num::{NonZeroU32, NonZeroUsize};

use fast_image_resize as fr;

pub struct SpectralRenderer<'a> {
    pub channels : usize,
    async_renderer: AsyncRendererData<Spectrogram>,
    resizer: fr::Resizer,
    canva_img: Option<Image<'a>>
}

impl<'a> SpectralRenderer<'a> {
    pub fn new(path: &std::path::PathBuf) -> Self {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");

        let channels = snd.get_channels();
        
        SpectralRenderer {
            channels,
            async_renderer: AsyncRendererData::new(path),
            resizer: fr::Resizer::new(fr::ResizeAlg::Nearest),
            // resizer: fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3)),
            canva_img: None
        }

    }

}

impl<'a> Renderer for SpectralRenderer<'a> {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block) {
        if ! self.async_renderer.rendered() {
            // Not rendered yet
            draw_loading(frame, area, block);
            return;
        }

        if channel >= self.channels { panic!(); }

        let canva_width = area.width as usize;
        let canva_height = area.height as usize;
        let data_ref = match self.async_renderer.data() {
            Some(data_ref) => data_ref,
            None => panic!()
        };

        // Create source image from spectrogram
        let src_image = fr::Image::from_slice_u8(
            NonZeroU32::new(data_ref.num_bins().try_into().unwrap()).unwrap(),
            NonZeroU32::new(data_ref.num_bands().try_into().unwrap()).unwrap(),
            data_ref.data(channel),
            fr::PixelType::U8,
        )
        .unwrap();

        // Compute dst images dimensions
        // /!\ The image is transposed (like a matrix) for better memory mapping /!\
        let resize_dst_width = canva_height - 2;
        let resize_dst_height = canva_width - 2;

        // Store in option to keep it in memory for the rendering
        self.canva_img = Some(fr::Image::new(
            NonZeroU32::new(resize_dst_width.try_into().unwrap()).unwrap(),
            NonZeroU32::new(resize_dst_height.try_into().unwrap()).unwrap(),
            fr::PixelType::U8
        ));

        let canva_img_ref = self.canva_img.as_mut().unwrap();
        let mut dst_view = canva_img_ref.view_mut();

        // Resize
        self.resizer.resize(&src_image.view(), &mut dst_view).unwrap();

        let greyscale_canva = TransposedGreyScaleCanva::new(
            canva_img_ref.buffer(),
            resize_dst_width,
            resize_dst_height
        );

        let canva = Canvas::default()
            .block(block)
            .background_color(Color::Rgb(0, 0, 0))
            .paint(|ctx| {
                ctx.draw(&greyscale_canva)
            })
            .marker(Marker::Block)
            .x_bounds([-1., resize_dst_width as f64 - 1.0])
            .y_bounds([1.0, resize_dst_height as f64 + 1.0]);

        frame.render_widget(canva, area);
    }

    fn needs_redraw(&mut self) -> bool {
        self.async_renderer.update_status()
    }
}
