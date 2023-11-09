#!/usr/bin/sh

echo building CLI with debug enabled
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release --features cli

echo recording perfomance for 10 seconds
# infinitely cycle through sample_text.txt and send lines to cli
timeout 10 sh -c 'while true; do cat tests/sample_text.txt; done | while read line; do echo "$line"; done' | CARGO_PROFILE_RELEASE_DEBUG=true perf record --call-graph dwarf -- target/release/pink_accents -a examples/linux.ron > /dev/null

echo folding stacks
perf script | inferno-collapse-perf > stacks.folded

echo generating flamegraph.svg
cat stacks.folded | inferno-flamegraph > flamegraph.svg

echo cleaning up
rm perf.data
rm stacks.folded
