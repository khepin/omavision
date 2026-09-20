# Omavision task runner. `just` lists these.

lib := "fixtures/library"

default:
    @just --list

# Build and run windowed against the local fixture
run *args:
    cargo run -p omavision -- --windowed --root {{lib}} {{args}}

# Build and run fullscreen with the config file's library root
tv:
    cargo run -p omavision -- --root {{lib}}

# Run all tests
test:
    cargo test

# Rebuild the empty-file fixture library from fixtures/library.txt
fixtures:
    ./fixtures/make-library.sh

# Show what the TMDB matcher sees for one title: just probe "Ponyo" 2008
probe title year="":
    cargo run -q -p omavision-core --example probe -- "{{title}}" {{year}}

# Replay keys through the app: just replay "Right Down Down Return"
replay keys:
    OMAVISION_REPLAY="{{keys}}" cargo run -p omavision -- --windowed --root {{lib}}
