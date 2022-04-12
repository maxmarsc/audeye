mod renderer;
mod waveform;
mod spectral;
// mod ascii;
mod metadata;
mod greyscale_canva;
mod headers;

use tui::{Frame, backend::Backend, layout::Rect, widgets::Block};
pub use waveform::WaveformRenderer;
pub use spectral::SpectralRenderer;
pub use metadata::MetadataRenderer;
pub use renderer::{Renderer, RenderingInfo};
pub use headers::{ChannelsTabs, ZoomHead};

use renderer::{draw_loading};

pub enum RendererType<'a> {
    Waveform(WaveformRenderer),
    Spectral(SpectralRenderer<'a>),
    Metadata(MetadataRenderer)
}

impl Renderer for RendererType<'_> {
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, info: &RenderingInfo, area : Rect) {
        match self {
            RendererType::Waveform(renderer) => renderer.draw(frame, info, area),
            RendererType::Spectral(renderer) => renderer.draw(frame, info, area),
            RendererType::Metadata(renderer) => renderer.draw(frame, info, area)
        }
    }

    fn needs_redraw(&mut self) -> bool {
        match self {
            RendererType::Waveform(renderer) => renderer.needs_redraw(),
            RendererType::Spectral(renderer) => renderer.needs_redraw(),
            RendererType::Metadata(renderer) => renderer.needs_redraw()
        }
    }

    fn max_width_resolution(&self) -> usize {
        match self {
            RendererType::Waveform(renderer) => renderer.max_width_resolution(),
            RendererType::Spectral(renderer) => renderer.max_width_resolution(),
            RendererType::Metadata(renderer) => renderer.max_width_resolution()
        }
    }
}