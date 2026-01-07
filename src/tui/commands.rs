#[derive(Debug, Clone)]
pub enum TuiCommand {
    Quit,
    Volume(i64),
    Seek(f64),
    PlayPause,
    NextChapter,
    PrevChapter,
    EnterCommandMode(bool),
}

fn quit(_: &mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand> {
    Some(TuiCommand::Quit)
}

fn seek(args: &mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand> {
    let offset: f64 = args.next()?.parse().ok()?;
    Some(TuiCommand::Seek(offset))
}

fn vol(args: &mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand> {
    let arg = args.next()?;
    if arg.starts_with('-') || arg.starts_with('+') {
        let mut volume: i64 = arg.parse().ok()?;
        volume = volume.clamp(-200, 200);
        Some(TuiCommand::Volume(volume))
    } else {
        None
    }
}

fn playpause(_: &mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand> {
    Some(TuiCommand::PlayPause)
}

fn playnext(_: &mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand> {
    Some(TuiCommand::NextChapter)
}

fn playprev(_: &mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand> {
    Some(TuiCommand::PrevChapter)
}

type CmdFn = fn(&mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand>;

static COMMANDS: phf::Map<
    &'static str,
    fn(&mut std::str::SplitWhitespace<'_>) -> Option<TuiCommand>,
> = phf::phf_map! {
    "quit" => quit as CmdFn,
    "q" => quit as CmdFn,
    "vol" => vol as CmdFn,
    "seek" => seek as CmdFn,
    "play-pause" => playpause as CmdFn,
    "play-next" => playnext as CmdFn,
    "play-prev" => playprev as CmdFn,
};

pub fn map_str_to_tuicommand(str: &str) -> Option<TuiCommand> {
    if str.split_whitespace().count() > 2 {
        return None;
    }

    let mut tokens = str.split_whitespace();
    let command_str = tokens.next()?;

    COMMANDS.get(command_str).map(|f| f(&mut tokens))?
}
