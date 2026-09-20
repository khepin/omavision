//! Playback is delegated to an external player wrapped in pre and post hooks.
use crate::config::Player;
use anyhow::{bail, Context, Result};
use log::warn;
use std::path::Path;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub struct Outcome {
    pub player: ExitStatus,
}

/// Runs `pre`, the player, then `post`. `post` always runs, even when the player fails to start.
pub fn play(cfg: &Player, file: &Path) -> Result<Outcome> {
    if let Err(e) = run_hook("pre", &cfg.pre) {
        warn!("{e:#}");
    }
    let result = run_player(cfg, file);
    if let Err(e) = run_hook("post", &cfg.post) {
        warn!("{e:#}");
    }
    result
}

fn run_player(cfg: &Player, file: &Path) -> Result<Outcome> {
    let (prog, args) = match cfg.command.split_first() {
        Some(x) => x,
        None => bail!("player.command is empty"),
    };
    let status = Command::new(prog)
        .args(args)
        .arg(file)
        .status()
        .with_context(|| format!("starting player `{prog}`"))?;
    Ok(Outcome { player: status })
}

fn run_hook(name: &str, argv: &[String]) -> Result<()> {
    let (prog, args) = match argv.split_first() {
        Some(x) => x,
        None => return Ok(()),
    };
    let status = Command::new(prog).args(args).status().with_context(|| format!("{name} hook: starting `{prog}`"))?;
    if !status.success() {
        bail!("{name} hook `{prog}` exited with {status}");
    }
    Ok(())
}
