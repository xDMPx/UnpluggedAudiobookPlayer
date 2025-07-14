use ratatui::crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal,
    widgets::{Block, Borders},
};

pub enum TuiMessage {
    StartFile,
    AudioReady,
    PlaybackPause,
    PlaybackResume,
    FileLoaded(FileLoadedData),
    VolumeUpdate(i64),
    PositionUpdate(f64),
}

pub enum LibMpvMessage {
    Quit,
    UpdateVolume(i64),
    UpdatePosition(f64),
    PlayPause,
}

pub struct FileLoadedData {
    media_title: String,
    duration: f64,
    time_pos: f64,
    volume: i64,
}

enum TuiCommand {
    Quit,
    Volume(i64),
    Seek(f64),
    PlayPause,
}

fn main() {
    let file_path = std::env::args().skip(1).next().expect("Provide file path");

    let (tui_s, tui_r) = crossbeam::channel::unbounded();
    let (libmpv_s, libmpv_r) = crossbeam::channel::unbounded();
    crossbeam::scope(move |scope| {
        scope.spawn(|_| {
            tui(libmpv_s, tui_r);
        });
        scope.spawn(move |_| {
            libmpv(&file_path, tui_s, libmpv_r);
        });
    })
    .unwrap();
}

pub fn tui(
    libmpv_s: crossbeam::channel::Sender<LibMpvMessage>,
    tui_r: crossbeam::channel::Receiver<TuiMessage>,
) {
    let commands = std::collections::HashMap::from([
        (KeyCode::Char('q'), TuiCommand::Quit),
        (KeyCode::Char('{'), TuiCommand::Volume(-1)),
        (KeyCode::Char('}'), TuiCommand::Volume(1)),
        (KeyCode::Char('['), TuiCommand::Volume(-10)),
        (KeyCode::Char(']'), TuiCommand::Volume(10)),
        (KeyCode::Left, TuiCommand::Seek(-10.0)),
        (KeyCode::Right, TuiCommand::Seek(10.0)),
        (KeyCode::Char(' '), TuiCommand::PlayPause),
    ]);

    let mut title = String::new();
    let mut terminal = ratatui::init();

    let mut playback_start = std::time::SystemTime::now();
    let mut playback_start_offset = 0.0;
    let mut playback_paused = true;
    let mut playback_ready = false;
    let mut playback_duration = 0;
    let mut playback_volume = 0;

    loop {
        let playback_time = {
            if !playback_ready {
                0.0
            } else if playback_paused {
                playback_start_offset
            } else {
                playback_start_offset + playback_start.elapsed().unwrap().as_secs_f64()
            }
        };
        let mut playback_time = playback_time.ceil() as u64;
        playback_time = playback_time.min(playback_duration);
        let symbol = {
            if !playback_ready {
                "|"
            } else if playback_paused {
                "|"
            } else {
                ">"
            }
        };
        let mut to_draw = title.clone();
        to_draw.push_str(&format!(
            "\n{} {} / {} vol: {}",
            symbol, playback_time, playback_duration, playback_volume
        ));
        draw(&mut terminal, &to_draw);

        if event::poll(std::time::Duration::from_millis(16)).unwrap() {
            let event = event::read();
            if let Ok(event) = event {
                match event {
                    event::Event::Key(key) => {
                        if let Some(command) = commands.get(&key.code) {
                            match command {
                                TuiCommand::Quit => {
                                    libmpv_s.send(LibMpvMessage::Quit).unwrap();
                                    break;
                                }
                                TuiCommand::Volume(vol) => {
                                    libmpv_s.send(LibMpvMessage::UpdateVolume(*vol)).unwrap();
                                }
                                TuiCommand::Seek(offset) => {
                                    libmpv_s
                                        .send(LibMpvMessage::UpdatePosition(*offset))
                                        .unwrap();
                                }
                                TuiCommand::PlayPause => {
                                    libmpv_s.send(LibMpvMessage::PlayPause).unwrap();
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        if let Ok(rec) = tui_r.try_recv() {
            match rec {
                TuiMessage::StartFile => {
                    playback_ready = false;
                }
                TuiMessage::AudioReady => {
                    playback_start = std::time::SystemTime::now();
                    playback_ready = true;
                    playback_paused = false;
                }
                TuiMessage::FileLoaded(data) => {
                    playback_start = std::time::SystemTime::now();
                    playback_start_offset = data.time_pos;
                    playback_duration = data.duration.ceil() as u64;
                    playback_volume = data.volume;
                    title = data.media_title;
                }
                TuiMessage::PlaybackPause => {
                    playback_start_offset += playback_start.elapsed().unwrap().as_secs_f64();
                    playback_paused = true;
                }
                TuiMessage::PlaybackResume => {
                    playback_start = std::time::SystemTime::now();
                    playback_paused = false;
                }
                TuiMessage::VolumeUpdate(vol) => {
                    playback_volume = vol;
                }
                TuiMessage::PositionUpdate(pos) => {
                    playback_start = std::time::SystemTime::now();
                    playback_start_offset = pos;
                }
            }
        }
    }
    ratatui::restore();
}

pub fn libmpv(
    path: &str,
    tui_s: crossbeam::channel::Sender<TuiMessage>,
    libmpv_r: crossbeam::channel::Receiver<LibMpvMessage>,
) {
    let mut mpv_handler = LibMpvHandler::initialize_libmpv(100).unwrap();
    mpv_handler.create_event_context().unwrap();
    mpv_handler.load_file(path).unwrap();

    loop {
        if let Some(ref mut ev_ctx) = mpv_handler.ev_ctx {
            let ev = ev_ctx
                .wait_event(0.016)
                .unwrap_or(Err(libmpv2::Error::Null));

            if let Ok(msg) = libmpv_r.try_recv() {
                match msg {
                    LibMpvMessage::Quit => {
                        mpv_handler.mpv.command("quit", &["0"]).unwrap();
                        break;
                    }
                    LibMpvMessage::UpdateVolume(vol) => {
                        let mut volume = mpv_handler.mpv.get_property::<i64>("volume").unwrap();
                        volume += vol;
                        if volume < 0 {
                            volume = 0;
                        }
                        if volume > 200 {
                            volume = 200;
                        }
                        mpv_handler.mpv.set_property("volume", volume).unwrap();
                    }
                    LibMpvMessage::UpdatePosition(offset) => {
                        mpv_handler
                            .mpv
                            .command("seek", &[&offset.to_string()])
                            .unwrap();
                    }
                    LibMpvMessage::PlayPause => {
                        mpv_handler.mpv.command("cycle", &["pause"]).unwrap();
                    }
                }
            }

            match ev {
                Ok(event) => match event {
                    libmpv2::events::Event::StartFile => {
                        tui_s.send(TuiMessage::StartFile).unwrap();
                    }
                    libmpv2::events::Event::PlaybackRestart => {
                        tui_s.send(TuiMessage::AudioReady).unwrap();
                    }
                    libmpv2::events::Event::PropertyChange {
                        name: "pause",
                        change: libmpv2::events::PropertyData::Flag(pause),
                        ..
                    } => {
                        if pause {
                            tui_s.send(TuiMessage::PlaybackPause).unwrap();
                        } else {
                            tui_s.send(TuiMessage::PlaybackResume).unwrap();
                        }
                    }
                    libmpv2::events::Event::PropertyChange {
                        name: "volume",
                        change: libmpv2::events::PropertyData::Int64(volume),
                        ..
                    } => {
                        tui_s.send(TuiMessage::VolumeUpdate(volume)).unwrap();
                    }
                    libmpv2::events::Event::Seek => {
                        let time_pos = mpv_handler
                            .mpv
                            .get_property::<f64>("time-pos/full")
                            .unwrap();
                        tui_s.send(TuiMessage::PositionUpdate(time_pos)).unwrap();
                    }
                    libmpv2::events::Event::FileLoaded => {
                        let media_title = mpv_handler
                            .mpv
                            .get_property::<libmpv2::MpvStr>("metadata/by-key/title")
                            .unwrap()
                            .to_string();
                        let duration = mpv_handler
                            .mpv
                            .get_property::<f64>("duration/full")
                            .unwrap();
                        let time_pos = mpv_handler
                            .mpv
                            .get_property::<f64>("time-pos/full")
                            .unwrap();
                        let volume = mpv_handler.mpv.get_property::<i64>("volume").unwrap();
                        tui_s
                            .send(TuiMessage::FileLoaded(FileLoadedData {
                                media_title,
                                duration,
                                time_pos,
                                volume,
                            }))
                            .unwrap();
                    }
                    _ => (),
                },
                Err(_err) => {
                    //println!("ERR: {err:?}");
                }
            }
        }
    }
}

pub fn draw(terminal: &mut DefaultTerminal, text: &str) {
    terminal
        .draw(|f| {
            let area = f.area();
            let block = Block::default().title("UAP").borders(Borders::ALL);
            let block = block.title_alignment(ratatui::layout::Alignment::Center);
            let text = ratatui::widgets::Paragraph::new(text);
            let inner = block.inner(f.area());
            f.render_widget(block, area);
            f.render_widget(text, inner);
        })
        .unwrap();
}

struct LibMpvHandler {
    mpv: libmpv2::Mpv,
    ev_ctx: Option<libmpv2::events::EventContext>,
}

impl LibMpvHandler {
    pub fn initialize_libmpv(volume: i64) -> Result<Self, libmpv2::Error> {
        let mpv = libmpv2::Mpv::new()?;
        mpv.set_property("volume", volume)?;
        mpv.set_property("vo", "null")?;

        Ok(LibMpvHandler { mpv, ev_ctx: None })
    }

    pub fn create_event_context(&mut self) -> Result<(), libmpv2::Error> {
        let ev_ctx = libmpv2::events::EventContext::new(self.mpv.ctx);
        ev_ctx.disable_deprecated_events()?;

        ev_ctx.observe_property("pause", libmpv2::Format::Flag, 0)?;
        ev_ctx.observe_property("volume", libmpv2::Format::Int64, 0)?;

        self.ev_ctx = Some(ev_ctx);

        Ok(())
    }

    pub fn load_file(&self, file: &str) -> Result<(), libmpv2::Error> {
        self.mpv
            .command("loadfile", &[format!("\"{file}\"").as_str(), "append-play"])
    }
}
