use crate::render::renderer::Renderer;
use crate::render::ascii::AsciiArtConverter;
use crate::render::greyscale::GreyScaleCanva;
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

use std::num::{NonZeroU32, NonZeroUsize};

use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::{ColorType, GenericImageView, DynamicImage};
use fast_image_resize as fr;

const SPECTRAL_NUANCES: usize = 32;

struct SpectralData {

    // Ordered by [channel]
    width: NonZeroU32,
    height: NonZeroU32,
    frames: Vec<u8>
}



impl Default for SpectralData {
    fn default() -> Self {
        SpectralData{
            width: NonZeroU32::new(1).unwrap(),
            height: NonZeroU32::new(1).unwrap(),
            frames: vec![]
        }
    }
}

pub struct SpectralRenderer<'a> {
    rendered: bool,
    // block_count : 
    pub channels : usize,
    data: SpectralData,
    kill_tx: Sender<bool>,
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<SpectralData>>,
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
        let (kill_tx, kill_rx) = mpsc::channel();
        let (rendered_tx, rendered_rx) = mpsc::channel();
        let handle = async_compute(snd, channels, kill_rx, rendered_tx);
        
        SpectralRenderer {
            rendered: false,
            channels,
            data: SpectralData::default(),
            kill_tx,
            rendered_rx,
            process_handle: Some(handle),
            resizer: fr::Resizer::new(fr::ResizeAlg::Nearest),
            canva_img: None
        }

    }

    fn load_results(&mut self) {
        if self.rendered {
            let opt_handle = self.process_handle.take();
            match opt_handle {
                Some(handle) => {
                    self.data = handle.join().expect("Spectral rendering failed");
                },
                None => panic!("Spectral rendering handle is None")
            }
        }
    }

    // fn convert_img_to_spans(&self, img: &fr::Image, width: usize, height: usize) -> Vec<Spans> {
    //     let mut spans: Vec<Spans> = vec![];
    //     let buffer = img.buffer();
    //     // let height: NonZeroUsize = NonZeroUsize::try_from(img.height()).unwrap();

    //     for y in 0..height {
    //         let mut str_bytes = vec![32u8; width];

    //         for x in 0..width {
    //             let buffer_idx = x + y * width;

    //             str_bytes[x] = self.ascii_converter.convert_u8_to_ascii(buffer[buffer_idx]);
    //         }

    //         // str_bytes[0] = 65 + (u8::try_from(y).expect("") % 26u8); // A-Z as first char
    //         // str_bytes[width - 3] = 36; // $

    //         let my_string = String::from_utf8(str_bytes).expect("");

    //         spans.push(Spans::from(Span::raw(my_string)));
    //     }

    //     spans
    // }


    // fn render(&'a mut self, channel: usize, width: usize, height: usize) -> Paragraph<'a> {
    //     if ! self.rendered {
    //         panic!("render must not be called before data processing has been completed")
    //     }

    //     // Create source image from spectrogram
    //     let src_image = fr::Image::from_slice_u8(
    //         self.data.width,
    //         self.data.height,
    //         self.data.frames.as_mut_slice(),
    //         fr::PixelType::U8,
    //     )
    //     .unwrap();

    //     // Create dst image
    //     let dst_width = width - 2;
    //     let dst_height = height - 2;
    //     let mut dst_img = fr::Image::new(
    //         NonZeroU32::new(dst_width.try_into().unwrap()).unwrap(),
    //         NonZeroU32::new(dst_height.try_into().unwrap()).unwrap(),
    //         fr::PixelType::U8
    //     );
    //     let mut dst_view = dst_img.view_mut();

    //     // Resize
    //     self.resizer.resize(&src_image.view(), &mut dst_view).unwrap();



    //     let spans = self.convert_img_to_spans(&dst_img, dst_width, dst_height);

    //     Paragraph::new(spans)
    //         .block(Block::default().title(format!["Channel {:?}", channel]).borders(Borders::ALL))
    //         .alignment(Alignment::Left).wrap(Wrap { trim: true })
    // }
}

impl<'a> Drop for SpectralRenderer<'a> {
    fn drop(&mut self) {
        // We send the kill signal to the computation thread
        let _ = self.kill_tx.send(true);
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

        let width = area.width as usize;
        let height = area.height as usize;

        // frame.render_widget(self.render(channel, width, height), area);

        // Create source image from spectrogram
        let src_image = fr::Image::from_slice_u8(
            self.data.width,
            self.data.height,
            self.data.frames.as_mut_slice(),
            fr::PixelType::U8,
        )
        .unwrap();

        // Create dst image
        let dst_width = width - 2;
        let dst_height = height - 2;

        // Store in option to keep it in memory for the rendering
        self.canva_img = Some(fr::Image::new(
            NonZeroU32::new(dst_width.try_into().unwrap()).unwrap(),
            NonZeroU32::new(dst_height.try_into().unwrap()).unwrap(),
            fr::PixelType::U8
        ));

        match &mut self.canva_img {
            None => panic!(),
            Some(dst_img) => {
                let mut dst_view = dst_img.view_mut();

                // Resize
                self.resizer.resize(&src_image.view(), &mut dst_view).unwrap();
        
                let greyscale_canva = GreyScaleCanva::new(
                    dst_img.buffer(),
                    dst_width,
                    dst_height
                );
        
                let mut canva = Canvas::default()
                    .block(Block::default().title(format!["Channel {:?}", channel]).borders(Borders::ALL))
                    .background_color(Color::Rgb(0, 0, 0))
                    .paint(|ctx| {
                        ctx.draw(&greyscale_canva)
                    })
                    .marker(Marker::Block)
                    .x_bounds([-1., dst_width as f64 - 1.0])
                    .y_bounds([1.0, dst_height as f64 + 1.0]);

                frame.render_widget(canva, area);

            }
        }

    }
}

fn async_compute(mut snd: SndFile, channels: usize, kill_rx: Receiver<bool>,
        render_tx: Sender<bool>) -> JoinHandle<SpectralData> {
    let mut data = SpectralData::default();
    snd.seek(SeekFrom::Start(0)).expect("Failed to seek 0");


    
    thread::spawn(move || {
        // Read source image from test file
        let img = ImageReader::open("./cropped_spectrogram.pgm")
        .unwrap()
        .decode()
        .unwrap();


        data.width = NonZeroU32::new(img.width()).unwrap();
        data.height = NonZeroU32::new(img.height()).unwrap();
        data.frames = img.to_luma8().into_raw();

        // Send rendered signal
        let _ = render_tx.send(true);

        data
    })
}

