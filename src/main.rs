fn main() {
    let file_path = std::env::args().skip(1).next().expect("Provide file path");
    let time: f64 = if let Ok(str) = std::fs::read_to_string(format!("{file_path}.txt")) {
        str.parse().unwrap()
    } else {
        0.0
    };

    let (tui_s, tui_r) = crossbeam::channel::unbounded();
    let (libmpv_s, libmpv_r) = crossbeam::channel::unbounded();
    crossbeam::scope(move |scope| {
        scope.spawn(|_| {
            unplugged_audiobook_player::tui::tui(libmpv_s, tui_r);
        });
        scope.spawn(move |_| {
            unplugged_audiobook_player::libmpv_handler::libmpv(&file_path, time, tui_s, libmpv_r);
        });
    })
    .unwrap();
}
