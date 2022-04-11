mod renderer;
mod waveform;
mod spectral;
// mod ascii;
mod greyscale_canva;
mod headers;

use tui::{Frame, backend::Backend, layout::Rect, widgets::Block};
pub use waveform::WaveformRenderer;
pub use spectral::SpectralRenderer;
pub use renderer::Renderer;
pub use headers::ChannelsTabs;

// use renderer::AsyncRendererData;
// use crate::dsp::AsyncDspData;
use renderer::draw_loading;

pub enum RendererType<'a> {
    Waveform(WaveformRenderer),
    Spectral(SpectralRenderer<'a>)
}

impl Renderer for RendererType<'_> {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block<'_>) {
        match self {
            RendererType::Waveform(renderer) => renderer.draw(frame, channel,  area, block),
            RendererType::Spectral(renderer) => renderer.draw(frame, channel,  area, block)
        }
    }

    fn needs_redraw(&mut self) -> bool {
        match self {
            RendererType::Waveform(renderer) => renderer.needs_redraw(),
            RendererType::Spectral(renderer) => renderer.needs_redraw()
        }
    }
}