use crate::tui::{FileLoadedData, TuiMessage};
use std::io::Write;

pub enum LibMpvMessage {
    Quit,
    UpdateVolume(i64),
    UpdatePosition(f64),
    PlayPause,
}

pub struct LibMpvHandler {
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

pub fn libmpv(
    path: &str,
    time: f64,
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
                        let diff = 5.0;
                        let mut pos = mpv_handler
                            .mpv
                            .get_property::<f64>("time-pos/full")
                            .unwrap_or(0.0);
                        if pos > diff {
                            pos -= diff;
                        } else {
                            pos = 0.0;
                        }
                        let mut file = std::fs::File::create(format!("{path}.txt")).unwrap();
                        file.write_all(pos.to_string().as_bytes()).unwrap();

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
                        mpv_handler
                            .mpv
                            .command("seek", &[&time.to_string(), "absolute"])
                            .unwrap();
                        let volume = mpv_handler.mpv.get_property::<i64>("volume").unwrap();
                        tui_s
                            .send(TuiMessage::FileLoaded(FileLoadedData {
                                media_title,
                                duration,
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
