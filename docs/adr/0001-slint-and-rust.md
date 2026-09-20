---
status: accepted
---

# Slint and Rust for the UI

The app is developed on macOS and runs on an Omarchy laptop (Arch, Hyprland, Wayland). Both must build with the same command and the target must not need a runtime installed. Slint with Rust gives one `cargo` workflow on both machines, one static binary, native Wayland, and enough for a keyboard-driven poster grid.

## Considered options

- **Quickshell**: rejected. Wayland layer-shell only, does not run on macOS, shaped for bars and widgets.
- **QML with Qt 6**: kept as fallback. Better spatial navigation and virtualized grids, but needs a Qt runtime on both machines. Switch to it only if the 10-foot UI outgrows Slint.
- **Kodi**: rejected. Too large for a library of about 130 items.
