---
status: accepted
---

# Cache lives next to the media, state lives in XDG state

The cache (index, metadata, posters) is stored in `<library root>/.omavision/`. It is derived and safe to delete, and keeping it with the media means the posters travel with the disk. User state (last played) and logs are not derived, so they live in `$XDG_STATE_HOME/omavision/`, default `~/.local/state/omavision/`, where the XDG spec places data that persists but is not worth backing up.

## Consequences

- Wiping the cache never loses what a person watched.
- The Mac and the laptop do not share state; they do not share the same physical library either.
