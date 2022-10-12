mod renderer;
mod spectral;
mod waveform;
// mod ascii;
mod headers;
mod help;
mod metadata;
mod widgets;

pub use headers::ChannelsTabs;
pub use help::HelperPopup;
pub use metadata::MetadataRenderer;
pub use renderer::{Renderer, RenderingInfo};
pub use spectral::SpectralRenderer;
use tui::{backend::Backend, layout::Rect, Frame};
pub use waveform::WaveformRenderer;

use renderer::draw_text_info;

pub enum RendererType<'a> {
    Waveform(WaveformRenderer),
    Spectral(SpectralRenderer<'a>),
    Metadata(Box<MetadataRenderer>),
}

impl Renderer for RendererType<'_> {
    fn draw<B: Backend>(&mut self, frame: &mut Frame<'_, B>, info: &RenderingInfo, area: Rect) {
        match self {
            RendererType::Waveform(renderer) => renderer.draw(frame, info, area),
            RendererType::Spectral(renderer) => renderer.draw(frame, info, area),
            RendererType::Metadata(renderer) => renderer.draw(frame, info, area),
        }
    }

    fn needs_redraw(&mut self) -> bool {
        match self {
            RendererType::Waveform(renderer) => renderer.needs_redraw(),
            RendererType::Spectral(renderer) => renderer.needs_redraw(),
            RendererType::Metadata(renderer) => renderer.needs_redraw(),
        }
    }

    fn max_width_resolution(&self) -> usize {
        match self {
            RendererType::Waveform(renderer) => renderer.max_width_resolution(),
            RendererType::Spectral(renderer) => renderer.max_width_resolution(),
            RendererType::Metadata(renderer) => renderer.max_width_resolution(),
        }
    }
}
