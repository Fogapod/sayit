# Say It!

String replacements using regex.

[![Crates.io](https://img.shields.io/crates/v/sayit)](https://crates.io/crates/sayit)
[![Documentation](https://docs.rs/sayit/badge.svg)](https://docs.rs/sayit)

Originally based on python [pink-accents](https://git.based.computer/fogapod/pink-accents) and primarily developed for [ssnt](https://github.com/SS-NT/ssnt/tree/main) game.

## Overview

Provides a way to define a set of rules for replacing text in string. Each rule consists of
regex pattern and Tag trait object. The original use case is to simulate
mispronounciations in speech accents via text.

See docs.rs documentation for API overview.

## Serialized format

Full reference:

```ron
(
    // main list of passes
    accent: [
        // `words` are applied before `patterns`
        (
            // names must be unique. they are used if you want to extend accent
            name: "words",

            // this optional field instructs all regexes inside this pass to be wrapped in \b
            format: r"\b{}\b",

            // pairs of (regex, tag)
            rules: {
                // this is the simplest rule to replace all "windows" words occurences with "spyware"
                "windows": {"Literal": "spyware"},

                // this replaces word "OS" with one of tags, with equal probability
                "os": {"Any": [
                    {"Literal": "Ubuntu"},
                    {"Literal": "Arch"},
                    {"Literal": "Gentoo"},
                ]},

                // `Literal` supports regex templating: https://docs.rs/regex/latest/regex/struct.Regex.html#example-9
                // this will swwap "a" and "b" "ab" -> "ba"
                r"(a)(?P<b_group>b)": {"Literal": "$b_group$a"},
            },
        ),
        (
            name: "patterns",
            rules: {
                // inserts one of the honks. first value of `Weights` is relative weight. higher is better
                "$": {"Weights": {
                    32: {"Literal": " HONK!"},
                    16: {"Literal": " HONK HONK!"},
                    08: {"Literal": " HONK HONK HONK!"},
                    // ultra rare sigma honk - 1 / 56
                    01: {"Literal": " HONK HONK HONK HONK!!!!!!!!!!!!!!!"},
                }},

                // lowercases all `p` letters (use "p" match from `Original`, then lowercase)
                "p": {"Lower": {"Original": ()}},

                // uppercases all `p` letters, undoing previous operation
                "P": {"Upper": {"Original": ()}},
            },
        ),
    ],

    // accent can be used with intensity (non negative value). higher intensities can either extend
    // lower level or completely replace it.
    // default intensity is 0. higher ones are defined here
    intensities: {
        // extends previous intensity (level 0, base one in this case), adding additional rules
        // below existingones. passes keep their relative order, rules inside passes also preserve order
        1: Extend([
            (
                name: "words",
                format: r"\b{}\b",
                rules: {
                    // even though we are extending, defining same rule will overwrite result.
                    // relative order of rules remain the same: "windows" will remain first
                    "windows": {"Literal": "bloatware"},
                },
            ),
            (
                // extend "patterns", adding 1 more rule
                name: "patterns",
                rules: {
                    // tags can be nested arbitrarily
                    "[A-Z]": {"Weights": {
                        // 50% to replace capital letter with one of the Es
                        1: {"Any": [
                            {"Literal": "E"},
                            {"Literal": "Ē"},
                            {"Literal": "Ê"},
                            {"Literal": "Ë"},
                            {"Literal": "È"},
                            {"Literal": "É"},
                        ]},
                        // 50% to do nothing, no replacement
                        1: {"Original": ()},
                    }},
                },
            ),
        ]),

        // replace intensity 1 entirely. in this case with nothing. remove all rules on intensity 2+
        2: Replace([]),
    },
)
```

See more examples in [examples](examples) folder.

## CLI tool

This library comes with a simple command line tool you can install with:

```sh
cargo install sayit --features=cli
```

Interactive session:

```sh
sayit --accent examples/scotsman.ron
```

Apply to file:

```sh
cat filename.txt | sayit --accent examples/french.ron > newfile.txt
```
