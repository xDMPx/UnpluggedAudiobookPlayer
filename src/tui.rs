use crate::libmpv_handler::{LibMpvEventMessage, LibMpvMessage};
use ratatui::crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    widgets::{Block, Borders},
};

#[derive(Debug)]
pub enum TuiCommand {
    Quit,
    Volume(i64),
    Seek(f64),
    PlayPause,
    NextChapter,
    PrevChapter,
}

pub fn tui(
    libmpv_s: crossbeam::channel::Sender<LibMpvMessage>,
    tui_r: crossbeam::channel::Receiver<LibMpvEventMessage>,
) {
    log::debug!("Tui::Start");
    let commands = std::collections::HashMap::from([
        (
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            TuiCommand::Quit,
        ),
        (
            KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
            TuiCommand::Volume(-1),
        ),
        (
            KeyEvent::new(KeyCode::Char('}'), KeyModifiers::NONE),
            TuiCommand::Volume(1),
        ),
        (
            KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
            TuiCommand::Volume(-10),
        ),
        (
            KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
            TuiCommand::Volume(10),
        ),
        (
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            TuiCommand::Seek(-10.0),
        ),
        (
            KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT),
            TuiCommand::Seek(-60.0),
        ),
        (
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            TuiCommand::Seek(10.0),
        ),
        (
            KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT),
            TuiCommand::Seek(60.0),
        ),
        (
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
            TuiCommand::PrevChapter,
        ),
        (
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
            TuiCommand::NextChapter,
        ),
        (
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
            TuiCommand::PlayPause,
        ),
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
            if !playback_ready || playback_paused {
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
                log::debug!("Tui::Event: {event:?}");
                if let event::Event::Key(key) = event {
                    if let Some(command) = commands.get(&key) {
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
                            TuiCommand::PrevChapter => {
                                libmpv_s.send(LibMpvMessage::PrevChapter).unwrap();
                            }
                            TuiCommand::NextChapter => {
                                libmpv_s.send(LibMpvMessage::NextChapter).unwrap();
                            }
                        }
                    }
                }
            }
        }
        if let Ok(rec) = tui_r.try_recv() {
            log::debug!("Tui::LibMpvEventMessage: {rec:?}");
            match rec {
                LibMpvEventMessage::StartFile => {
                    playback_ready = false;
                }
                LibMpvEventMessage::PlaybackRestart(paused) => {
                    playback_start = std::time::SystemTime::now();
                    playback_ready = true;
                    playback_paused = paused;
                }
                LibMpvEventMessage::FileLoaded(data) => {
                    playback_start = std::time::SystemTime::now();
                    playback_duration = data.duration.floor() as u64;
                    playback_volume = data.volume;
                    title = data.media_title;
                }
                LibMpvEventMessage::PlaybackPause => {
                    playback_start_offset += playback_start.elapsed().unwrap().as_secs_f64();
                    playback_paused = true;
                }
                LibMpvEventMessage::PlaybackResume => {
                    playback_start = std::time::SystemTime::now();
                    playback_paused = false;
                }
                LibMpvEventMessage::VolumeUpdate(vol) => {
                    playback_volume = vol;
                }
                LibMpvEventMessage::PositionUpdate(pos) => {
                    playback_start = std::time::SystemTime::now();
                    playback_start_offset = pos;
                }
                LibMpvEventMessage::ChapterUpdate(chap) => {
                    chapter = chap;
                }
                LibMpvEventMessage::Quit => (),
            }
        }
    }
    ratatui::restore();
    log::debug!("Tui::End");
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
