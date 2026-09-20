mod logging;
mod replay;
mod tasks;
mod ui;

use anyhow::Result;
use clap::Parser;
use log::warn;
use omavision_core::browse::Browser;
use omavision_core::cache::Cache;
use omavision_core::{config, tmdb, Index};
use slint::ComponentHandle;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use ui::{AppState, Post};

slint::include_modules!();

#[derive(Parser, Debug)]
#[command(name = "omavision", about = "A TV media browser that hands playback to mpv")]
struct Args {
    /// Library root for this launch, overriding the config file
    #[arg(long)]
    root: Option<PathBuf>,
    /// Start in a window even when the config says fullscreen
    #[arg(long)]
    windowed: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    logging::init();
    let loaded = config::load_or_init()?;
    let mut config = loaded.config;
    if let Some(r) = &args.root {
        config.library.root = r.to_string_lossy().to_string();
    }
    if args.windowed {
        config.ui.fullscreen = false;
    }

    let ui = MainWindow::new()?;
    ui.global::<Theme>().set_scale(config.ui.scale);
    ui.global::<Theme>().set_attribution(tmdb::ATTRIBUTION.into());
    if config.ui.fullscreen {
        ui.window().set_fullscreen(true);
    }

    let state = Arc::new(Mutex::new(AppState {
        browser: Browser::new(Index::default(), HashMap::new(), config.root().unwrap_or_default()),
        cache: config.cache_dir().map(Cache::open),
        config: config.clone(),
        enriching: false,
    }));
    let post = Post::new(ui.as_weak(), state.clone());
    let _theme_watcher = tasks::watch_theme(&config, &ui, post.clone());

    match config.root() {
        Some(root) if root.is_dir() && config.cache_dir().is_some() => {
            let mut s = state.lock().unwrap();
            match s.cache.as_ref().map(Cache::index) {
                Some(Ok(Some(index))) => {
                    let meta = s.cache.as_ref().map(|c| c.load_all_meta(&index)).unwrap_or_default();
                    let n = index.item_count();
                    s.browser.set_index(index, meta);
                    ui.set_status(format!("{n} items from cache · scanning…").into());
                }
                Some(Err(e)) => {
                    warn!("cached index: {e:#}");
                    ui.set_status("scanning…".into());
                }
                _ => ui.set_status("scanning…".into()),
            }
            drop(s);
            tasks::scan(config.clone(), post.clone());
        }
        Some(root) => {
            ui.set_setup_needed(true);
            ui.set_setup_message(format!("Library root {} is not a directory.\nEdit your settings or point your agent to\n{}", root.display(), loaded.path.display()).into());
        }
        None => {
            ui.set_setup_needed(true);
            ui.set_setup_message(
                format!(
                    "{}Edit your settings or point your agent to\n{}\nand set library.root, then restart omavision.",
                    if loaded.created { "Settings file created.\n" } else { "" },
                    loaded.path.display()
                )
                .into(),
            );
        }
    }

    ui::render(&ui, &state.lock().unwrap());

    {
        let post = post.clone();
        let weak = ui.as_weak();
        ui.on_key(move |text| ui::handle_key(&weak.unwrap(), &post, &text));
    }

    let _replay = std::env::var("OMAVISION_REPLAY").ok().map(|script| replay::keys(&ui, script));

    if config.ui.fullscreen {
        keep_asking_for_fullscreen(ui.as_weak(), 20);
    }
    ui.run()?;
    Ok(())
}

/// Asking once before show does not stick on Hyprland: its first tiled configure lands
/// before the compositor acknowledges fullscreen, Slint's winit backend reads that as "not
/// fullscreen" and sends an unset. Re-ask every 100 ms for the first `tries` ticks; the check
/// reads the state the backend synced from the compositor, so this stops once it took.
fn keep_asking_for_fullscreen(weak: slint::Weak<MainWindow>, tries: u32) {
    slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
        let Some(ui) = weak.upgrade() else { return };
        if !ui.window().is_fullscreen() {
            ui.window().set_fullscreen(true);
        }
        if tries > 1 {
            keep_asking_for_fullscreen(weak, tries - 1);
        }
    });
}
