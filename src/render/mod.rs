pub mod renderer;
pub mod waveform;
pub mod spectral;
pub mod ascii;
pub mod greyscale_canva;
pub mod headers;

use tui::{Frame, backend::Backend, layout::Rect};
use waveform::WaveformRenderer;
use spectral::SpectralRenderer;
use renderer::Renderer;

pub enum RendererType<'a> {
    Waveform(WaveformRenderer),
    Spectral(SpectralRenderer<'a>)
}

impl Renderer for RendererType<'_> {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, title: &str, area : Rect) {
        match self {
            RendererType::Waveform(renderer) => renderer.draw(frame, channel, title,  area),
            RendererType::Spectral(renderer) => renderer.draw(frame, channel, title,  area)
        }
    }
}