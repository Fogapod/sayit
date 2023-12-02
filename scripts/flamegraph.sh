#!/usr/bin/sh

echo building CLI with debug enabled
CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_STRIP=false cargo build --release --features cli

echo recording perfomance for 20 seconds
# infinitely cycle through sample_text.txt and send lines to cli
timeout 20 python -c 'for l in __import__("itertools").cycle(l.strip() for l in open("tests/sample_text.txt").readlines() if l.strip() and l != " :\n"): print(l)' | pv | perf record --call-graph dwarf -- target/release/pink_accents -a examples/linux.ron > /dev/null

echo folding stacks
perf script | inferno-collapse-perf > stacks.folded

echo generating flamegraph.svg
cat stacks.folded | inferno-flamegraph > flamegraph.svg

echo cleaning up
rm perf.data
rm stacks.folded
