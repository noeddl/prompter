use std::{
    collections::HashSet,
    error::Error,
    fmt,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    iter::FromIterator,
    path::Path,
};

use clap::{Parser, Subcommand};
use env_logger::{Builder, Target};
use itertools::Itertools;
use log::{debug, info, LevelFilter};

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

        let color_code = w.match_code(target);

        let constraints = ConstraintSet::try_from((w.0.as_ref(), color_code.as_ref()));

        if constraints.as_ref().unwrap().correct_word() {
            debug!("\nI won after {} round{}.", i, plural(i));
            return Some(i);
        }

        wordlist = Wordlist::from_iter(wordlist.filter(&constraints.unwrap()));

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

    iter.into_iter().flatten().chain(word_opt.into_iter())
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
        return Err(InputError::IncorrectWordLength);
    }

    Ok(word)
}

fn get_contraints(word: &str) -> Result<ConstraintSet, InputError> {
    println!("\nPlease enter Wordle's answer. (G = Green, Y = Yellow, X = Gray)");
    let colors = user_input();

    if colors.len() != WORD_LEN {
        return Err(InputError::IncorrectColorCodeLength);
    }

    ConstraintSet::try_from((word, colors.as_ref()))
}

#[derive(Debug)]
pub enum InputError {
    InvalidColorCode(char),
    IncorrectWordLength,
    IncorrectColorCodeLength,
}

impl Error for InputError {}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use InputError::*;

        let s = match self {
            InvalidColorCode(c) => format!("Invalid color code character '{}'", c),
            IncorrectWordLength => format!("Word must be {} characters long", WORD_LEN),
            IncorrectColorCodeLength => format!("Color code must be {} characters long", WORD_LEN),
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Constraint {
    AtPos(usize, char),
    NotAtPos(usize, char),
    Absent(char),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ConstraintSet {
    constraints: Vec<Constraint>,
    present_chars: Vec<char>,
}

impl ConstraintSet {
    pub fn iter(&self) -> ::std::slice::Iter<Constraint> {
        self.constraints.iter()
    }

    #[allow(clippy::needless_collect)]
    pub fn is_match(&self, word: &Word) -> bool {
        use Constraint::*;

        let chars: Vec<_> = word
            .chars()
            .filter(|c| !self.present_chars.contains(c))
            .collect();

        for constraint in self {
            let is_match = match constraint {
                AtPos(i, c) => word.char(*i) == *c,
                NotAtPos(i, c) => word.char(*i) != *c && word.contains(*c),
                Absent(c) => !chars.contains(c),
            };

            if !is_match {
                return false;
            }
        }

        true
    }

    pub fn correct_word(&self) -> bool {
        self.iter().all(|c| matches!(c, Constraint::AtPos(_, _)))
    }
}

impl TryFrom<(&str, &str)> for ConstraintSet {
    type Error = InputError;

    fn try_from(input: (&str, &str)) -> Result<Self, Self::Error> {
        let (word, colors) = input;

        let mut constraints = vec![];
        let mut present_chars = vec![];

        let word = word.to_lowercase();
        let colors = colors.to_uppercase();

        let char_iter = word.chars().zip(colors.chars()).enumerate();

        for (i, (c, color)) in char_iter {
            let constraint = match color {
                'G' => {
                    present_chars.push(c);
                    Constraint::AtPos(i, c)
                }
                'Y' => {
                    present_chars.push(c);
                    Constraint::NotAtPos(i, c)
                }
                'X' => Constraint::Absent(c),
                c => return Err(InputError::InvalidColorCode(c)),
            };

            constraints.push(constraint);
        }

        Ok(Self {
            constraints,
            present_chars,
        })
    }
}

impl IntoIterator for ConstraintSet {
    type Item = Constraint;
    type IntoIter = ::std::vec::IntoIter<Constraint>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConstraintSet {
    type Item = &'a Constraint;
    type IntoIter = ::std::slice::Iter<'a, Constraint>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Word(String);

impl Word {
    pub fn contains(&self, c: char) -> bool {
        self.0.contains(c)
    }

    pub fn char(&self, index: usize) -> char {
        self.0.chars().nth(index).unwrap()
    }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0.chars()
    }

    pub fn match_code(&self, w: &Word) -> String {
        self.chars()
            .zip(w.chars())
            .map(|(c1, c2)| {
                if c1 == c2 {
                    'G'
                } else if w.contains(c1) {
                    'Y'
                } else {
                    'X'
                }
            })
            .collect()
    }

    pub fn filter_potential(&self, wordlist: &Wordlist) -> usize {
        let constraints: HashSet<_> = wordlist.iter().map(|w| self.match_code(w)).collect();

        constraints.len()
    }
}

impl<S: AsRef<str>> From<S> for Word {
    fn from(s: S) -> Self {
        Self(s.as_ref().to_string())
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Default)]
pub struct Wordlist(Vec<Word>);

impl Wordlist {
    pub fn load() -> Self {
        let words = include_str!("words.txt");

        words.lines().map(Word::from).collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> ::std::slice::Iter<Word> {
        self.0.iter()
    }

    pub fn filter(self, constraints: &ConstraintSet) -> impl Iterator<Item = Word> + '_ {
        self.into_iter().filter(|w| constraints.is_match(w))
    }

    pub fn rank_words(&self) -> impl Iterator<Item = (&Word, usize)> {
        self.iter()
            .map(|w| (w, w.filter_potential(self)))
            .sorted_by(|a, b| (b.1).cmp(&a.1))
    }

    pub fn remove(&mut self, word: &str) {
        if let Some(index) = self.iter().position(|w| w.0 == word) {
            self.0.remove(index);
        }
    }
}

impl<P: AsRef<Path>> From<P> for Wordlist {
    fn from(path: P) -> Self {
        let file = File::open(path).expect("file not found!");
        let reader = BufReader::new(file);

        reader.lines().map(|w| Word::from(w.unwrap())).collect()
    }
}

impl FromIterator<Word> for Wordlist {
    fn from_iter<I: IntoIterator<Item = Word>>(iter: I) -> Self {
        let mut wordlist = Wordlist::default();

        for w in iter {
            wordlist.0.push(w);
        }

        wordlist
    }
}

impl IntoIterator for Wordlist {
    type Item = Word;
    type IntoIter = ::std::vec::IntoIter<Word>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Wordlist {
    type Item = &'a Word;
    type IntoIter = ::std::slice::Iter<'a, Word>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn verify_app() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }

    #[rstest(
        input,
        code,
        target,
        is_match,
        case("words", "GGGGG", "words", true),
        case("abcde", "XXXXX", "fghij", true),
        case("choir", "XXXXY", "wrung", true),
        case("child", "XYYYX", "light", true),
        case("stole", "YYGXG", "those", true),
        case("raise", "XXGGX", "moist", true),
        case("slate", "XGYYY", "pleat", true),
        case("blast", "XGYXG", "aloft", true),
        case("raise", "YXXXY", "elder", true),
        case("brink", "YYYYX", "robin", true),
        case("phase", "XGGYG", "shake", true),
        case("armor", "GGYYX", "aroma", true),
        case("canal", "GGXXY", "caulk", true),
        case("robot", "YYXXY", "thorn", true),
        case("nylon", "XXXYG", "thorn", true),
        case("tacit", "GXXXX", "thorn", true),
        case("crate", "XXYGX", "haste", false)
    )]
    fn test_is_match(input: &str, code: &str, target: &str, is_match: bool) {
        let constraint_set = ConstraintSet::try_from((input, code)).unwrap();

        assert_eq!(constraint_set.is_match(&Word::from(target)), is_match);
    }
}
