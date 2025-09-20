# UnpluggedAudiobookPlayer

---

A fully offline, TUI‑based audiobook player written in Rust.

## Preview

<img src="assets/preview.gif"></img>

## Features

- Playback progress saved in a plain text file
	- The progress is saved independently for all books in their folder
	- Inspired by [Smart AudioBook Player](https://play.google.com/store/apps/details?id=ak.alizandro.smartaudiobookplayer&hl=en-US)
- Play next and play prev switches chapters
- Chapter based navigation – `Play next` / `Play previous` switches between chapters
- Integration with OS media controls and metadata system
	- Linux - MPRIS
	- Windows - SystemMediaTransportControls
	- thanks to [souvlaki](https://crates.io/crates/souvlaki)
-  Support for audiobooks in format:
	- m4b
	- mp3
- [cmus](https://cmus.github.io/) inspired controls
- Last‑used file is automatically reloaded when no path is supplied

## Keybindings

| Key       | Action       |
| --------- | ------------ |
| `z`       | Prev Chapter |
| `b`       | Next Chapter |
| `Space`   | Play / Pause |
| `q`       | Quit         |
| `{`       | Volume -1    |
| `}`       | Volume +1    |
| `[`       | Volume -10   |
| `]`       | Volume +10   |
| ←         | Seek -10 s   |
| Shift + ← | Seek -60 s   |
| →         | Seek +10 s   |
| Shift + → | Seek +60 s   |

## Building

To build, clone this repository and run:
```sh
	$ cargo build --release
```

## Installation

### Linux

Locally:
```sh
	$ CARGO_INSTALL_ROOT=~/.local cargo install --path=.
```

### Windows

Build the binary as described in [Building](#Building) section and use the generated executable in `target/release`.

## Usage

```sh
	# Start the player with a specific file
	$ unplugged_audiobook_player /path/to/audiobook_file.[m4b|mp3]
	# Resume the last‑used file
	$ unplugged_audiobook_player 
```

or

```sh
	# Start the player with a specific file
	$ cargo run --release -- /path/to/audiobook_file.[m4b|mp3]
	# Resume the last‑used file
	$ cargo run --release
```

## License

This project is licensed under [MIT](LICENSE) License.
