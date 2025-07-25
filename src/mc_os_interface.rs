use crate::libmpv_handler::{LibMpvEventMessage, LibMpvMessage};

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
    pub fn new(libmpv_s: crossbeam::channel::Sender<LibMpvMessage>) -> Self {
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

        let mut media_controller = souvlaki::MediaControls::new(config).unwrap();

        // The closure must be Send and have a static lifetime.
        media_controller
            .attach(move |event: souvlaki::MediaControlEvent| {
                log::debug!("MCOSInterface::Event: {event:?}");
                match event {
                    souvlaki::MediaControlEvent::Play => {
                        libmpv_s.send(LibMpvMessage::PlayPause).unwrap();
                    }
                    souvlaki::MediaControlEvent::Pause => {
                        libmpv_s.send(LibMpvMessage::PlayPause).unwrap();
                    }
                    souvlaki::MediaControlEvent::Previous => {
                        libmpv_s.send(LibMpvMessage::PrevChapter).unwrap();
                    }
                    souvlaki::MediaControlEvent::Next => {
                        libmpv_s.send(LibMpvMessage::NextChapter).unwrap();
                    }
                    _ => (),
                }
            })
            .unwrap();

        MCOSInterface {
            media_controller,
            #[cfg(target_os = "windows")]
            dummy_window,
        }
    }

    pub fn handle_signals(
        &mut self,
        tui_r: crossbeam::channel::Receiver<crate::libmpv_handler::LibMpvEventMessage>,
    ) {
        log::debug!("MCOSInterface::handle_signals Start");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            if let Ok(rec) = tui_r.try_recv() {
                log::debug!("MCOSInterface::LibMpvEventMessage: {rec:?}");
                match rec {
                    LibMpvEventMessage::StartFile => {
                        self.media_controller
                            .set_playback(souvlaki::MediaPlayback::Playing { progress: None })
                            .unwrap();
                    }
                    LibMpvEventMessage::PlaybackRestart(paused) => {
                        if paused {
                            self.media_controller
                                .set_playback(souvlaki::MediaPlayback::Playing { progress: None })
                                .unwrap();
                        } else {
                            self.media_controller
                                .set_playback(souvlaki::MediaPlayback::Paused { progress: None })
                                .unwrap();
                        }
                    }
                    LibMpvEventMessage::FileLoaded(data) => {
                        self.media_controller
                            .set_playback(souvlaki::MediaPlayback::Playing { progress: None })
                            .unwrap();
                        self.media_controller
                            .set_metadata(souvlaki::MediaMetadata {
                                title: Some(&data.media_title),
                                ..Default::default()
                            })
                            .unwrap();
                    }
                    LibMpvEventMessage::PlaybackPause => {
                        self.media_controller
                            .set_playback(souvlaki::MediaPlayback::Paused { progress: None })
                            .unwrap();
                    }
                    LibMpvEventMessage::PlaybackResume => {
                        self.media_controller
                            .set_playback(souvlaki::MediaPlayback::Playing { progress: None })
                            .unwrap();
                    }
                    LibMpvEventMessage::VolumeUpdate(_) => (),
                    LibMpvEventMessage::ChapterUpdate(_) => (),
                    LibMpvEventMessage::PositionUpdate(_) => (),
                    LibMpvEventMessage::Quit => {
                        break;
                    }
                }
            }
        }
        log::debug!("MCOSInterface::handle_signals END");
    }
}
