use std::{
    collections::HashMap,
    io::{self, Write},
};

use clap::{Parser, Subcommand};
use env_logger::{Builder, Target};
use itertools::Itertools;
use log::{debug, info, LevelFilter};
use prompter::*;

/// Length of the word to be guessed.
const WORD_LEN: usize = 5;

/// Number of rounds to play.
const ROUND_NUM: usize = 6;

#[derive(Parser)]
#[clap(name = "prompter")]
#[clap(about = "A Wordle solver in Rust", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get help while playing Wordle
    Play {},
    /// Simulate a Wordle game
    Simulate {
        /// Start word
        #[clap(long, short, value_name = "WORD")]
        start: Option<String>,

        /// Target word
        #[clap(long, short, requires = "start", value_name = "WORD")]
        target: Option<String>,
    },
    /// Show the different "buckets" in which the words in the wordlist are sorted for WORD
    Buckets {
        #[clap(value_name = "WORD")]
        word: String,
    },
}

fn main() {
    let args = Cli::parse();

    match &args.command {
        Commands::Play {} => {
            play();
        }
        Commands::Simulate { start, target } => {
            let mut builder = Builder::new();

            builder
                .format(|buf, record| writeln!(buf, "{}", record.args()))
                .target(Target::Stdout);

            let level = match (start, target) {
                (Some(_), Some(_)) => LevelFilter::Debug,
                (None, None) => LevelFilter::Warn,
                (_, _) => LevelFilter::Info,
            };

            builder.filter_level(level);
            builder.init();
            simulate_all(start.as_ref(), target.as_ref());
        }
        Commands::Buckets { word } => {
            let word = Word::from(word);

            let wordlist = Wordlist::load();

            let mut map = HashMap::new();

            for w in &wordlist {
                let code = word.match_code(w);

                let vec = map.entry(code).or_insert_with(Vec::new);
                vec.push(w);
            }

            println!("\"{}\" has {} Wordle buckets.", word, map.len());

            for (code, words) in map.iter().sorted() {
                println!("\n{} ({} word{})", code, words.len(), plural(words.len()));

                for w in words {
                    println!("{}", w);
                }
            }
        }
    }
}

fn plural(number: usize) -> String {
    let s = if number == 1 { "" } else { "s" };

    s.to_string()
}

fn play() {
    println!("Welcome! Let's play Wordle.");

    let mut wordlist = Wordlist::load();

    for i in 1..=ROUND_NUM {
        println!(
            "\n---[ Round #{} ]------------------------------------------------",
            i
        );

        let w_count = wordlist.len();
        println!("\n{} candidate word{} left.", w_count, plural(w_count));

        let start = std::time::Instant::now();
        let candidates = wordlist.rank_words();
        let duration = start.elapsed();

        println!("\nTop candidate word{}:", plural(w_count));

        for (w, score) in candidates.take(10) {
            println!("{} ({})", w, score);
        }
        debug!("\nTime elapsed for word ranking: {:?}", duration);

        if wordlist.len() == 1 {
            println!("\nCongratulations! You won after {} round{}.", i, plural(i));
            break;
        }

        let mut word = get_user_word(i);

        while let Err(error) = word {
            println!("\nError: {}", error);
            word = get_user_word(i);
        }

        let mut constraints = get_contraints(word.as_ref().unwrap());

        while let Err(error) = constraints {
            println!("\nError: {}", error);
            constraints = get_contraints(word.as_ref().unwrap());
        }

        if constraints.as_ref().unwrap().correct_word() {
            println!("\nCongratulations! You won after {} round{}.", i, plural(i));
            break;
        }

        wordlist = Wordlist::from_iter(wordlist.filter(&constraints.unwrap()));
        wordlist.remove(word.as_ref().unwrap());

        if wordlist.len() > 1 && i == ROUND_NUM {
            println!("\n{} candidate words left.", wordlist.len());
            println!("\nGame over.");
            break;
        }

        if wordlist.is_empty() {
            println!("\nSomething went wrong. There are no matching words left.");
            break;
        }
    }
}

