use std::io::Write;

#[derive(Debug)]
pub enum LibMpvMessage {
    Quit,
    UpdateVolume(i64),
    UpdatePosition(f64),
    PlayPause,
    NextChapter,
    PrevChapter,
}

#[derive(Debug)]
pub enum LibMpvEventMessage {
    StartFile,
    PlaybackRestart(bool),
    PlaybackPause,
    PlaybackResume,
    FileLoaded(FileLoadedData),
    VolumeUpdate(i64),
    PositionUpdate(f64),
    ChapterUpdate(String),
    Quit,
}

#[derive(Debug)]
pub struct FileLoadedData {
    pub media_title: String,
    pub artist: Option<String>,
    pub duration: f64,
    pub volume: i64,
    pub chapter: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Chapter {
    title: String,
    #[allow(dead_code)]
    time: f32,
}

pub struct LibMpvHandler {
    mpv: libmpv2::Mpv,
    ev_ctx: Option<libmpv2::events::EventContext>,
    chapters: Vec<Chapter>,
}

impl LibMpvHandler {
    pub fn initialize_libmpv(volume: i64) -> Result<Self, libmpv2::Error> {
        let mpv = libmpv2::Mpv::new()?;
        mpv.set_property("volume", volume)?;
        mpv.set_property("vo", "null")?;

        Ok(LibMpvHandler {
            mpv,
            ev_ctx: None,
            chapters: vec![],
        })
    }

    pub fn create_event_context(&mut self) -> Result<(), libmpv2::Error> {
        let ev_ctx = libmpv2::events::EventContext::new(self.mpv.ctx);
        ev_ctx.disable_deprecated_events()?;

        ev_ctx.observe_property("pause", libmpv2::Format::Flag, 0)?;
        ev_ctx.observe_property("volume", libmpv2::Format::Int64, 0)?;
        ev_ctx.observe_property("chapter", libmpv2::Format::Int64, 0)?;

        self.ev_ctx = Some(ev_ctx);

        Ok(())
    }

    pub fn load_file(&self, file: &str) -> Result<(), libmpv2::Error> {
        self.mpv
            .command("loadfile", &[format!("\"{file}\"").as_str(), "append-play"])
    }

    pub fn fech_chapters(&mut self) -> Result<(), libmpv2::Error> {
        let chapters = self.mpv.get_property::<libmpv2::MpvStr>("chapter-list")?;
        let chapters: Vec<Chapter> = serde_json::from_str(chapters.trim()).unwrap_or(vec![]);

        self.chapters = chapters;

        Ok(())
    }

