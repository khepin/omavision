---
status: accepted
---

# Config is TOML in ~/.config/omavision on every OS

Omarchy's convention is that agents edit everything as text files in well-known places. The config lives at `$XDG_CONFIG_HOME/omavision/config.toml`, falling back to `~/.config/omavision/config.toml`, on macOS as well as Linux. Using `~/Library/Application Support` on the Mac would be the platform norm but would put two paths in every instruction given to an agent.

The app writes a fully commented default file on first launch so the file exists before anyone reads docs. Secrets such as the provider API key go in a separate file next to it, never in `config.toml`.
