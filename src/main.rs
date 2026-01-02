use std::io::Write;

use unplugged_audiobook_player::libmpv_handler::{LibMpvEventMessage, LibMpvMessage};

fn main() {
    let logger = unplugged_audiobook_player::logger::Logger::new();
    let log_send = unplugged_audiobook_player::logger::LogSender::new(logger.get_signal_send());
    log::set_boxed_logger(Box::new(log_send.clone())).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    std::thread::spawn(move || {
        logger.log();
        logger.flush();
    });

    let file_path = std::env::args()
        .nth(1)
        .map_or_else(|| std::fs::read_to_string("last.txt"), Ok)
        .expect("Provide file path\n");

    let abs_file_path = std::path::absolute(&file_path).unwrap();
    if !abs_file_path.try_exists().unwrap() {
        eprintln!("Provide valid file path");
        std::process::exit(0);
    }
    if !is_audiofile(&abs_file_path) {
        eprintln!("Provide valid audiobook file (.m4b/.mp3)");
        std::process::exit(0);
    }

    let mut file = std::fs::File::create(format!("last.txt")).unwrap();
    file.write_all(abs_file_path.to_str().unwrap().as_bytes())
        .unwrap();
    log::debug!("File path: {file_path}");

    let time: f64 = if let Ok(str) = std::fs::read_to_string(format!("{file_path}.txt")) {
        str.parse().unwrap()
    } else {
        0.0
    };
    log::debug!("Time: {file_path}");

    let (tui_s, tui_r) = crossbeam::channel::unbounded();
    let (libmpv_s, libmpv_r) = crossbeam::channel::unbounded();
    let (mc_tui_s, mc_tui_r) = crossbeam::channel::unbounded();

    let mc_tui_s2 = mc_tui_s.clone();
    let tui_s2 = tui_s.clone();
    let libmpv_s2 = libmpv_s.clone();

    let mut mpv =
        unplugged_audiobook_player::libmpv_handler::LibMpvHandler::initialize_libmpv(100).unwrap();
    let mut mc_os_interface =
        unplugged_audiobook_player::mc_os_interface::MCOSInterface::new(libmpv_s.clone()).unwrap();

    crossbeam::scope(move |scope| {
        scope.spawn(move |_| {
            unplugged_audiobook_player::tui::tui(libmpv_s.clone(), tui_r)
                .map_err(|err| {
                    let _ = libmpv_s.send(LibMpvMessage::Quit);
                    let _ = mc_tui_s2.send(LibMpvEventMessage::Quit).unwrap();
                    err
                })
                .unwrap();
        });
        scope.spawn(move |_| {
            mpv.run(&file_path, time, tui_s.clone(), mc_tui_s.clone(), libmpv_r)
                .map_err(|err| {
                    let _ = tui_s.send(LibMpvEventMessage::Quit);
                    let _ = mc_tui_s.send(LibMpvEventMessage::Quit);
                    err
                })
                .unwrap();
        });
        scope.spawn(move |_| {
            mc_os_interface
                .handle_signals(mc_tui_r)
                .map_err(|err| {
                    let _ = tui_s2.send(LibMpvEventMessage::Quit);
                    let _ = libmpv_s2.send(LibMpvMessage::Quit);
                    err
                })
                .unwrap();
        });
    })
    .unwrap();

    log_send.send_quit_signal();
}

fn is_audiofile(path: &std::path::PathBuf) -> bool {
    if let Some(ext) = path.extension() {
        if ext == "m4b" {
            return true;
        } else if ext == "mp3" {
            return true;
        }
    }

    false
}
