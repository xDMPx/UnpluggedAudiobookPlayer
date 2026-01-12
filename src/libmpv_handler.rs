use crate::UAPlayerError;
use std::io::Write;

#[derive(Debug)]
pub enum LibMpvMessage {
    Quit,
    UpdateVolume(i64),
    SetVolume(i64),
    UpdatePosition(f64),
    SetPosition(f64),
    Resume,
    Pause,
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
    pub album: Option<String>,
    pub duration: f64,
    pub volume: i64,
    pub chapter: Option<String>,
    pub chapters: Vec<Chapter>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Chapter {
    pub title: String,
    #[allow(dead_code)]
    time: f32,
}

pub struct LibMpvHandler {
    mpv: libmpv2::Mpv,
    chapters: Vec<Chapter>,
}

impl LibMpvHandler {
    pub fn initialize_libmpv(volume: i64) -> Result<Self, libmpv2::Error> {
        let mpv = libmpv2::Mpv::new()?;
        mpv.set_property("volume", volume)?;
        mpv.set_property("vo", "null")?;

        Ok(LibMpvHandler {
            mpv,
            chapters: vec![],
        })
    }

    pub fn create_client(&self) -> Result<libmpv2::Mpv, libmpv2::Error> {
        let client = self.mpv.create_client(None)?;
        client.disable_deprecated_events()?;

        client.observe_property("pause", libmpv2::Format::Flag, 0)?;
        client.observe_property("volume", libmpv2::Format::Int64, 0)?;
        client.observe_property("chapter", libmpv2::Format::Int64, 0)?;

        Ok(client)
    }

    pub fn load_file(&self, file: &str) -> Result<(), libmpv2::Error> {
        self.mpv
            .command("loadfile", &[format!("{file}").as_str(), "append-play"])
    }

    pub fn fech_chapters(&mut self) -> Result<(), libmpv2::Error> {
        let chapters = self.mpv.get_property::<libmpv2::MpvStr>("chapter-list")?;
        let chapters: Vec<Chapter> = serde_json::from_str(chapters.trim()).unwrap_or(vec![]);

        self.chapters = chapters;

        Ok(())
    }

