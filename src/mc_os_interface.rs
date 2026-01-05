use crate::{
    UAPlayerError,
    libmpv_handler::{LibMpvEventMessage, LibMpvMessage},
};

#[derive(Debug)]
pub enum MCOSInterfaceSignals {
    Pause,
    Resume,
    PlayNext,
    PlayPrev,
    UpdateMetadataTitle(String),
    End,
}

pub struct MCOSInterface {
    media_controller: souvlaki::MediaControls,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    dummy_window: windows_async::DummyWindow,
}

impl MCOSInterface {
    pub fn new(libmpv_s: crossbeam::channel::Sender<LibMpvMessage>) -> Result<Self, UAPlayerError> {
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;

        #[cfg(target_os = "windows")]
        let dummy_window = windows_async::create_dummy_window();
        #[cfg(target_os = "windows")]
        let hwnd = {
            use std::os::raw::c_void;
            let hwnd = dummy_window.hwnd().0.to_owned() as *mut c_void;
            Some(hwnd)
        };

        let config = souvlaki::PlatformConfig {
            dbus_name: "unplugged_audiobook_player",
            display_name: "UAP",
            hwnd,
        };

        let mut media_controller = souvlaki::MediaControls::new(config)?;

        // The closure must be Send and have a static lifetime.
        media_controller.attach(move |event: souvlaki::MediaControlEvent| {
            log::debug!("MCOSInterface::Event: {event:?}");
            let result = match event {
                souvlaki::MediaControlEvent::Play => libmpv_s.send(LibMpvMessage::Resume),
                souvlaki::MediaControlEvent::Pause => libmpv_s.send(LibMpvMessage::Pause),
                souvlaki::MediaControlEvent::Toggle => libmpv_s.send(LibMpvMessage::PlayPause),
                souvlaki::MediaControlEvent::Previous => libmpv_s.send(LibMpvMessage::PrevChapter),
                souvlaki::MediaControlEvent::Next => libmpv_s.send(LibMpvMessage::NextChapter),
                _ => Ok(()),
            };
            if result.is_err() {
                log::error!("{:?}", result.unwrap_err());
            }
        })?;

        Ok(MCOSInterface {
            media_controller,
            #[cfg(target_os = "windows")]
            dummy_window,
        })
    }

    pub fn handle_signals(
        &mut self,
        tui_r: crossbeam::channel::Receiver<crate::libmpv_handler::LibMpvEventMessage>,
    ) -> Result<(), UAPlayerError> {
        log::debug!("MCOSInterface::handle_signals Start");
        self.media_controller
            .set_playback(souvlaki::MediaPlayback::Playing { progress: None })?;

        let mut playback_start = std::time::SystemTime::now();
        let mut playback_start_offset = 0.0;
        let mut playback_paused = true;
        let mut playback_ready = false;

        let mut update_playback_timer = std::time::SystemTime::now();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            if let Ok(rec) = tui_r.try_recv() {
                log::debug!("MCOSInterface::LibMpvEventMessage: {rec:?}");
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
                        playback_start_offset = 0.0;

                        self.media_controller
                            .set_metadata(souvlaki::MediaMetadata {
                                title: Some(&data.media_title),
                                artist: data.artist.as_deref(),
                                album: data.album.as_deref(),
                                ..Default::default()
                            })?;
                    }
                    LibMpvEventMessage::PlaybackPause => {
                        self.media_controller
                            .set_playback(souvlaki::MediaPlayback::Paused { progress: None })?;

                        playback_start_offset += playback_start.elapsed()?.as_secs_f64();
                        playback_paused = true;
                    }
                    LibMpvEventMessage::PlaybackResume => {
                        self.media_controller
                            .set_playback(souvlaki::MediaPlayback::Playing { progress: None })?;

                        playback_start = std::time::SystemTime::now();
                        playback_paused = false;
                    }
                    LibMpvEventMessage::VolumeUpdate(_) => (),
                    LibMpvEventMessage::ChapterUpdate(_) => (),
                    LibMpvEventMessage::PositionUpdate(pos) => {
                        playback_start = std::time::SystemTime::now();
                        playback_start_offset = pos;
                    }
                    LibMpvEventMessage::Quit => {
                        break;
                    }
                }
            }

            if update_playback_timer.elapsed()?.as_secs_f64() > 0.25 {
                update_playback_timer = std::time::SystemTime::now();

                let playback_time = {
                    if !playback_ready {
                        0.0
                    } else if playback_paused {
                        playback_start_offset
                    } else {
                        playback_start_offset + playback_start.elapsed()?.as_secs_f64()
                    }
                };

                if playback_paused {
                    self.media_controller
                        .set_playback(souvlaki::MediaPlayback::Paused {
                            progress: Some(souvlaki::MediaPosition(
                                std::time::Duration::from_secs_f64(playback_time),
                            )),
                        })?;
                } else {
                    self.media_controller
                        .set_playback(souvlaki::MediaPlayback::Playing {
                            progress: Some(souvlaki::MediaPosition(
                                std::time::Duration::from_secs_f64(playback_time),
                            )),
                        })?;
                }
            }
        }
        log::debug!("MCOSInterface::handle_signals END");

        Ok(())
    }
}
