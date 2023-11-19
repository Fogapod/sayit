# pink accents

Allows defining a set of patterns to be replaced in string. This is a glorified regex replace, a sequence of them. Primary use case is for simulating silly speech accents.

Originally based on python [pink-accents](https://git.based.computer/fogapod/pink-accents) and primarily developed for [ssnt](https://github.com/SS-NT/ssnt/tree/main) game.

Currently unusable on it's own because you cannot construct `Accent` using internal structures but there is a plan to support programmatic definitions.

## Types of replacements

Accent is a sequence of rules which are applied in order.
Each rule consists of regex pattern and a replacement. When regex match occurs the replacement is called. It then decides what to put instead (if anything).

Supported replacements:

- `Original`: Do not replace
- `Literal("text")`: Puts string as is. Has templating and case mimicking by default
- `Any([inner, ...])`: Selects random replacement with equal weights
- `Weights([(weight, inner), ...])`: Selects replacement based on relative weights
- `Upper(inner)`: Converts inner result to uppercase
- `Lower(inner)`: Converts inner result to lowercase
- `Template(inner)`: Enables regex templating for inner types
- `NoTemplate(inner)`: Disables regex templating for inner types
- `MimicCase(inner)`: Enables case mimicking for inner types
- `NoMimicCase(inner)`: Disables case mimicking for inner types
- `Concat(left, right)`: Adds `left` and `right` together

## Serialized format

`deserialize` feature provides an opinionated way of defining rules, specifically designed for speech accents.
Deserialization is primarily developed to support [ron](https://github.com/ron-rs/ron) format which has it's quirks but should work in json and maybe others.

Full reference:

```ron
(
    // pairs of (regex, replacement)
    // this is same as `patterns` except that each regex is surrounded with \b to avoid copypasting.
    // `words` are applied before `patterns`
    words: [
        // this is the simplest rule to replace all "windows" words (separated by regex \b)
        // occurences with "linux", case sensitive
        ("windows", Literal("linux")),
        // this replaces word "OS" with one of replacements, with equal probability
        ("os", Any([
            Literal("Ubuntu"),
            Literal("Arch"),
            Literal("Gentoo"),
        ])),
        // `Literal` supports regex templating: https://docs.rs/regex/latest/regex/struct.Regex.html#example-9
        // this will swwap "a" and "b" "ab" -> "ba"
        (r"(a)(?P<b_group>b)", Literal("$b_group$a")),
    ],

    // pairs of (regex, replacement)
    // this is same as `words` except these are used as is, without \b
    patterns: [
        // inserts one of the honks. first value of `Weights` is relative weight. higher is better
        ("$", Weights([
            (32, Literal(" HONK!")),
            (16, Literal(" HONK HONK!")),
            (08, Literal(" HONK HONK HONK!")),
            // ultra rare sigma honk - 1 / 56
            (01, Literal(" HONK HONK HONK HONK!!!!!!!!!!!!!!!")),
        ])),
        // lowercases all `p` letters (use "p" match from `Original`, then lowercase)
        ("p", Lowercase(Original)),
        // uppercases all `p` letters, undoing previous operation
        ("p", Uppercase(Original)),
    ],

    // accent can be used with intensity (non negative value). higher intensities can either extend
    // lower level or completely replace it.
    // default intensity is 0. higher ones are defined here
    intensities: {
        // extends previous intensity (level 0, base one in this case), adding additional rules
        // below existingones. words and patterns keep their relative order though - words are
        // processed first
        1: Extend(
            (
                words: [
                    // even though we are extending, defining same rule will overwrite result.
                    // relative order of rules remain the same: "windows" will remain first
                    ("windows", Literal("windoos")),
                ],

                // extend patterns, adding 1 more rule
                patterns: [
                    // replacements can be nested arbitrarily
                    ("[A-Z]", Weights([
                        // 50% to replace capital letter with one of the Es
                        (1, Any([
                            Literal("E"),
                            Literal("Ē"),
                            Literal("Ê"),
                            Literal("Ë"),
                            Literal("È"),
                            Literal("É"),
                        ])),
                        // 50% to do nothing, no replacement
                        (1, Original),
                    ])),
                ],
            ),
        ),

        // replace intensity 1 entirely. in this case with nothing. remove all rules on intensity 2+
        2: Replace(()),
    },
)
```

See more examples in [examples](examples) folder.
