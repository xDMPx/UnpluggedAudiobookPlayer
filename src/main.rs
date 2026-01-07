use std::io::Write;

use unplugged_audiobook_player::{
    ProgramOption,
    libmpv_handler::{LibMpvEventMessage, LibMpvMessage},
    print_help, process_args,
};

fn main() {
    let mut log_send: Option<unplugged_audiobook_player::logger::LogSender> = None;
    let options = process_args()
        .map_err(|err| {
            match err {
                unplugged_audiobook_player::UAPlayerError::InvalidOption(option) => {
                    eprintln!("Provided option {option} is invalid")
                }
                unplugged_audiobook_player::UAPlayerError::InvalidOptionsStructure => {
                    eprintln!("Invalid input")
                }
                unplugged_audiobook_player::UAPlayerError::InvalidFile => {
                    eprintln!("Provide valid audiobook file (.m4b/.mp3)")
                }
                _ => panic!("{:?}", err),
            }
            print_help();
            std::process::exit(-1);
        })
        .unwrap();
    if options.contains(&ProgramOption::PrintHelp) {
        print_help();
        std::process::exit(-1);
    }

    if options.contains(&ProgramOption::Verbose) {
        let logger = unplugged_audiobook_player::logger::Logger::new();
        log_send = Some(unplugged_audiobook_player::logger::LogSender::new(
            logger.get_signal_send(),
        ));
        log::set_boxed_logger(Box::new(log_send.as_ref().unwrap().clone())).unwrap();
        log::set_max_level(log::LevelFilter::Trace);

        std::thread::spawn(move || {
            logger.log();
            logger.flush();
        });
        log::debug!("Args: {:?}", std::env::args());
    }

    let volume = if let Some(vol) = options.iter().find_map(|o| match o {
        ProgramOption::Volume(vol) => Some(*vol),
        _ => None,
    }) {
        vol
    } else {
        100
    };

    let file_path = options
        .iter()
        .find_map(|o| match o {
            ProgramOption::PATH(path) => Some(path),
            _ => None,
        })
        .unwrap();
    let mut file = std::fs::File::create(format!("last.txt")).unwrap();
    file.write_all(file_path.as_bytes()).unwrap();
    log::debug!("File path: {file_path}");

    let time: f64 = if let Ok(str) = std::fs::read_to_string(format!("{file_path}.txt")) {
        str.parse().unwrap()
    } else {
        0.0
    };
    log::debug!("Time: {time}");

    let (tui_s, tui_r) = crossbeam::channel::unbounded();
    let (libmpv_s, libmpv_r) = crossbeam::channel::unbounded();
    let (mc_tui_s, mc_tui_r) = crossbeam::channel::unbounded();

    let mc_tui_s2 = mc_tui_s.clone();
    let tui_s2 = tui_s.clone();
    let libmpv_s2 = libmpv_s.clone();

    let mut mpv =
        unplugged_audiobook_player::libmpv_handler::LibMpvHandler::initialize_libmpv(volume)
            .unwrap();
    let mpv_client = mpv.create_client().unwrap();
    let mut mc_os_interface =
        unplugged_audiobook_player::mc_os_interface::MCOSInterface::new(libmpv_s.clone()).unwrap();

    crossbeam::scope(move |scope| {
        scope.spawn(move |_| {
            log::debug!("TUI: START");
            unplugged_audiobook_player::tui::tui(libmpv_s.clone(), tui_r)
                .map_err(|err| {
                    let _ = libmpv_s.send(LibMpvMessage::Quit);
                    let _ = mc_tui_s2.send(LibMpvEventMessage::Quit).unwrap();
                    err
                })
                .unwrap();
            log::debug!("TUI: END");
        });
        scope.spawn(move |_| {
            log::debug!("MPV: START");
            mpv.run(
                mpv_client,
                &file_path,
                time,
                tui_s.clone(),
                mc_tui_s.clone(),
                libmpv_r,
            )
            .map_err(|err| {
                let _ = tui_s.send(LibMpvEventMessage::Quit);
                let _ = mc_tui_s.send(LibMpvEventMessage::Quit);
                err
            })
            .unwrap();
            log::debug!("MPV: END");
        });
        scope.spawn(move |_| {
            log::debug!("MCOSInterface: START");
            mc_os_interface
                .handle_signals(mc_tui_r)
                .map_err(|err| {
                    let _ = tui_s2.send(LibMpvEventMessage::Quit);
                    let _ = libmpv_s2.send(LibMpvMessage::Quit);
                    err
                })
                .unwrap();
            log::debug!("MCOSInterface: END");
        });
    })
    .unwrap();

    if let Some(log_send) = log_send {
        log_send.send_quit_signal();
    }
}