    pub fn run(
        &mut self,
        path: &str,
        time: f64,
        tui_s: crossbeam::channel::Sender<LibMpvEventMessage>,
        mc_os_s: crossbeam::channel::Sender<LibMpvEventMessage>,
        libmpv_r: crossbeam::channel::Receiver<LibMpvMessage>,
    ) {
        log::debug!("LibMpv::Run Start");
        self.create_event_context().unwrap();
        self.load_file(path).unwrap();

        loop {
            if let Some(ref mut ev_ctx) = self.ev_ctx {
                let ev = ev_ctx
                    .wait_event(0.016)
                    .unwrap_or(Err(libmpv2::Error::Null));

                if let Ok(msg) = libmpv_r.try_recv() {
                    log::debug!("LibMpv::LibMpvMessage: {msg:?}");
                    match msg {
                        LibMpvMessage::Quit => {
                            mc_os_s.send(LibMpvEventMessage::Quit).unwrap();
                            let diff = 5.0;
                            let mut pos =
                                self.mpv.get_property::<f64>("time-pos/full").unwrap_or(0.0);
                            if pos > diff {
                                pos -= diff;
                            } else {
                                pos = 0.0;
                            }
                            let mut file = std::fs::File::create(format!("{path}.txt")).unwrap();
                            file.write_all(pos.to_string().as_bytes()).unwrap();

                            self.mpv.command("quit", &["0"]).unwrap();
                            break;
                        }
                        LibMpvMessage::UpdateVolume(vol) => {
                            let mut volume = self.mpv.get_property::<i64>("volume").unwrap();
                            volume += vol;
                            volume = volume.clamp(0, 200);
                            self.mpv.set_property("volume", volume).unwrap();
                        }
                        LibMpvMessage::UpdatePosition(offset) => {
                            self.mpv.command("seek", &[&offset.to_string()]).unwrap();
                        }
                        LibMpvMessage::PlayPause => {
                            self.mpv.command("cycle", &["pause"]).unwrap();
                        }
                        LibMpvMessage::PrevChapter => {
                            if self.chapters.len() > 0 {
                                let chapter = self.mpv.get_property::<i64>("chapter").unwrap() - 1;
                                if chapter >= 0 {
                                    self.mpv.set_property("chapter", chapter).unwrap();
                                }
                            }
                        }
                        LibMpvMessage::NextChapter => {
                            if self.chapters.len() > 0 {
                                let chapter = self.mpv.get_property::<i64>("chapter").unwrap() + 1;
                                if chapter < (self.chapters.len() as i64) {
                                    self.mpv.set_property("chapter", chapter).unwrap();
                                }
                            }
                        }
                    }
                }

                if ev.is_ok() {
                    log::debug!("LibMpv::Event {ev:?}");
                }
                match ev {
                    Ok(event) => match event {
                        libmpv2::events::Event::StartFile => {
                            tui_s.send(LibMpvEventMessage::StartFile).unwrap();
                            mc_os_s.send(LibMpvEventMessage::StartFile).unwrap();
                        }
                        libmpv2::events::Event::PlaybackRestart => {
                            let pause = self.mpv.get_property::<bool>("pause").unwrap();
                            tui_s
                                .send(LibMpvEventMessage::PlaybackRestart(pause))
                                .unwrap();
                            mc_os_s
                                .send(LibMpvEventMessage::PlaybackRestart(pause))
                                .unwrap();
                        }
                        libmpv2::events::Event::PropertyChange {
                            name: "pause",
                            change: libmpv2::events::PropertyData::Flag(pause),
                            ..
                        } => {
                            if pause {
                                tui_s.send(LibMpvEventMessage::PlaybackPause).unwrap();
                                mc_os_s.send(LibMpvEventMessage::PlaybackPause).unwrap();
                            } else {
                                tui_s.send(LibMpvEventMessage::PlaybackResume).unwrap();
                                mc_os_s.send(LibMpvEventMessage::PlaybackResume).unwrap();
                            }
                        }
                        libmpv2::events::Event::PropertyChange {
                            name: "volume",
                            change: libmpv2::events::PropertyData::Int64(volume),
                            ..
                        } => {
                            tui_s
                                .send(LibMpvEventMessage::VolumeUpdate(volume))
                                .unwrap();
                            mc_os_s
                                .send(LibMpvEventMessage::VolumeUpdate(volume))
                                .unwrap();
                        }
                        libmpv2::events::Event::PropertyChange {
                            name: "chapter",
                            change: libmpv2::events::PropertyData::Int64(i),
                            ..
                        } => {
                            if i >= 0 {
                                let chapter = self.chapters.get(i as usize).unwrap().title.clone();
                                tui_s
                                    .send(LibMpvEventMessage::ChapterUpdate(chapter.clone()))
                                    .unwrap();
                                mc_os_s
                                    .send(LibMpvEventMessage::ChapterUpdate(chapter))
                                    .unwrap();
                            }
                        }
                        libmpv2::events::Event::Seek => {
                            let time_pos = self.mpv.get_property::<f64>("time-pos/full").unwrap();
                            tui_s
                                .send(LibMpvEventMessage::PositionUpdate(time_pos))
                                .unwrap();
                            mc_os_s
                                .send(LibMpvEventMessage::PositionUpdate(time_pos))
                                .unwrap();
                        }
                        libmpv2::events::Event::FileLoaded => {
                            let media_title = self
                                .mpv
                                .get_property::<libmpv2::MpvStr>("metadata/by-key/title")
                                .unwrap()
                                .to_string();
                            let duration = self.mpv.get_property::<f64>("duration/full").unwrap();
                            self.mpv
                                .command("seek", &[&time.to_string(), "absolute"])
                                .unwrap();
                            self.fech_chapters().unwrap();
                            let chapter = {
                                if self.chapters.len() > 0 {
                                    let chapter = self.mpv.get_property::<i64>("chapter").unwrap();
                                    Some(self.chapters.get(chapter as usize).unwrap().title.clone())
                                } else {
                                    None
                                }
                            };
                            let volume = self.mpv.get_property::<i64>("volume").unwrap();
                            let artist = self
                                .mpv
                                .get_property::<libmpv2::MpvStr>("metadata/by-key/artist")
                                .map(|s| Some(s.to_string()))
                                .unwrap_or_else(|_| None);
                            tui_s
                                .send(LibMpvEventMessage::FileLoaded(FileLoadedData {
                                    media_title: media_title.clone(),
                                    artist: artist.clone(),
                                    duration,
                                    volume,
                                    chapter: chapter.clone(),
                                }))
                                .unwrap();
                            mc_os_s
                                .send(LibMpvEventMessage::FileLoaded(FileLoadedData {
                                    media_title,
                                    artist,
                                    duration,
                                    volume,
                                    chapter: chapter,
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
        log::debug!("LibMpv::Run END");
    }
}
