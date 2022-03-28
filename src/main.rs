
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
use crate::render::renderer;
use crate::render::waveform;
use crate::sndfile::SndFileIO;
use std::io::SeekFrom;



mod event;
use crate::event::{Event, Events};
mod r#mod;
use crate::r#mod::{
    TabsState
};

mod render;
use crate::render::renderer::Renderer;
use crate::render::waveform::WaveformRenderer;
use crate::render::spectral::SpectralRenderer;
use crate::render::RendererType;
// use crate::util::

// mod dsp;
// use crate::dsp::spectrogram::compute_spectrogram;

mod utils;
mod dsp;

// use crate::util::event::{Config, Event, Events};
use std::{error::Error, io, time::Duration};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    // style::Color::{Yellow, Green},
    style::Style,
    widgets::{
        canvas::{Canvas, Map, MapResolution, Rectangle},
        Block, Borders, Chart, Axis, Tabs
    },
    text::{Span, Spans},
    Terminal,
    symbols,
    
};

struct App<'a> {
    tabs: TabsState<'a>,
}

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
    let backend = CrosstermBackend::new(stdout);
    // let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();


    // Compute the size of each block to fit the screen
    let block_count: usize = 1920 / 4 ;


    let waveform_render = WaveformRenderer::new(&args.path);
    // let mut spectral_render = SpectralRenderer::new(&args.path);
    // let mut waveform_render = RendererType::Waveform(WaveformRenderer::new(block_count, &args.path));
    let spectral_render = SpectralRenderer::new(&args.path);
    let channels = waveform_render.channels;
    let chunk_count: u16 = max(1, channels).try_into().expect("");
    const tab_size: u16 = 3;
    // let layout_constraints = vec![Constraint::Ratio(1, chunk_count); chunk_count as usize];
    // let titles = ["Waveform", "Spectrum"].iter().cloned()
    //     .map(Spans::from).collect();
    let mut app = App {
        tabs: TabsState::new(vec!["Waveform", "Spectral"])
    };

    let mut waveform = RendererType::Waveform(waveform_render);
    let mut spectral = RendererType::Spectral(spectral_render);

    let mut redraw_needed = true;


    loop {

        if true {
            terminal.draw(|f| {
                // Chunks settings
                let size = f.size();
                let channel_rd = u32::from(chunk_count * f.size().height);
                let channel_rn = u32::from(f.size().height - tab_size);
 
                // TODO: find a way to do it without mut
                let mut layout_constraints = vec![
                    Constraint::Ratio(channel_rn, channel_rd); (chunk_count+1) as usize
                ];
                layout_constraints[0] = Constraint::Length(tab_size);
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(layout_constraints.as_ref())
                    .split(size);
    
                // Tabs drawing
                let titles: Vec<Spans> = app.tabs.titles.iter()
                    .map(|t| {
                        let (first, rest) = t.split_at(1);
                        Spans::from(vec![
                            Span::styled(first, Style::default().fg(Color::Yellow)),
                            Span::styled(rest, Style::default().fg(Color::Green))
                        ])
                    })
                    .collect();
                let tabs = Tabs::new(titles)
                    .block(Block::default().borders(Borders::ALL).title("Tabs"))
                    .select(app.tabs.index)
                    .highlight_style(
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .bg(Color::DarkGray)
                    );
                f.render_widget(tabs, chunks[0]);
        
    
                let renderer = match app.tabs.index {
                    0 => &mut waveform,
                    1 => &mut spectral,
                    _ => unreachable!()
                };
                
    
                // Channel drawing
                for ch_idx in 0..channels {
                    renderer.draw(f, ch_idx, chunks[ch_idx + 1]);
                }
            
            })?;

            redraw_needed = false;
        }

        let event = events.next().expect("");

        match event {
            Event::Input(input) => {
                match input {
                    Key::Char('q') => break,
                    Key::Right => {
                        app.tabs.next();
                        redraw_needed = true;
                    },
                    Key::Left => {
                        app.tabs.previous();
                        redraw_needed = false;
                    },
                    _ => {}
                }

            }
            Event::Tick => {
                continue;
            },
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
