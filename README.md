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
    // Consists of named blocks named "pass" that are applied in top to bottom order
    // pass names must be unique. they are used if you want to extend accent
    accent: {
        // First pass
        "words": (
            // This optional field instructs all regexes inside this pass to be wrapped in
            // regex word boundaries
            format: r"\<{}\>",

            // Pairs of (regex, tag)
            rules: {
                // Simplest rule to replace all "windows" words occurences with "spyware"
                "windows": {"Literal": "spyware"},

                // This replaces word "os" with one of tags, with equal probability
                "os": {"Any": [
                    {"Literal": "Ubuntu"},
                    {"Literal": "Arch"},
                    {"Literal": "Gentoo"},
                ]},

                // `Literal` supports regex templating:
                // https://docs.rs/regex/latest/regex/struct.Regex.html#example-9
                // This will swap "a" and "b" using named and numbered groups
                r"(a)(?P<b_group>b)": {"Literal": "$b_group$1"},
            },
        ),

        // Second pass
        "patterns": (
            // Both rules use "(?-i)" which opts out of case insensivity
            rules: {
                // Lowercases all `P` letters
                "(?-i)P": {"Lower": {"Original": ()}},

                // Uppercases all `m` letters
                "(?-i)m": {"Upper": {"Original": ()}},
            },
        ),

        // Third pass. note that ^ and $ may overlap with words at beginning and
        // end of strings. These should be defined separately
        "ending": (
            rules: {
                // Selects honks using relative weights. Higher is better
                "$": {"Weights": {
                    32: {"Literal": " HONK!"},
                    16: {"Literal": " HONK HONK!"},
                    08: {"Literal": " HONK HONK HONK!"},
                    // Ultra rare sigma honk - 1 / 56 chance
                    01: {"Literal": " HONK HONK HONK HONK!!!!!!!!!!!!!!!"},
                }},
            },
        ),
    },

    // Accent can be used with intensity (non negative value). Higher
    // intensities can either extend lower level or completely replace it.
    // Default intensity (rules above) is 0. Higher ones are defined here
    intensities: {
        // Extends previous intensity (base one in this case), adding additional
        // rules and overwritiong passes that have same names.
        1: Extend({
            "words": (
                format: r"\<{}\>",
                rules: {
                    // Will overwrite "windows" pattern in "main" pass
                    "windows": {"Literal": "bloatware"},
                },
            ),

            // Extend "patterns", adding 1 more rule with new pattern
            "patterns": (
                name: "patterns",
                rules: {
                    "(?-i)[A-Z]": {"Weights": {
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
        }),

        // Replace intensity 1 entirely. In this case with nothing
        2: Replace({}),
    },
)
```

See more examples in [examples](https://git.based.computer/fogapod/sayit/src/branch/master/examples) folder.

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
