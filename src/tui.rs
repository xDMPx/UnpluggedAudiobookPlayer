use crate::libmpv_handler::LibMpvMessage;
use ratatui::crossterm::event::{self, KeyCode};
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
    ChapterUpdate(String),
}

pub enum TuiCommand {
    Quit,
    Volume(i64),
    Seek(f64),
    PlayPause,
}

pub struct FileLoadedData {
    pub media_title: String,
    pub duration: f64,
    pub volume: i64,
    pub chapter: String,
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
    let mut chapter = String::new();
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
        let mut playback_time = playback_time.floor() as u64;
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
            "\n{chapter}\n{} {} / {} vol: {}",
            symbol,
            secs_to_hms(playback_time),
            secs_to_hms(playback_duration),
            playback_volume
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
                    playback_duration = data.duration.floor() as u64;
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
                TuiMessage::ChapterUpdate(chap) => {
                    chapter = chap;
                }
            }
        }
    }
    ratatui::restore();
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

fn secs_to_hms(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds - h * 3600) / 60;
    let s = seconds - h * 3600 - m * 60;

    format!("{h:02}:{m:02}:{s:02}")
}
