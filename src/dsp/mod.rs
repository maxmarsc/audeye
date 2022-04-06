pub mod spectrogram;
mod time_window;
pub mod waveform;

pub use spectrogram::Spectrogram;
pub use waveform::Waveform;

extern crate sndfile;
use crate::sndfile::SndFile;

pub trait DspData {
    fn new(file: SndFile) -> Self;
}