fn simulate(start: &Word, target: &Word) -> Option<usize> {
    let mut wordlist = Wordlist::load();

    debug!("{} -> {}", start, target);

    for i in 1..=ROUND_NUM {
        debug!(
            "\n---[ Round #{} ]------------------------------------------------",
            i
        );

        let w_count = wordlist.len();
        debug!("\n{} candidate word{} left.", w_count, plural(w_count));

        let w = match i {
            1 => start,
            _ => wordlist.rank_words().next().unwrap().0,
        };

        debug!("Top candidate word: {}", w);

        if wordlist.len() == 1 {
            debug!("\nI won after {} round{}.", i, plural(i));
            return Some(i);
        }

        let w_string = w.to_string();
        let color_code = w.match_code(target);
        debug!("Wordle hint: {}", color_code);

        let constraints = ConstraintSet::try_from((w_string.as_ref(), color_code.as_ref()));

        if constraints.as_ref().unwrap().correct_word() {
            debug!("\nI won after {} round{}.", i, plural(i));
            return Some(i);
        }

        wordlist = Wordlist::from_iter(wordlist.filter(&constraints.unwrap()));
        wordlist.remove(&w_string);

        if wordlist.len() > 1 && i == ROUND_NUM {
            debug!("\n{} candidate words left.", wordlist.len());
            debug!("\nGame over.");
            break;
        }
    }

    None
}

fn word_iter<'a>(
    word_opt: Option<&'a Word>,
    wordlist: &'a Wordlist,
) -> impl Iterator<Item = &'a Word> {
    let iter = if word_opt.is_none() {
        Some(wordlist.iter())
    } else {
        None
    };

    iter.into_iter().flatten().chain(word_opt)
}

fn simulate_all(start: Option<&String>, target: Option<&String>) {
    let wordlist = Wordlist::load();

    let start_word = start.map(Word::from);
    let start_words = word_iter(start_word.as_ref(), &wordlist);

    for s in start_words {
        let mut scores = Vec::with_capacity(wordlist.len());

        let target_word = target.map(Word::from);
        let target_words = word_iter(target_word.as_ref(), &wordlist);

        for t in target_words {
            if let Some(score) = simulate(s, t) {
                scores.push(score);
                info!("{} -> {}: Won after {} round{}", s, t, score, plural(score));
            } else {
                info!("{} -> {}: Lost", s, t);
            }
        }

        if !(start.is_some() && target.is_some()) {
            print_results(s, scores.iter().sum(), scores.len(), wordlist.len());
        }
    }
}

fn print_results(start_word: &Word, total_score: usize, won_count: usize, game_count: usize) {
    let won_percentage = won_count as f32 / game_count as f32 * 100.0;
    let avg_score = total_score as f32 / won_count as f32;

    println!(
        "With start word \"{}\", I won {} / {} games ({:.2} %) in on average {:.2} rounds.",
        start_word, won_count, game_count, won_percentage, avg_score
    )
}

fn user_input() -> String {
    let mut buffer = String::new();
    print!("> ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}

fn get_user_word(i: usize) -> Result<String, InputError> {
    println!(
        "\nPlease enter your {} word.",
        if i == 1 { "first" } else { "next" }
    );
    let word = user_input();

    if word.len() != WORD_LEN {
        return Err(InputError::IncorrectWordLength(WORD_LEN));
    }

    Ok(word)
}

fn get_contraints(word: &str) -> Result<ConstraintSet, InputError> {
    println!("\nPlease enter Wordle's answer. (G = Green, Y = Yellow, _ = Gray)");
    let colors = user_input();

    if colors.len() != WORD_LEN {
        return Err(InputError::IncorrectColorCodeLength(WORD_LEN));
    }

    ConstraintSet::try_from((word, colors.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_app() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
