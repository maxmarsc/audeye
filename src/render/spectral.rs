use super::widgets;
use super::{draw_text_info, renderer::ChannelRenderer};
use core::panic;
extern crate sndfile;
use crate::utils::Zoom;
use fr::Image;
use std::convert::{TryFrom, TryInto};
use tui::backend::Backend;
use tui::layout::Rect;
use tui::widgets::Block;
use tui::Frame;

use crate::dsp::{AsyncDspData, AsyncDspDataState, Spectrogram, SpectrogramParameters};

use std::num::NonZeroU32;

use fast_image_resize as fr;

pub struct SpectralRenderer<'a> {
    channels: usize,
    async_renderer: AsyncDspData<Spectrogram, SpectrogramParameters>,
    resizer: fr::Resizer,
    canva_img: Option<Image<'a>>,
    max_width_resolution: usize,
}

impl<'a> SpectralRenderer<'a> {
    pub fn new(
        path: &std::path::PathBuf,
        parameters: SpectrogramParameters,
        normalize: bool,
    ) -> Self {
        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path)
            .expect("Could not open wave file");

        let channels = snd.get_channels();
        let max_res = snd.len().unwrap()
            / (parameters.window_size as f64 * (1f64 - parameters.overlap_rate)) as u64;

        SpectralRenderer {
            channels,
            async_renderer: AsyncDspData::new(path, parameters, normalize),
            // resizer: fr::Resizer::new(fr::ResizeAlg::Nearest),
            resizer: fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3)),
            canva_img: None,
            max_width_resolution: usize::try_from(max_res).unwrap(),
        }
    }
}

impl<'a> ChannelRenderer for SpectralRenderer<'a> {
    fn draw_single_channel<B: Backend>(
        &mut self,
        frame: &mut Frame<'_, B>,
        channel: usize,
        area: Rect,
        block: Block,
        zoom: &Zoom,
    ) {
        match self.async_renderer.state() {
            AsyncDspDataState::Normalizing => {
                draw_text_info(frame, area, block, "Normalizing...");
                return;
            }
            AsyncDspDataState::Created | AsyncDspDataState::Processing => {
                draw_text_info(frame, area, block, "Loading...");
                return;
            }
            AsyncDspDataState::Failed => {
                // Should crash soon
                draw_text_info(frame, area, block, "Error");
                return;
            }
            _ => {}
        }

        if channel >= self.channels {
            panic!();
        }

        let canva_width = area.width as usize;
        let canva_height = area.height as usize;
        let data_ref = match self.async_renderer.data() {
            Some(data_ref) => data_ref,
            None => panic!(),
        };

        // Create source image from spectrogram
        let num_bins = data_ref.num_bins();

        let (data_slice, num_bands) = data_ref.data(channel, zoom);
        let src_image = fr::Image::from_slice_u8(
            NonZeroU32::new(num_bins.try_into().unwrap()).unwrap(),
            NonZeroU32::new(num_bands.try_into().unwrap()).unwrap(),
            data_slice,
            fr::PixelType::U8x3,
        )
        .unwrap();

        // Compute dst images dimensions
        // /!\ The image is transposed (like a matrix) for better memory mapping /!\
        let resize_dst_width = (canva_height - 2) * 2;
        let resize_dst_height = canva_width - 2;

        // Store in option to keep it in memory for the renderingspectrogram
        self.canva_img = Some(fr::Image::new(
            NonZeroU32::new(resize_dst_width.try_into().unwrap()).unwrap(),
            NonZeroU32::new(resize_dst_height.try_into().unwrap()).unwrap(),
            fr::PixelType::U8x3,
        ));

        let canva_img_ref = self.canva_img.as_mut().unwrap();
        let mut dst_view = canva_img_ref.view_mut();

        // Resize
        self.resizer
            .resize(&src_image.view(), &mut dst_view)
            .unwrap();

        // Build Image widget
        let img_widget = widgets::Image::new(canva_img_ref.buffer()).block(block);

        frame.render_widget(img_widget, area);
    }

    fn needs_redraw(&mut self) -> bool {
        self.async_renderer.update_status()
    }

    fn max_width_resolution(&self) -> usize {
        // nasty, should rely on the same variables as the time window generator
        self.max_width_resolution
    }
}
