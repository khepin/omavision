---
status: accepted
---

# Playback is delegated to mpv, never embedded

Omavision spawns the configured player command with the file path and waits for it to exit. It passes no playback settings: subtitles, audio tracks and quality are mpv's own config, which is also an agent-editable text file. Embedding video would multiply the size and platform surface of the app for no gain over mpv.

Two hooks, `pre` and `post`, run around the player as argv arrays. `post` always runs, even when the player fails, because it undoes display changes such as Omarchy's movie mode.
