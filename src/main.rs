use std::{
    collections::HashSet,
    error::Error,
    fmt,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    iter::FromIterator,
    path::Path,
};

use itertools::Itertools;

/// Number of rounds to play.
const ROUND_NUM: usize = 6;

fn main() {
    println!("Welcome! Let's play Wordle.");

    let mut wordlist = Wordlist::from("data/words.txt");

    for i in 1..=ROUND_NUM {
        println!(
            "\n---[ Round #{} ]------------------------------------------------",
            i
        );

        let w_count = wordlist.len();
        println!(
            "\n{} candidate word{} left.",
            w_count,
            if w_count == 1 { "" } else { "s" }
        );

        let start = std::time::Instant::now();
        let candidates = wordlist.rank_words();
        let duration = start.elapsed();

        println!(
            "\nTop candidate word{}:",
            if w_count == 1 { "" } else { "s" }
        );

        for (w, score) in candidates.take(10) {
            println!("{} ({})", w, score);
        }
        println!("\nTime elapsed for word ranking: {:?}", duration);

        if wordlist.len() == 1 {
            println!(
                "\nCongratulations! You won after {} round{}.",
                i,
                if i == 1 { "" } else { "s" }
            );
            break;
        }

        let mut constraints = get_contraints(i);

        while let Err(error) = constraints {
            println!("\nError: {}", error);
            constraints = get_contraints(i);
        }

        if constraints.as_ref().unwrap().correct_word() {
            println!(
                "\nCongratulations! You won after {} round{}.",
                i,
                if i == 1 { "" } else { "s" }
            );
            break;
        }

        wordlist = Wordlist::from_iter(wordlist.filter(&constraints.unwrap()));

        if wordlist.len() > 1 && i == ROUND_NUM {
            println!("\n{} candidate words left.", wordlist.len());
            println!("\nGame over.");
            break;
        }
    }
}

fn user_input() -> String {
    let mut buffer = String::new();
    print!("> ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}

fn get_contraints(i: usize) -> Result<ConstraintSet, InputError> {
    println!(
        "\nPlease enter your {} word.",
        if i == 1 { "first" } else { "next" }
    );
    let word = user_input();

    println!("\nPlease enter Wordle's answer. (G = Green, Y = Yellow, X = Gray)");
    let colors = user_input();

    ConstraintSet::try_from((word.as_ref(), colors.as_ref()))
}

#[derive(Debug)]
pub enum InputError {
    InvalidColorCode,
}

impl Error for InputError {}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InputError::InvalidColorCode => "Invalid color code",
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

impl Constraint {
    pub fn is_match(&self, word: &Word) -> bool {
        use Constraint::*;

        match self {
            AtPos(i, c) => word.char(*i) == *c,
            NotAtPos(i, c) => word.char(*i) != *c && word.contains(*c),
            Absent(c) => !word.contains(*c),
        }
    }

    pub fn matching_variant(w: &Word, i: usize, c: char) -> Constraint {
        if !w.contains(c) {
            Constraint::Absent(c)
        } else if w.char(i) == c {
            Constraint::AtPos(i, c)
        } else {
            Constraint::NotAtPos(i, c)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ConstraintSet(Vec<Constraint>);

impl ConstraintSet {
    pub fn is_match(&self, word: &Word) -> bool {
        self.0.iter().all(|c| c.is_match(word))
    }

    pub fn correct_word(&self) -> bool {
        self.0.iter().all(|c| matches!(c, Constraint::AtPos(_, _)))
    }
}

impl TryFrom<(&str, &str)> for ConstraintSet {
    type Error = InputError;

    fn try_from(input: (&str, &str)) -> Result<Self, Self::Error> {
        let (word, colors) = input;

        let mut constraints = vec![];

        let word = word.to_lowercase();
        let colors = colors.to_uppercase();

        let char_iter = word.chars().zip(colors.chars()).enumerate();

        for (i, (c, color)) in char_iter {
            let constraint = match color {
                'G' => Constraint::AtPos(i, c),
                'Y' => Constraint::NotAtPos(i, c),
                'X' => Constraint::Absent(c),
                _ => return Err(InputError::InvalidColorCode),
            };

            constraints.push(constraint);
        }

        Ok(Self(constraints))
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

    pub fn distinct_chars(&self) -> impl Iterator<Item = char> {
        self.chars().sorted().dedup()
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
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
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
            .sorted_by(|a, b| {
                (b.1, b.0.distinct_chars().count()).cmp(&(a.1, a.0.distinct_chars().count()))
            })
    }
}

impl<P: AsRef<Path>> From<P> for Wordlist {
    fn from(path: P) -> Self {
        let file = File::open(path).expect("file not found!");
        let reader = BufReader::new(file);

        let words = reader.lines().map(|w| Word::from(w.unwrap())).collect();

        Self(words)
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

    #[rstest(
        w,
        w_norm,
        case("about", "abotu"),
        case("itchy", "chity"),
        case("afoot", "afot"),
        case("alibi", "abil"),
        case("jazzy", "ajyz"),
        case("jewel", "ejlw")
    )]
    fn test_distinct_chars(w: &str, w_norm: &str) {
        let word = Word::from(w);

        assert_eq!(
            word.distinct_chars().collect::<Vec<_>>(),
            w_norm.chars().collect::<Vec<_>>()
        );
    }
}
