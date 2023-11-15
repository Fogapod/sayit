# pink accents

Allows defining a set of patterns to be replaced in string. This is a glorified regex replace, a sequence of them. Primary use case is for simulating silly speech accents.

Originally based on python [pink-accents](https://git.based.computer/fogapod/pink-accents) and primarily developed for [ssnt](https://github.com/SS-NT/ssnt/tree/main) game.

Currently unusable on it's own because you cannot construct `Accent` using internal structures but there is a plan to support programmatic definitions.

## Types of replacements

Accent is a sequence of rules which are applied in order.
Each rule consists of regex pattern and a replacement. When regex match occurs the replacement is called. It then decides what to put instead (if anything).

Possible replacements are:

- `Original`: Do not replace
- `Simple`: Puts string as is
- `Any` (recursive): Selects random replacement with equal weights
- `Weights` (recursive): Selects replacement based on relative weights
- `Uppercase` (recursive): Converts inner result to uppercase
- `Lowercase` (recursive): Converts inner result to lowercase

## Serialized format

`deserialize` feature provides an opinionated way of defining rules, specifically designed for speech accents.
Deserialization is primarily developed to support [ron](https://github.com/ron-rs/ron) format which has it's quirks but should work in json and maybe others.

Full reference:

```ron
(
    // on by default, tries to match input case with output after each rule
    // for example, if you replaced "HELLO" with "bye", it would use "BYE" instead
    normalize_case: true,

    // pairs of (regex, replacement)
    // this is same as `patterns` except that each regex is surrounded with \b to avoid copypasting.
    // `words` are applied before `patterns`
    words: [
        // this is the simplest rule to replace all "windows" words (separated by regex \b)
        // occurences with "linux", case sensitive
        ("windows", Simple("linux")),
        // this replaces word "OS" with one of replacements, with equal probability
        ("os", Any([
            Simple("Ubuntu"),
            Simple("Arch"),
            Simple("Gentoo"),
        ])),
    ],

    // pairs of (regex, replacement)
    // this is same as `words` except these are used as is, without \b
    patterns: [
        // inserts one of the honks. first value of `Weights` is relative weight. higher is better
        ("$", Weights([
            (32, Simple(" HONK!")),
            (16, Simple(" HONK HONK!")),
            (08, Simple(" HONK HONK HONK!")),
            // ultra rare sigma honk - 1 / 56
            (01, Simple(" HONK HONK HONK HONK!!!!!!!!!!!!!!!")),
        ])),
        // lowercases all `p` letters (use "p" match from `Original`, then lowercase)
        ("p", Lowercase(Original)),
        // uppercases all `p` letters, undoing previous operation
        ("p", Uppercase(Original)),
    ],

    // accent can be used with severity (non negative value). higher severities can either extend
    // lower level or completely replace it.
    // default severity is 0. higher ones are defined here
    severities: {
        // extends previous severity (level 0, base one in this case), adding additional rules
        // below existingones. words and patterns keep their relative order though - words are
        // processed first
        1: Extend(
            (
                words: [
                    // even though we are extending, defining same rule will overwrite result.
                    // relative order of rules remain the same: "windows" will remain first
                    ("windows", Simple("windoos")),
                ],

                // extend patterns, adding 1 more rule
                patterns: [
                    // replacements can be nested arbitrarily
                    ("[A-Z]", Weights([
                        // 50% to replace capital letter with one of the Es
                        (1, Any([
                            Simple("E"),
                            Simple("Ē"),
                            Simple("Ê"),
                            Simple("Ë"),
                            Simple("È"),
                            Simple("É"),
                        ])),
                        // 50% to do nothing, no replacement
                        (1, Original),
                    ])),
                ],
            ),
        ),

        // replace severity 1 entirely. in this case with nothing. remove all rules on severity 2+
        2: Replace(()),
    },
)
```

See more examples in [examples](examples) folder.
