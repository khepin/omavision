# Omavision

A TV media browser for a personal video library. It lists what is on disk, will decorate it with posters and synopses from TMDB, and hands playback to mpv. Built with Rust and Slint; runs on macOS for development and on Omarchy (Arch, Hyprland) on the TV.

## Build and run

```
cargo build
./target/debug/omavision --windowed --root /path/to/library
```

The first launch writes `~/.config/omavision/config.toml` with every setting documented. Set `library.root` there and drop the `--root` flag. `library.order` lists the categories to show first; the rest follow alphabetically. `--windowed` overrides `ui.fullscreen` for one launch.

## Metadata

Posters and synopses come from TMDB. Create a free key at https://www.themoviedb.org/settings/api and put it, alone, in `~/.config/omavision/tmdb.key`. Both a v3 key and a v4 read access token work. Without the file the app shows parsed names only and says so in the status line.

## Theme

Colours follow the Omarchy theme when `~/.local/state/omarchy/current/theme/colors.toml` exists, and update live when it changes. `ui.theme_file` in the config points at any `colors.toml` instead. See `docs/omarchy-theme.md`.

## Developing without the media

`fixtures/library.txt` lists the real library's file names. `./fixtures/make-library.sh` turns it into `fixtures/library/` as empty files. Run with `--root fixtures/library` to develop the browser without the drive or the mount; playback of an empty file fails, which is expected.

## Keys

Arrows move. Left/Right switch category. Typing filters, Backspace edits, Esc clears. Enter plays. PageUp/PageDown move ten rows.

## Driving it without a keyboard

`OMAVISION_REPLAY="Right w i c k Down Return"` replays keys through the normal input pipeline after startup, one every 400 ms. Useful for agents and for screenshots. Names: `Up Down Left Right Return Escape Backspace PageUp PageDown Home End Space`; anything else is typed literally.

## Where things live

| What | Where |
| --- | --- |
| Settings | `~/.config/omavision/config.toml` |
| Scan index, metadata, posters | `<library root>/.omavision/` (safe to delete) |
| Played state and logs | `~/.local/state/omavision/` (`omavision.log`, level from `RUST_LOG`) |
| Vocabulary | `CONTEXT.md` |
| Decisions | `docs/adr/` |
| Look and feel | `docs/design-direction.md`, mockup in `docs/mockups/filmotheque/` |
| Omarchy theme integration | `docs/omarchy-theme.md` |

## Layout

```
crates/core        omavision-core: config, name parsing, scan, index, cache,
                   metadata, search, browse (the keyboard model) and player
crates/omavision   the binary: Slint UI, worker threads and bundled fonts
```

The binary owns no logic worth testing: `browse::Browser` answers a key with an `Effect` and
hands back a `View`, and `ui.rs` turns that into Slint properties.
