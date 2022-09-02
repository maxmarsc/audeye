extern crate sndfile;
use sndfile::SndFileIO;

use crate::sndfile::SndFile;
use std::io::SeekFrom;

use std::convert::TryFrom;

use rayon::prelude::*;

use super::{DspData, DspErr};
use crate::utils::Zoom;

fn compute_point(frames: &[i32]) -> WaveformPoint<i32> {
    let mut min = 0i32;
    let mut max = 0i32;
    let mut sum = 0f64;

    for elm in frames {
        sum += (*elm as f64) * (*elm as f64);
        if *elm > max {
            max = *elm;
        } else if *elm < min {
            min = *elm;
        }
    }

    WaveformPoint {
        rms: (sum / frames.len() as f64).sqrt() as i32,
        peak_min: min,
        peak_max: max,
    }
}

pub struct Waveform {
    frames: Vec<Vec<i32>>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct WaveformPoint<T> {
    pub rms: T,
    pub peak_min: T,
    pub peak_max: T,
}

#[derive(Default)]
pub struct WaveformParameters;

impl DspData<WaveformParameters> for Waveform {
    fn new(
        mut sndfile: SndFile,
        _: WaveformParameters,
        norm: Option<f64>,
    ) -> Result<Waveform, DspErr> {
        // Compute block size
        let frames = sndfile.len().expect("Unable to retrieve number of frames");
        sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
        let block_size = 4096usize;
        let block_count = if frames % block_size as u64 == 0 {
            usize::try_from(frames / block_size as u64).unwrap()
        } else {
            usize::try_from(frames / block_size as u64 + 1).unwrap()
        };
        let channels = sndfile.get_channels();

        // Create data vectors
        let mut data = Waveform {
            frames: vec![vec![0i32; usize::try_from(frames).unwrap()]; channels],
        };
        let mut block_data: Vec<i32> = vec![0; block_size * channels];

        // Find min and max for each block
        for block_idx in 0..block_count {
            // Read block from file
            // let mut nb_frames: usize = 0;
            let read = sndfile.read_to_slice(block_data.as_mut_slice());
            let nb_frames = match read {
                Ok(frames) => {
                    if frames == 0 {
                        panic!("0 frames read")
                    }
                    frames
                }
                Err(err) => panic!("{:?}", err),
            };

            // Load into frames vector
            let frame_offset = block_idx * block_size;
            {
                let interleaved_slice = &block_data[..nb_frames * channels];
                // let mono_slices = data.frames.iter_mut().map(|channel| {
                //     channel.as_mut_slice()[frame_idx..frame_idx + interleaved_slice.len()]
                // }).collect();

                // We could use dynamic dispatch to automatically switch btw
                // the different evaluation method (norm / no-norm) but it would
                // surely slow it down.
                // TODO: benchmark ?
                match norm {
                    Some(fnorm) => {
                        let fnorm_inv = 1f64 / fnorm;

                        interleaved_slice
                            .chunks_exact(channels)
                            .enumerate()
                            .for_each(|(frame_idx, samples)| {
                                for (channel, value) in samples.iter().enumerate() {
                                    data.frames[channel][frame_offset + frame_idx] =
                                        (*value as f64 * fnorm_inv) as i32;
                                }
                            });
                    }
                    None => {
                        interleaved_slice
                            .chunks_exact(channels)
                            .enumerate()
                            .for_each(|(frame_idx, samples)| {
                                for (channel, value) in samples.iter().enumerate() {
                                    data.frames[channel][frame_offset + frame_idx] = *value;
                                }
                            });
                    }
                }
            }
        }

        Ok(data)
    }
}

impl Waveform {
    pub fn compute_points(
        &self,
        channel: usize,
        block_count: usize,
        zoom: &Zoom,
    ) -> Vec<WaveformPoint<i32>> {
        // Alloc vectors
        let mut points = vec![WaveformPoint::default(); block_count];

        // Compute block size and count
        let total_frames = self.frames[0].len();
        let start = (total_frames as f64 * zoom.start()) as usize;
        let end = (total_frames as f64 * (zoom.start() + zoom.length())) as usize;
        let rendered_frames = end - start;
        let block_size = if rendered_frames % block_count == 0 {
            rendered_frames / block_count
        } else {
            rendered_frames / block_count + 1
        };

        let samples_chunks = self.frames[channel][start..end].par_chunks_exact(block_size);
        let remains = samples_chunks.remainder();

        points[..block_count]
            .par_iter_mut()
            .zip(samples_chunks.into_par_iter())
            .for_each(|(point, chunk)| {
                *point = compute_point(chunk);
            });

        if !remains.is_empty() {
            // Consume the end
            points[block_count - 1] = compute_point(remains);
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use crate::dsp::{AsyncDspData, AsyncDspDataState, DspData, Waveform, WaveformParameters};
    use crate::Zoom;
    use sndfile;
    use std::path::{Path, PathBuf};
    use std::thread::sleep;
    use std::time::Duration;

    fn get_test_files_location() -> PathBuf {
        return Path::new(&env!("CARGO_MANIFEST_DIR").to_string())
            .join("tests")
            .join("files");
    }

    #[test]
    fn build() {
        for norm in [None, Some(1.1f64)] {
            let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
                .from_path(get_test_files_location().join("rock_1s.wav"))
                .unwrap();

            Waveform::new(snd, WaveformParameters::default(), norm).unwrap();
        }
    }

    #[test]
    fn async_build() {
        let sleep_interval = Duration::new(1, 0);
        let path = get_test_files_location().join("rock_1s.wav");

        let mut async_data: AsyncDspData<Waveform, WaveformParameters> =
            AsyncDspData::new(&path, WaveformParameters::default(), false);
        let mut attempts = 0;

        loop {
            sleep(sleep_interval);
            async_data.update_status();
            let state = async_data.state();

            assert_ne!(state, AsyncDspDataState::Failed);
            assert!(attempts < 30);

            if state == AsyncDspDataState::Finished {
                break;
            }
            attempts += 1;
        }
    }

    #[test]
    fn async_build_normalize() {
        let sleep_interval = Duration::new(1, 0);
        let path = get_test_files_location().join("rock_1s.wav");

        let mut async_data: AsyncDspData<Waveform, WaveformParameters> =
            AsyncDspData::new(&path, WaveformParameters::default(), true);
        let mut attempts = 0;

        loop {
            sleep(sleep_interval);
            async_data.update_status();
            let state = async_data.state();

            assert_ne!(state, AsyncDspDataState::Failed);
            assert!(attempts < 30);

            if state == AsyncDspDataState::Finished {
                break;
            }
            attempts += 1;
        }
    }

    #[test]
    fn compute_points() {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav"))
            .unwrap();
        let channels = snd.get_channels();
        let mut zoom = Zoom::new(0.5f64).unwrap();
        let blocks_counts = [50usize, 100, 128, 150, 1024];

        let waveform = Waveform::new(snd, WaveformParameters::default(), None).unwrap();

        // No zoom
        for ch_idx in 0..channels {
            for block_count in blocks_counts {
                let points = waveform.compute_points(ch_idx, block_count, &zoom);

                assert_eq!(block_count, points.len());
            }
        }

        // Zoom in and move
        for _ in 0..10 {
            zoom.zoom_in();
            zoom.move_right();

            for ch_idx in 0..channels {
                for block_count in blocks_counts {
                    let points = waveform.compute_points(ch_idx, block_count, &zoom);

                    assert_eq!(block_count, points.len());
                }
            }
        }
    }
}
