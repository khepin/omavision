# Omavision

A TV media browser for a personal video collection on a local disk. It lists what is there, decorates it with posters and synopses, and hands playback to an external player.

## Language

### Library

**Library**:
The root folder that holds every video the app knows about. There is exactly one.
_Avoid_: collection, media folder, root

**Category**:
A top-level folder of the library. Categories are discovered, never configured.
_Avoid_: section, genre, tab

**Item**:
Anything a person can select in a category: a movie, a show, or a flat item.
_Avoid_: entry, media, title

**Movie**:
An item that is a single video file with a title and a release year.
_Avoid_: film

**Show**:
An item that groups seasons. A show is never played directly.
_Avoid_: series, TV show

**Season**:
An ordered group of episodes within a show.

**Episode**:
A single video file inside a season, identified by its season and episode numbers.

**Flat item**:
An item that is a single video file with no recognizable year or episode pattern. Shown by its cleaned filename.
_Avoid_: misc, other, unknown

### Discovery

**Scan**:
The comparison of the library on disk against the index. A scan adds new items, removes missing ones, and touches nothing else.
_Avoid_: refresh, sync, import

**Index**:
The stored mirror of what a scan found. Derived from disk, so rebuildable at any time.
_Avoid_: database, catalog

**Name parsing**:
The extraction of title, year, season and episode from a file or folder name. It is the only source of identity for an item.

### Metadata

**Provider**:
The external service that supplies metadata. Today it is TMDB.

**Enrichment**:
The act of fetching metadata for an item from the provider and storing it locally. Enrichment never blocks browsing; an item without it shows its parsed name.
_Avoid_: scrape, fetch, lookup

**Metadata**:
What enrichment brings back for an item: poster, synopsis, localized title, episode titles.

**Poster**:
The one image that represents an item in a grid.
_Avoid_: thumbnail, cover, artwork

**Cache**:
Everything derived from disk or the provider: the index, metadata and posters. Deleting it loses nothing that cannot be rebuilt.

### Playback

**Player**:
The external program that plays a file. Omavision starts it and waits for it to exit. The player owns subtitles, audio and quality choices.

**Hook**:
A command run before the player starts or after it exits, for things like switching the display into movie mode.
_Avoid_: script, callback

**State**:
What the app remembers about a person's use: which item was played last and when. Not derived, so never part of the cache.
_Avoid_: history, progress

**Next unwatched**:
The episode after the last-played episode of a show, or its first episode when none was played.
