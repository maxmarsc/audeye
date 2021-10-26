
// use std::fs::File;
use structopt::StructOpt;
use hound;
use terminal_size::terminal_size;
use tui::layout;
use tui::style::Modifier;
use tui::widgets::Dataset;
use tui::widgets::GraphType;
use std::convert::From;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::thread::panicking;
use std::cmp::max;
extern crate sndfile;
use crate::render::waveform;
use crate::sndfile::SndFileIO;
use std::io::SeekFrom;



mod event;
use crate::event::{Event, Events};

mod render;
use crate::render::renderer::Renderer;
use crate::render::waveform::WaveformRenderer;
// use crate::util::


// use crate::util::event::{Config, Event, Events};
use std::{error::Error, io, time::Duration};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::Style,
    widgets::{
        canvas::{Canvas, Map, MapResolution, Rectangle},
        Block, Borders, Chart, Axis
    },
    text::Span,
    Terminal,
    symbols
};



#[derive(StructOpt)]
struct CliArgs {
    // The file to read
    #[structopt(parse(from_os_str), help = "The path of the file to analyze")]
    path: std::path::PathBuf,

    // Normalize option
    // unused for now
    #[structopt(short = "n", long = "normalize")]
    normalize: bool,
}


fn main() ->  Result<(), io::Error> {
    // Get some infos about the terminal
    // let (width, height) = terminal_size().expect("Unable to get terminal size");

    // Get cli args
    let args = CliArgs::from_args();

    // Setup UI
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();


    // Compute the size of each block to fit the screen
    let block_count: u16 = 1920 / 4 ;


    let mut waveform_render = WaveformRenderer::new(block_count, &args.path);
    let channels = waveform_render.channels;
    let chunk_count: u32 = max(2, channels).try_into().expect("");
    let layout_constraints = vec![Constraint::Ratio(1, chunk_count); chunk_count as usize];


    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(layout_constraints.as_ref())
                .split(f.size());
    
            for ch_idx in 0..channels {
                match waveform_render.get_representation(ch_idx) {
                    Some(chart) => f.render_widget(chart, chunks[ch_idx]),
                    None => ()
                }
            }
        
        })?;

        let event = events.next().expect("");

        match event {
            Event::Input(input) => {
                if input == Key::Char('q') {
                    break;
                }
            }
            Event::Tick => {
                continue;
            }
        }
    }


    
    

    // // let shape = Shape::Bars();
    // let graph_height: u32 = u32::max(height.0.into(), 32);
    // Chart::new_with_y_range((width.0).into(), graph_height, 0., block_count as f32, -1., 1.)
    //     .lineplot(&Shape::Lines(p_data.as_slice()))
    //     .lineplot(&Shape::Lines(n_data.as_slice()))
    //     .display();
    // Chart::new(block_count.into(), graph_height, 0., block_count as f32)
    //     .lineplot(&Shape::Lines(p_data.as_slice()))
    //     .lineplot(&Shape::Lines(n_data.as_slice()))
    //     .display();

    Ok(())
}
