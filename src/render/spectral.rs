use crate::render::renderer::Renderer;
use crate::render::ascii::AsciiArtConverter;
use crate::render::greyscale::TransposedGreyScaleCanva;
use core::panic;
use std::borrow::Cow;
use std::default;
use std::io::SeekFrom;
use std::task::Context;
extern crate sndfile;
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

use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::{ColorType, GenericImageView, DynamicImage};
use fast_image_resize as fr;

const SPECTRAL_NUANCES: usize = 32;

pub struct SpectralRenderer<'a> {
    rendered: bool,
    pub channels : usize,
    data: Option<Spectrogram>,
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<Spectrogram>>,
    resizer: fr::Resizer,
    canva_img: Option<Image<'a>>
}

impl<'a> SpectralRenderer<'a> {
    pub fn new(path: &std::path::PathBuf) -> Self {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }

        let channels = snd.get_channels();
        // let (kill_tx, _) = mpsc::channel();
        let (rendered_tx, rendered_rx) = mpsc::channel();
        let handle = async_compute(snd, rendered_tx);
        
        SpectralRenderer {
            rendered: false,
            channels,
            data: None,
            rendered_rx,
            process_handle: Some(handle),
            resizer: fr::Resizer::new(fr::ResizeAlg::Nearest),
            // resizer: fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3)),
            canva_img: None
        }

    }

    fn load_results(&mut self) {
        if self.rendered {
            let opt_handle = self.process_handle.take();
            match opt_handle {
                Some(handle) => {
                    self.data = Some(handle.join().expect("Spectral rendering failed"));
                },
                None => panic!("Spectral rendering handle is None")
            }
        }
    }
}

impl<'a> Renderer for SpectralRenderer<'a> {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area : Rect) {
        // Check for end of rendering
        if !self.rendered {
            match self.rendered_rx.try_recv() {
                Ok(true) => {
                    self.rendered = true;
                    self.load_results();
                },
                _ => ()
            }
        }

        if !self.rendered || channel >= self.channels { return; }

        let canva_width = area.width as usize;
        let canva_height = area.height as usize;
        let data_ref = match self.data.as_mut() {
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

        match &mut self.canva_img {
            None => panic!(),
            Some(dst_img) => {
                let mut dst_view = dst_img.view_mut();

                // Resize
                self.resizer.resize(&src_image.view(), &mut dst_view).unwrap();
        
                let greyscale_canva = TransposedGreyScaleCanva::new(
                    dst_img.buffer(),
                    resize_dst_width,
                    resize_dst_height
                );
        
                let canva = Canvas::default()
                    .block(Block::default().title(format!["Channel {:?}", channel]).borders(Borders::ALL))
                    .background_color(Color::Rgb(0, 0, 0))
                    .paint(|ctx| {
                        ctx.draw(&greyscale_canva)
                    })
                    .marker(Marker::Block)
                    .x_bounds([-1., resize_dst_width as f64 - 1.0])
                    .y_bounds([1.0, resize_dst_height as f64 + 1.0]);

                frame.render_widget(canva, area);

            }
        }

    }
}

fn async_compute(snd: SndFile, render_tx: Sender<bool>) -> JoinHandle<Spectrogram> {
    thread::spawn(move || {
        let data = Spectrogram::new(snd, 4096, 0.75);

        // Send rendered signal
        let _ = render_tx.send(true);

        data
    })
}

