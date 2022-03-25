pub mod renderer;
pub mod waveform;
pub mod spectral;
pub mod ascii;
pub mod greyscale_canva;

use tui::{Frame, backend::Backend, layout::Rect};
use waveform::WaveformRenderer;
use spectral::SpectralRenderer;
use renderer::Renderer;

pub enum RendererType<'a> {
    Waveform(WaveformRenderer),
    Spectral(SpectralRenderer<'a>)
}

impl Renderer for RendererType<'_> {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area : Rect) {
        match self {
            RendererType::Waveform(renderer) => renderer.draw(frame, channel, area),
            RendererType::Spectral(renderer) => renderer.draw(frame, channel, area)
        }
    }
}