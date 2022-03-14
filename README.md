# prompter

`prompter` is a command line tool that helps you choose the next word in a game of [Wordle](https://www.nytimes.com/games/wordle/index.html) - just like a promper in a theater tells the actors what to say next in case they forget.

## Demo

![Demo of how prompter is run in the terminal](demo.gif)

## Install

Archives of precompiled binaries for each [release](https://github.com/noeddl/prompter/releases) of `prompter` are available for Windows, macOS and Linux.

If you are a Rust programmer, `prompter` can be installed with `cargo`.

```
$ cargo install prompter
```

## Usage

You can use `prompter` in two ways: Either by letting it help you interactively during a game of Wordle or by letting it play by itself simulating how a game with a certain start and target word would have turned out.

### Get help during a Wordle game

```
$ prompter play
```

In each round, `prompter` presents you the 10 best-ranked words and asks you to input the word that you guessed in this round followed by a code representing the colors shown by Wordle (G = Green, Y = Yellow, X = Gray). See also the demo above.

### Simulate one or several games

```
$ prompter simulate --start <WORD> --target <WORD>
```

This subcommand simulates a game where `start` is the first word to be guessed and `target` is the mystery word that `prompter` tries to find. The next word after `start` is chosen by always "guessing" the word to which the algorithm assigns the highest score (words that have the same score are sorted lexicographically).

[Beispiel]

If no `--target` is given, `--start` is tested against all words in the wordlist.

[Beispiel]

Using this subcommand without any arguments runs the simulation on all combinations of words in the wordlist which takes several hours.

[Beispiel]

The results of running all simulations can be found in the file [data/results.csv](https://github.com/noeddl/prompter/blob/main/data/results.csv)

## Algorithm

`prompter`'s algorithm follows the simple intuition that a "good" word (or a good sequence of words) should eliminate a many candidates as possible. The idea is to find words that can "split" the wordlist in as many different ways as possible. For each word `w1` in the wordlist, `prompter` computes the color codes that Wordle would assign to each other word `w2` in the wordlist if the user playing the game wrote `w1` while `w2` is the target word to be found:

|  w1   |  w2   | Code  |
|-------|-------|-------|
| aback | aback | GGGGG |
| aback | abase | GGGXX |
| aback | abate | GGGXX |
| aback | abbey | GGXXX |
| ...   | ...   | ...   |

The number of color codes that `w1` can elicit is `w1`'s score. The higher the score, the better is a word considered a good next word. This calculation is repeated in each round on the remaining words after the Wordle's hints from previous rounds have been applied (i.e. `prompter` is always playing "hard mode").

## Wordlist

`prompter` uses the [list of Wordle's mystery words](https://docs.google.com/spreadsheets/d/1-M0RIVVZqbeh0mZacdAsJyBrLuEmhKUhNaVAI-7pr2Y/edit#gid=0) (minus the word "slave" which Wordle did not accept as a guess when I tried to use it). The list was provided by Zach Wissner-Gross, author of the column [The Riddler](https://fivethirtyeight.com/features/when-the-riddler-met-wordle/).

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.