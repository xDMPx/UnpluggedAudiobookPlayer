use std::io::Write;

fn main() {
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        std::fs::File::create("debug.log").unwrap(),
    )
    .unwrap();

    let file_path = std::env::args()
        .nth(1)
        .map_or_else(|| std::fs::read_to_string("last.txt"), Ok)
        .expect("Provide file path\n");

    let abs_file_path = std::path::absolute(&file_path).unwrap();
    if !abs_file_path.try_exists().unwrap() {
        eprintln!("Provide valid file path");
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

    let mut mpv =
        unplugged_audiobook_player::libmpv_handler::LibMpvHandler::initialize_libmpv(101).unwrap();
    let mut mc_os_interface =
        unplugged_audiobook_player::mc_os_interface::MCOSInterface::new(libmpv_s.clone());

    crossbeam::scope(move |scope| {
        scope.spawn(move |_| {
            unplugged_audiobook_player::tui::tui(libmpv_s, tui_r);
        });
        scope.spawn(move |_| {
            mpv.run(&file_path, time, tui_s, mc_tui_s, libmpv_r);
        });
        scope.spawn(move |_| {
            mc_os_interface.handle_signals(mc_tui_r);
        });
    })
    .unwrap();
}
