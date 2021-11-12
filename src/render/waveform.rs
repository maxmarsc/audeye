use crate::render::renderer::Renderer;
use core::panic;
use std::io::SeekFrom;
extern crate sndfile;
use crate::sndfile::{SndFileIO, SndFile};
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Receiver, TryRecvError, Sender};
use std::convert::{TryFrom, TryInto};
use tui::{
    widgets::{Chart, Dataset, GraphType, Block, Borders, Axis},
    symbols,
    style::{Style, Color, Modifier},
    text::Span
};
// use std::convert::TryInto;


struct WaveformData {
    p: Vec<Vec<(f64, f64)>>,
    n: Vec<Vec<(f64, f64)>>
}

impl Default for WaveformData {
    fn default() -> WaveformData {
        WaveformData {
            p: vec![],
            n: vec![]
        }
    }
}

impl WaveformData {
    fn reserve(&mut self, channels: usize, block_count: usize) {
        self.p.clear();
        self.n.clear();
        for ch_idx in 0..channels {
            self.p.push(vec![]);
            self.n.push(vec![]);
            self.p[ch_idx].reserve_exact(block_count);
            self.n[ch_idx].reserve_exact(block_count);
        }
    }
}

pub struct WaveformRenderer {
    rendered : bool,
    block_count : u16,
    pub channels: usize,
    data : WaveformData,
    kill_tx: Sender<bool>,
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<WaveformData>>,
}

impl WaveformRenderer {
    pub fn new(block_count: u16, path: &std::path::PathBuf) -> Self {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }
        
        let channels = snd.get_channels();
        let (kill_tx, kill_rx) = mpsc::channel();
        let (rendered_tx, rendered_rx) = mpsc::channel();
        let handle = async_compute(snd, block_count, 
            channels, kill_rx, rendered_tx);
            
        WaveformRenderer {
            rendered: false,
            block_count,
            channels,
            data: WaveformData::default(),
            kill_tx,
            rendered_rx,
            process_handle: Some(handle)
        }
    }

    fn load_results(&mut self) {
        if self.rendered {
            let opt_handle = self.process_handle.take();
            match opt_handle {
                Some(handle) => {
                    self.data = handle.join().expect("Waveform rendering failed");
                },
                None => panic!("Waveform rendering handle is None")
            }
        }
    }

    fn render<'a>(&'a self, channel: usize) -> Chart<'a> {
        let datasets = vec![
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::White))
                .graph_type(GraphType::Line)
                .data(self.data.p[channel].as_ref()),
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::White))
                .graph_type(GraphType::Line)
                .data(self.data.n[channel].as_ref()),
            // Dataset::default()
            //     .marker(symbols::Marker::Braille)
            //     .style(Style::default().fg(Color::LightRed))
            //     .graph_type(GraphType::Line)
            //     .data(zero_data.as_ref()),
        ];

        Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!["Channel {:?}", channel],
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
            ).x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, self.block_count as f64]),
            ).y_axis(
                Axis::default()
                    .title("Amplitude")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([-1.0, 1.0])
            )
    }
}

impl Drop for WaveformRenderer {
    fn drop(&mut self) {
        // We send the kill signal to the computation thread
        let _ = self.kill_tx.send(true);
    }
}

impl<'a> Renderer<'a, Chart<'a>> for WaveformRenderer {
    fn get_representation(&'a mut self, channel: usize) -> Option<Chart<'a>> {
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

        if !self.rendered || channel >= self.channels { 
            return None
        }

        Some(self.render(channel))
    }
}


fn async_compute(mut snd: SndFile, block_count: u16, channels: usize, 
        kill_rx: Receiver<bool>, rendered_tx: Sender<bool>) -> JoinHandle<WaveformData> {
    let mut data = WaveformData::default();
    let frames = snd.len().expect("Unable to retrieve number of frames");
    snd.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
    let block_size = usize::try_from(frames / u64::from(block_count))
            .expect("Block size is too big, file is probably too long");

    thread::spawn(move || {
    
        let mut block_data : Vec<i32> = vec![0; block_size*channels];
        const THRESHOLD: i32 = 0;//1024 * 128;

        data.reserve(channels, block_count as usize);

        for block_idx in 0..usize::from(block_count) {
            // Check for termination signal
            match kill_rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    return data;
                }
                Err(TryRecvError::Empty) => {}
            }

            // Read block from file
            let mut nb_frames: usize = 0;
            let read = snd.read_to_slice(block_data.as_mut_slice());
            match read {
                Ok(frames) => {
                    if frames == 0 { panic!("0 frames read")}
                    nb_frames = frames;
                },
                Err(err) => panic!("{:?}", err)
            }
    
            // Compute min & max
            let mut mins = vec![0 as i32; channels.try_into().expect("")];
            let mut maxs = vec![0 as i32; channels.try_into().expect("")];
            for frame_idx in 0..nb_frames {
                match kill_rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        println!("Terminating.");
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }

                for ch_idx in 0..channels {
                    let value = block_data[frame_idx * channels + ch_idx];
                    if value < mins[ch_idx] {
                        mins[ch_idx] = value
                    } else if value > maxs[ch_idx] {
                        maxs[ch_idx] = value;
                    }
                }
            }

            // Check for termination signal
            match kill_rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    return data;
                }
                Err(TryRecvError::Empty) => {}
            }


            for ch_idx in 0..channels {
                if mins[ch_idx] < - THRESHOLD {
                    data.n[ch_idx].push((block_idx as f64, mins[ch_idx] as f64 / i32::MAX as f64));
                }
                if maxs[ch_idx] > THRESHOLD {
                    data.p[ch_idx].push((block_idx as f64, maxs[ch_idx] as f64 / i32::MAX as f64));
                }
            }
        }

        // Send rendered signal
        let _ = rendered_tx.send(true);
        
        data
    })
}