    pub fn run(
        &mut self,
        mut mpv_client: libmpv2::Mpv,
        path: &str,
        time: f64,
        tui_s: crossbeam::channel::Sender<LibMpvEventMessage>,
        mc_os_s: crossbeam::channel::Sender<LibMpvEventMessage>,
        libmpv_r: crossbeam::channel::Receiver<LibMpvMessage>,
    ) -> Result<(), UAPlayerError> {
        self.load_file(path)?;

        loop {
            let ev = mpv_client
                .wait_event(0.016)
                .unwrap_or(Err(libmpv2::Error::Null));

            if ev.is_ok() {
                log::debug!("Event {ev:?}");
            }
            match ev {
                Ok(event) => match event {
                    libmpv2::events::Event::StartFile => {
                        tui_s.send(LibMpvEventMessage::StartFile)?;
                        mc_os_s.send(LibMpvEventMessage::StartFile)?;
                    }
                    libmpv2::events::Event::PlaybackRestart => {
                        let pause = self.mpv.get_property::<bool>("pause")?;
                        tui_s.send(LibMpvEventMessage::PlaybackRestart(pause))?;
                        mc_os_s.send(LibMpvEventMessage::PlaybackRestart(pause))?;
                    }
                    libmpv2::events::Event::PropertyChange {
                        name: "pause",
                        change: libmpv2::events::PropertyData::Flag(pause),
                        ..
                    } => {
                        if pause {
                            tui_s.send(LibMpvEventMessage::PlaybackPause)?;
                            mc_os_s.send(LibMpvEventMessage::PlaybackPause)?;
                        } else {
                            tui_s.send(LibMpvEventMessage::PlaybackResume)?;
                            mc_os_s.send(LibMpvEventMessage::PlaybackResume)?;
                        }
                    }
                    libmpv2::events::Event::PropertyChange {
                        name: "volume",
                        change: libmpv2::events::PropertyData::Int64(volume),
                        ..
                    } => {
                        tui_s.send(LibMpvEventMessage::VolumeUpdate(volume))?;
                        mc_os_s.send(LibMpvEventMessage::VolumeUpdate(volume))?;
                    }
                    libmpv2::events::Event::PropertyChange {
                        name: "chapter",
                        change: libmpv2::events::PropertyData::Int64(i),
                        ..
                    } => {
                        if i >= 0 {
                            if let Some(chapter) = self.chapters.get(i as usize) {
                                let chapter = chapter.title.clone();
                                tui_s.send(LibMpvEventMessage::ChapterUpdate(chapter.clone()))?;
                                mc_os_s.send(LibMpvEventMessage::ChapterUpdate(chapter))?;
                            }
                        }
                    }
                    libmpv2::events::Event::Seek => {
                        let time_pos = self.mpv.get_property::<f64>("time-pos/full")?;
                        tui_s.send(LibMpvEventMessage::PositionUpdate(time_pos))?;
                        mc_os_s.send(LibMpvEventMessage::PositionUpdate(time_pos))?;
                    }
                    libmpv2::events::Event::FileLoaded => {
                        let media_title = self
                            .mpv
                            .get_property::<libmpv2::MpvStr>("metadata/by-key/title")?
                            .to_string();
                        let duration = self.mpv.get_property::<f64>("duration/full")?;
                        self.mpv.command("seek", &[&time.to_string(), "absolute"])?;
                        self.fech_chapters()?;
                        let chapter = {
                            if self.chapters.len() > 0 {
                                let chapter = self.mpv.get_property::<i64>("chapter")?;
                                if let Some(chapter) = self.chapters.get(chapter as usize) {
                                    Some(chapter.title.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };
                        let volume = self.mpv.get_property::<i64>("volume")?;
                        let artist = self
                            .mpv
                            .get_property::<libmpv2::MpvStr>("metadata/by-key/artist")
                            .map(|s| Some(s.to_string()))
                            .unwrap_or_else(|_| None);
                        let album = self
                            .mpv
                            .get_property::<libmpv2::MpvStr>("metadata/by-key/album")
                            .map(|s| Some(s.to_string()))
                            .unwrap_or_else(|_| None);

                        tui_s.send(LibMpvEventMessage::FileLoaded(FileLoadedData {
                            media_title: media_title.clone(),
                            artist: artist.clone(),
                            album: album.clone(),
                            duration,
                            volume,
                            chapter: chapter.clone(),
                            chapters: self.chapters.clone(),
                        }))?;
                        mc_os_s.send(LibMpvEventMessage::FileLoaded(FileLoadedData {
                            media_title,
                            artist,
                            album,
                            duration,
                            volume,
                            chapter,
                            chapters: vec![],
                        }))?;
                    }
                    _ => (),
                },
                Err(_err) => {
                    //println!("ERR: {err:?}");
                }
            }

            if let Ok(msg) = libmpv_r.try_recv() {
                log::debug!("LibMpv::LibMpvMessage: {msg:?}");
                match msg {
                    LibMpvMessage::Quit => {
                        mc_os_s.send(LibMpvEventMessage::Quit)?;
                        let diff = 5.0;
                        let mut pos = self.mpv.get_property::<f64>("time-pos/full").unwrap_or(0.0);
                        if pos > diff {
                            pos -= diff;
                        } else {
                            pos = 0.0;
                        }
                        let mut file = std::fs::File::create(format!("{path}.txt"))?;
                        file.write_all(pos.to_string().as_bytes())?;

                        self.mpv.command("quit", &["0"])?;
                        break;
                    }
                    LibMpvMessage::UpdateVolume(vol) => {
                        let mut volume = self.mpv.get_property::<i64>("volume")?;
                        volume += vol;
                        volume = volume.clamp(0, 200);
                        self.mpv.set_property("volume", volume)?;
                    }
                    LibMpvMessage::SetVolume(vol) => {
                        let volume = vol.clamp(0, 200);
                        self.mpv.set_property("volume", volume)?;
                    }
                    LibMpvMessage::UpdatePosition(offset) => {
                        self.mpv.command("seek", &[&offset.to_string()])?;
                    }
                    LibMpvMessage::SetPosition(pos) => {
                        self.mpv.command("seek", &[&pos.to_string(), "absolute"])?;
                    }
                    LibMpvMessage::PlayPause => {
                        self.mpv.command("cycle", &["pause"])?;
                    }
                    LibMpvMessage::PrevChapter => {
                        if self.chapters.len() > 0 {
                            let chapter = self.mpv.get_property::<i64>("chapter")? - 1;
                            if chapter >= 0 {
                                self.mpv.set_property("chapter", chapter)?;
                            }
                        }
                    }
                    LibMpvMessage::NextChapter => {
                        if self.chapters.len() > 0 {
                            let chapter = self.mpv.get_property::<i64>("chapter")? + 1;
                            if chapter < (self.chapters.len() as i64) {
                                self.mpv.set_property("chapter", chapter)?;
                            }
                        }
                    }
                    LibMpvMessage::Resume => {
                        self.mpv.set_property("pause", false)?;
                    }
                    LibMpvMessage::Pause => {
                        self.mpv.set_property("pause", true)?;
                    }
                }
            }
        }
        log::debug!("LibMpv::Run END");

        Ok(())
    }
}
