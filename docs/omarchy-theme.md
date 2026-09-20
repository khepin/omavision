# Reading the Omarchy theme

Verified against the Omarchy repo on 2026-09-19 (version 4.0.0.alpha, branch `quattro`; `github.com/basecamp/omarchy` redirects to `omacom/omarchy`).

## Where the active theme lives

- `~/.local/state/omarchy/current/theme/` is a real directory, replaced atomically by `mv` on every theme change.
- `~/.local/state/omarchy/current/theme.name` holds the kebab-case name, e.g. `tokyo-night`.
- Pre-migration installs used `~/.config/omarchy/current/theme`. Check the state path first, then fall back.
- If neither exists, omavision uses its own built-in palette. The Mac never has one.

## The file to parse: `colors.toml`

Flat TOML, no tables, quoted `#rrggbb` strings. Always present in the current theme dir because Omarchy generates it from `alacritty.toml` for themes that lack it. Example, tokyo-night verbatim:

```toml
mode = "dark"

accent = "#7aa2f7"
selection = "#292e42"
muted = "#414868"

background = "#1a1b26"
dark_background = "#13141c"
darker_background = "#0e0e14"
lighter_background = "#24283b"

foreground = "#a9b1d6"
dark_foreground = "#565f89"
light_foreground = "#b4bee6"
bright_foreground = "#c0caf5"

red = "#f7768e"
yellow = "#e0af68"
orange = "#eb927b"
green = "#9ece6a"
cyan = "#449dab"
blue = "#7aa2f7"
magenta = "#ad8ee6"
brown = "#75493d"

bright_red = "#ff7a93"
bright_yellow = "#ff9e64"
bright_green = "#b9f27c"
bright_cyan = "#0db9d7"
bright_blue = "#7da6ff"
bright_magenta = "#bb9af7"
```

Semantics from `docs/theming.md`: `foreground` primary text, `background` primary ground, `accent` preferred accent, `muted` comments, placeholders and dividers, `selection` selection background, `red` urgent.

## Fallback chain (mirror `bin/omarchy-theme-color`)

Third-party or old themes may use legacy keys. Resolve in this order:

| Token | Try in order |
| --- | --- |
| background | `background`, `bg`, `color0` |
| foreground | `foreground`, `fg`, `color7` |
| lighter_background | `lighter_background`, `lighter_bg`, else background mixed toward foreground |
| dark_background | `dark_background`, `dark_bg`, else background mixed 25% toward black |
| accent | `accent`, `blue`, `color4` |
| magenta | `magenta`, `color5`, `purple` |
| muted | `muted`, `color8`, `dark_foreground` |
| selection | `selection`, `selection_background`, `color8`, `background` |
| mode | `mode`, legacy `theme_type`, legacy empty file `light.mode` beside the toml, else light when R+G+B of background > 382, else dark |

Some keys may hold Hyprland gradients such as `rgba(33ccffee) rgba(00ff99ee) 45deg`. Accept and ignore non-hex values.

Alternative when the binary exists: `omarchy-theme-color --all` prints fully resolved `key<TAB>value` lines including `mode`.

## Omavision token mapping

| Omavision token | colors.toml key |
| --- | --- |
| bg | background |
| surface | lighter_background |
| surface2 | selection |
| text | foreground |
| text2 (secondary text) | dark_foreground, else foreground mixed 40% toward background |
| muted (rules only) | muted |
| accent | accent |
| accent2 | magenta |
| border | muted |

## Reacting to a theme change

No D-Bus signal. Two options:

1. Watch `~/.local/state/omarchy/current/` with inotify for the `theme` directory being replaced or `theme.name` rewritten. Watch the parent, not the file, because the directory is swapped by `mv`.
2. Install a hook: `omarchy hook install theme-set <script>`, which runs `bash <script> <theme-name>` after every change. Executables in `~/.config/omarchy/hooks/theme-set.d/` also run.

Option 1 is what omavision does (`theme::watch` in the core crate). `ui.theme_file` in the config points at a specific `colors.toml` for machines without Omarchy.

## Reference values for six shipped themes

| theme | mode | background | foreground | accent | magenta | muted | lighter_background | selection |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| tokyo-night | dark | #1a1b26 | #a9b1d6 | #7aa2f7 | #ad8ee6 | #414868 | #24283b | #292e42 |
| catppuccin | dark | #1e1e2e | #cdd6f4 | #89b4fa | #f5c2e7 | #585b70 | #313244 | #45475a |
| gruvbox | dark | #282828 | #d4be98 | #7daea3 | #d3869b | #665c54 | #3c3836 | #504945 |
| nord | dark | #2e3440 | #d8dee9 | #81a1c1 | #b48ead | #4c566a | #3b4252 | #434c5e |
| everforest | dark | #2d353b | #d3c6aa | #7fbbb3 | #d699b6 | #475258 | #343f44 | #3d484d |
| rose-pine | light | #faf4ed | #575279 | #56949f | #907aa9 | #cecacd | #f2e9e1 | #dfdad9 |

Sources: `docs/theming.md`, `bin/omarchy-theme-set`, `bin/omarchy-theme-color`, `bin/omarchy-hook`, `migrations/1781043107.sh`, and each theme's `colors.toml` in the Omarchy repository.
