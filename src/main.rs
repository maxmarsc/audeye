
use sndfile::SndFile;
// use std::fs::File;
use structopt::StructOpt;
use hound;
use terminal_size::terminal_size;
use tui::Frame;
use tui::backend::Backend;
use tui::layout;
use tui::style::Modifier;
use tui::widgets::Clear;
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
use crate::render::ZoomHead;
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
use utils::Zoom;
use utils::bindings;
// use utils::TabsState;

mod render;
use render::Renderer;
use render::WaveformRenderer;
use render::SpectralRenderer;
use render::RendererType;
use render::ChannelsTabs;
use render::RenderingInfo;
use render::HelperPopup;
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

const WAVEFORM_TAB_IDX: usize = 0;
const SPECTRAL_TAB_IDX: usize = 1;
const METADATA_TAB_IDX: usize = 2;

struct App<'a> {
    tabs: TabsState<'a>,
    channels: ChannelsTabs,
    previous_frame: Rect,
    repaint: bool,
    zoom: Zoom,
    helper: HelperPopup
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

fn draw_zoom_head<B: Backend>(frame: &mut Frame<'_, B>, area : Rect, zoom_start: f64, zoom_len: f64) {
    let canva = Canvas::default()
            .background_color(Color::Rgb(16,16,16))
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
            .paint(|ctx| {
                ctx.draw(&Rectangle{
                    x: zoom_start,
                    y: 0f64,
                    width: zoom_len,
                    height: 1f64,
                    color: Color::White
                })})
            .x_bounds([0f64, 1f64])
            .y_bounds([0f64, 1f64]);

        frame.render_widget(canva, area);
}

fn helper_layout(area: Rect) -> Rect {
    let x_offset = area.width / 4;
    let y_offset = area.height / 4;
    
    Rect {
        x: area.x + x_offset,
        y: area.y + y_offset,
        width: area.width / 2,
        height: area.height / 2
    }
    // Rect { x: (), y: (), width: (), height: () }
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
    const TAB_SIZE: u16 = 3;

    let events = Events::new();

    // Check file info
    let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(&args.path).expect("Could not open wave file");
    let channels = snd.get_channels();
    if channels > 9usize {
        let err = Error::new(ErrorKind::InvalidInput, 
                "Audeye does not support configuration with more than 9 channels");
        return Err(err);
    }

    // Create the renderers
    let mut waveform = RendererType::Waveform(WaveformRenderer::new(&args.path));
    let mut spectral = RendererType::Spectral(SpectralRenderer::new(&args.path));
    let mut metadata_render = RendererType::Metadata(MetadataRenderer::new(&args.path));


    // Build the app
    // Compute the max zoom allowed
    let res_max = usize::min(waveform.max_width_resolution(), spectral.max_width_resolution()) as f64;

    let mut app = App {
        tabs: TabsState::new(vec!["Waveform", "Spectral", "Metadata"]),
        channels: ChannelsTabs::new(channels),
        previous_frame: Rect::default(),
        repaint: true,
        zoom: Zoom::new(terminal.size()?.width as f64 / res_max).unwrap(),
        helper: HelperPopup::default()
    };

    // let mut zoom_head = ZoomHead::new(&mut app.zoom);

    loop {
        // Get current size
        let tsize = terminal.size()?;

        let renderer = match app.tabs.index {
            WAVEFORM_TAB_IDX => &mut waveform,
            SPECTRAL_TAB_IDX => &mut spectral,
            METADATA_TAB_IDX => &mut metadata_render,
            _ => unreachable!()
        };

        if tsize != app.previous_frame {
            app.repaint = true;
            let new_max_zoom = terminal.size()?.width as f64 / res_max;
            app.zoom.update_zoom_max(new_max_zoom);
        }

        if app.repaint || renderer.needs_redraw() {
            terminal.draw(|f| {
                // Chunks settings
                let size = f.size();

                // Build rendering info structure for the renderers
                let rendering_info = RenderingInfo {
                    activated_channels: app.channels.activated(),
                    zoom: &app.zoom
                };

                
                // Setup headers and view layout
                let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(TAB_SIZE), Constraint::Min(3)])
                .split(size);
                
                let header_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33)])
                    .split(chunks[0]);
                    
                // View tabs
                draw_tabs(f, header_chunks[0], &app);
                
                // Channel tabs
                app.channels.render(f, header_chunks[2]);

                // Zoom head
                draw_zoom_head(f, header_chunks[1], app.zoom.start(), app.zoom.length());         
                
                // Renderer view drawing
                renderer.draw(f, &rendering_info, chunks[1]);

                // Helper menu
                if app.helper.is_visible() {
                    let helper_rect = helper_layout(chunks[1]);
                    f.render_widget(Clear, helper_rect);
                    app.helper.draw(f, &rendering_info, helper_rect);
                }
            })?;
        }
        
        // Reset state
        app.previous_frame = tsize;
        app.repaint = false;

        let event = events.next().unwrap();

        match event {
            Event::Input(input) => {
                match input {
                    bindings::QUIT => break,
                    bindings::NEXT_PANEL => {
                        app.tabs.next();
                        app.repaint = true;
                    },
                    bindings::PREVIOUS_PANEL => {
                        app.tabs.previous();
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_1 =>  {
                        app.channels.update(0);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_2 =>  {
                        app.channels.update(1);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_3 =>  {
                        app.channels.update(2);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_4 =>  {
                        app.channels.update(3);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_5 =>  {
                        app.channels.update(4);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_6 =>  {
                        app.channels.update(5);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_7 =>  {
                        app.channels.update(6);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_8 =>  {
                        app.channels.update(7);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_SELECTOR_9 =>  {
                        app.channels.update(8);
                        app.repaint = true;
                    },
                    bindings::CHANNEL_RESET => {
                        app.channels.reset();
                        app.repaint = true;
                    },
                    bindings::MOVE_LEFT => {
                        app.zoom.move_left();
                        app.repaint = true;
                    },
                    bindings::MOVE_RIGHT => {
                        app.zoom.move_right();
                        app.repaint = true;
                    },
                    bindings::ZOOM_OUT => {
                        app.zoom.zoom_out();
                        app.repaint = true;
                    },
                    bindings::ZOOM_IN => {
                        app.zoom.zoom_in();
                        app.repaint = true;
                    },
                    bindings::HELP => {
                        app.helper.set_visible(!app.helper.is_visible());
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
