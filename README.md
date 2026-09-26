# dice-lint

Dice notation shows up all over tabletop tooling: character sheet configs,
loot tables, chat bot commands, game data files. It's plain text, so nothing
stops a typo from sitting there until someone rolls it: `0d6` in a loot
table quietly rolls nothing, `3D6` gets rejected by a parser that only
accepts lowercase `d`, `d20` with no count is ambiguous about whether it
means one die or a mistake, `5d1000` is almost certainly a stray zero.

dice-lint scans text files for tokens that look like dice notation and
checks each one, reporting the line and column so you can find it in an
editor.

## Usage

Given `rolls.txt`:

```
Fireball damage: 8d6
Sneak attack:    3D6
Initiative:      d20
Cursed die:      0d6
Loot roll:       5d1000
```

```
$ dice-lint rolls.txt
rolls.txt:2:18: error: '3D6' uses uppercase 'D'; write it lowercase [uppercase-d]
rolls.txt:3:18: error: 'd20' omits the dice count; write '1d20' [implicit-count]
rolls.txt:4:18: error: '0d6' rolls zero dice [zero-count]
rolls.txt:5:18: warning: '5d1000' uses an unusually large side count [large-sides]
```

The exit code is 1 if any `error` finding was reported, 0 otherwise, and 2 if
a file could not be read.

Pass more than one file and each is checked in turn, with the path in every
finding telling you which file it came from:

```
$ dice-lint rolls.txt loot.txt
```

Pass `-` instead of a file to read from stdin, useful for piping in output
from something else:

```
$ grep loot_table *.json | dice-lint -
```

Pass `--format json` to get one JSON object per finding instead, one per
line, useful for feeding into another tool:

```
$ dice-lint --format json rolls.txt
{"path":"rolls.txt","line":2,"col":18,"severity":"error","rule":"uppercase-d","message":"'3D6' uses uppercase 'D'; write it lowercase"}
{"path":"rolls.txt","line":3,"col":18,"severity":"error","rule":"implicit-count","message":"'d20' omits the dice count; write '1d20'"}
{"path":"rolls.txt","line":4,"col":18,"severity":"error","rule":"zero-count","message":"'0d6' rolls zero dice"}
{"path":"rolls.txt","line":5,"col":18,"severity":"warning","rule":"large-sides","message":"'5d1000' uses an unusually large side count"}
```

By default dice-lint is strict: it enforces lowercase `d`, an explicit dice
count, no leading zeros, and flags suspiciously large counts or side counts.
If your source material is looser than that on purpose (imported data,
freeform chat logs, homebrew with oversized dice), pass `--lenient` to skip
the style checks and keep only the ones that catch dice that can't actually
be rolled:

```
$ dice-lint --lenient rolls.txt
rolls.txt:4:18: error: '0d6' rolls zero dice [zero-count]
```

Pass `--rules` to turn individual rules on or off, regardless of what the
current mode would do by default. The value is a comma-separated list of
rule ids, each optionally prefixed with `-` (turn off) or `+` (turn on, same
as no prefix). Later entries win if an id shows up twice:

```
$ dice-lint --rules -large-sides rolls.txt
rolls.txt:2:18: error: '3D6' uses uppercase 'D'; write it lowercase [uppercase-d]
rolls.txt:3:18: error: 'd20' omits the dice count; write '1d20' [implicit-count]
rolls.txt:4:18: error: '0d6' rolls zero dice [zero-count]
```

This composes with `--lenient`: `--lenient --rules +leading-zero` runs only
the correctness checks plus `leading-zero`.

Percentile dice (`d%`, `3d%`) and fudge/Fate dice (`dF`, `4dF`) are recognized
too. Neither has a numeric side count, so the sides-related rules that only
make sense for a number (`zero-sides`, `leading-zero` on the sides, `flat-die`,
`large-sides`) don't apply to them, but count-related rules (`implicit-count`,
`zero-count`, `large-count`) and `uppercase-d` still do:

```
$ dice-lint --format json - <<< 'd% and 4DF'
{"path":"-","line":1,"col":1,"severity":"error","rule":"implicit-count","message":"'d%' omits the dice count; write '1d%'"}
{"path":"-","line":1,"col":9,"severity":"error","rule":"uppercase-d","message":"'4DF' uses uppercase 'D'; write it lowercase"}
```

## Rules

| rule | mode | meaning |
| --- | --- | --- |
| `missing-sides` | always | `3d` has no number of sides |
| `zero-count` | always | `0d6` rolls no dice |
| `zero-sides` | always | `3d0` has a die with no sides |
| `count-out-of-range` / `sides-out-of-range` | always | number too large to be a real roll |
| `bad-modifier` | always | trailing text after the sides isn't a known modifier (`kh`, `kl`, `dh`, `dl`, `r`, `!`, `+N`, `-N`) |
| `uppercase-d` | strict only | `3D6` instead of `3d6` |
| `implicit-count` | strict only | `d20` instead of `1d20` |
| `leading-zero` | strict only | `03d06` |
| `flat-die` | strict only | `2d1` is always 2, better written as `+2` |
| `large-count` / `large-sides` | strict only | count over 100 or sides over 1000, likely a typo |

`zero-sides`, `leading-zero` (sides), `flat-die`, and `large-sides` only apply
to a numeric sides count, so they never fire for percentile (`d%`) or fudge
(`dF`) dice.

## Building

Standard library only, no dependencies:

```
cargo build --release
```
