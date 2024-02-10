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
    // pairs of (regex, tag)
    // this is same as `patterns` except that each regex is surrounded with \b to avoid copypasting.
    // `words` are applied before `patterns`
    words: {
        // this is the simplest rule to replace all "windows" words (separated by regex \b)
        // occurences with "spyware": case sensitive
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

    // pairs of (regex, tag)
    // this is same as `words` except these are used as is, without \b
    patterns: {
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
        "p": {"Upper": {"Original": ()}},
    },

    // accent can be used with intensity (non negative value). higher intensities can either extend
    // lower level or completely replace it.
    // default intensity is 0. higher ones are defined here
    intensities: {
        // extends previous intensity (level 0, base one in this case), adding additional rules
        // below existingones. words and patterns keep their relative order though - words are
        // processed first
        1: Extend((
            words: {
                // even though we are extending, defining same rule will overwrite result.
                // relative order of rules remain the same: "windows" will remain first
                "windows": {"Literal": "bloatware"},
            },

            // extend patterns, adding 1 more rule
            patterns: {
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
        )),

        // replace intensity 1 entirely. in this case with nothing. remove all rules on intensity 2+
        2: Replace(()),
    },
)
```

See more examples in [examples](examples) folder.
