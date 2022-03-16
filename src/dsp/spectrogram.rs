extern crate sndfile;
use crate::sndfile::SndFile;

use std::convert::{TryFrom, TryInto};

// use super::compute_spectrogram;
use super::time_window::Batcher;

pub fn compute_spectrogram(mut sndfile: SndFile, twindow_size: u32, overlap_size: f64) -> Vec<Vec<u8>> {
    let frames = sndfile.len().unwrap();
    let channels = sndfile.get_channels();
    let mut window_batcher = Batcher::new(sndfile, twindow_size.try_into().unwrap(), overlap_size);
    let fband_size = u64::from(twindow_size / 2);
    let num_band = window_batcher.get_num_bands();

    // Allocate the memory for the u8 spectrograms
    let mut spectrograms = vec![vec![0 as u8; usize::try_from(num_band * fband_size).unwrap()]; channels];




    return spectrograms;
}
