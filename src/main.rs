
// use std::fs::File;
use structopt::StructOpt;
use hound;
use terminal_size::terminal_size;
use tui::Frame;
use tui::backend::Backend;
use tui::layout;
use tui::style::Modifier;
use tui::widgets::Dataset;
use tui::widgets::GraphType;
use std::convert::From;
use std::convert::TryFrom;
use std::convert::TryInto;
// use std::ptr::metadata;
use std::thread::panicking;
use std::cmp::max;
extern crate sndfile;
use crate::render::MetadataRenderer;
// use crate::render::renderer;
// use crate::render::waveform;
use crate::sndfile::SndFileIO;
use std::io::{Error, ErrorKind};



mod event;
use crate::event::{Event, Events};
mod r#mod;
use crate::r#mod::{
    TabsState
};

mod utils;
// use utils::TabsState;

mod render;
use render::Renderer;
use render::WaveformRenderer;
use render::SpectralRenderer;
use render::RendererType;
use render::ChannelsTabs;
use render::RenderingInfo;
// use crate::util::

// mod dsp;
// use crate::dsp::spectrogram::compute_spectrogram;

mod dsp;

// use crate::util::event::{Config, Event, Events};
use std::{io, time::Duration};
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
    channels: ChannelsTabs,
    previous_frame: Rect,
    repaint: bool
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

fn draw_tabs<B: Backend>(frame: &mut Frame<'_, B>, area : Rect, app: &App) {
    // Tabs drawing
    let tab_titles: Vec<Spans> = app.tabs.titles.iter()
        .map(|t| {
            let (first, rest) = t.split_at(1);
            Spans::from(vec![
                Span::styled(first, Style::default().fg(Color::Yellow)),
                Span::styled(rest, Style::default().fg(Color::Green))
            ])
        })
        .collect();
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
        .title("Tabs"))
        .select(app.tabs.index)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray)
        );

    frame.render_widget(tabs, area);
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
    let spectral_render = SpectralRenderer::new(&args.path);
    let mut metadata_render = RendererType::Metadata(MetadataRenderer::new(&args.path));
    let channels = waveform_render.channels;
    if channels > 9usize {
        let err = Error::new(ErrorKind::InvalidInput, 
                "Audeye does not support configuration with more than 9 channels");
        return Err(err);
    }

    const tab_size: u16 = 3;
    let mut app = App {
        tabs: TabsState::new(vec!["Waveform", "Spectral", "Metadata"]),
        channels: ChannelsTabs::new(channels),
        previous_frame: Rect::default(),
        repaint: true
    };

    let mut waveform = RendererType::Waveform(waveform_render);
    let mut spectral = RendererType::Spectral(spectral_render);


    loop {
        // Get current size
        let tsize = terminal.size()?;

        let renderer = match app.tabs.index {
            0 => &mut waveform,
            1 => &mut spectral,
            2 => &mut metadata_render,
            _ => unreachable!()
        };

        if app.repaint || renderer.needs_redraw() || (tsize != app.previous_frame) {
            terminal.draw(|f| {
                // Chunks settings
                let size = f.size();

                // Get activated channels and setup their layout
                // let activated_channels = app.channels.activated();
                let rendering_info = RenderingInfo {
                    activated_channels: app.channels.activated()
                };
    
                // TODO: find a way to do it without mut
                let layout_constraints = vec![
                    Constraint::Length(3), Constraint::Min(3)
                ];
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(layout_constraints.as_ref())
                    .split(size);

                let header_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[0]);

                // View tabs
                draw_tabs(f, header_chunks[0], &app);

                // Channel tabs
                app.channels.render(f, header_chunks[1]);
        
                // Audio data drawing
                renderer.draw(f, &rendering_info, chunks[1]);
            })?;
        }
        
        // Reset state
        app.previous_frame = tsize;
        app.repaint = false;

        let event = events.next().unwrap();

        match event {
            Event::Input(input) => {
                match input {
                    Key::Char('q') => break,
                    Key::Right => {
                        app.tabs.next();
                        app.repaint = true;
                    },
                    Key::Left => {
                        app.tabs.previous();
                        app.repaint = true;
                    },
                    Key::Char('1') =>  {
                        app.channels.update(0);
                        app.repaint = true;
                    },
                    Key::Char('2') =>  {
                        app.channels.update(1);
                        app.repaint = true;
                    },
                    Key::Char('3') =>  {
                        app.channels.update(2);
                        app.repaint = true;
                    },
                    Key::Char('4') =>  {
                        app.channels.update(3);
                        app.repaint = true;
                    },
                    Key::Char('5') =>  {
                        app.channels.update(4);
                        app.repaint = true;
                    },
                    Key::Char('6') =>  {
                        app.channels.update(5);
                        app.repaint = true;
                    },
                    Key::Char('7') =>  {
                        app.channels.update(6);
                        app.repaint = true;
                    },
                    Key::Char('8') =>  {
                        app.channels.update(7);
                        app.repaint = true;
                    },
                    Key::Char('9') =>  {
                        app.channels.update(8);
                        app.repaint = true;
                    },
                    Key::Esc => {
                        app.channels.reset();
                        app.repaint = true;
                    }
                    _ => {}
                }

            }
            Event::Tick => {
                continue;
            },
        }
    }

    Ok(())
}
