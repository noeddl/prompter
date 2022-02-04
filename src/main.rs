use std::{
    collections::HashSet,
    error::Error,
    fmt,
    fs::File,
    io::{self, BufRead, BufReader},
    iter::FromIterator,
    path::Path,
};

use itertools::Itertools;

pub type Alphabet = HashSet<char>;

fn main() {
    println!("Welcome!");

    let mut wordlist = Wordlist::from("data/words.txt");

    loop {
        println!("\nPlease enter the next word.");
        let word = user_input();

        println!("\nPlease enter Wordle's answer.");
        let colors = user_input();

        let constraints = ConstraintSet::try_from((word.as_ref(), colors.as_ref()));
        println!("{:?}", constraints);

        wordlist = Wordlist::from_iter(wordlist.filter(&constraints.unwrap()));

        for word in &wordlist {
            println!("{}", word);
        }

        println!("{} candidate words left", wordlist.len());
    }
}

fn user_input() -> String {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
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

#[derive(Debug)]
pub enum Constraint {
    AtPos((usize, char)),
    NotAtPos((usize, char)),
    Absent(char),
}

impl Constraint {
    pub fn matches(&self, word: &Word) -> bool {
        use Constraint::*;

        match self {
            AtPos((i, c)) => word.char(*i) == *c,
            NotAtPos((i, c)) => word.char(*i) != *c && word.contains(*c),
            Absent(c) => !word.contains(*c),
        }
    }
}

#[derive(Debug)]
pub struct ConstraintSet(Vec<Constraint>);

impl ConstraintSet {
    pub fn matches(&self, word: &Word) -> bool {
        self.0.iter().all(|c| c.matches(word))
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
                'G' => Constraint::AtPos((i, c)),
                'Y' => Constraint::NotAtPos((i, c)),
                'B' => Constraint::Absent(c),
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

    pub fn is_heterogram(&self) -> bool {
        self.distinct_chars().count() == self.0.len()
    }

    pub fn matches(&self, constraint: Constraint) -> bool {
        use Constraint::*;

        match constraint {
            AtPos((i, c)) => self.char(i) == c,
            NotAtPos((i, c)) => self.char(i) != c && self.contains(c),
            Absent(c) => !self.contains(c),
        }
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

    pub fn alphabet(&self) -> Alphabet {
        self.iter().flat_map(|w| w.chars()).collect()
    }

    pub fn filter(self, constraints: &ConstraintSet) -> impl Iterator<Item = Word> + '_ {
        self.into_iter().filter(|w| constraints.matches(w))
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

    #[rstest(
        w,
        is_heterogram,
        case("about", true),
        case("itchy", true),
        case("afoot", false),
        case("alibi", false),
        case("jazzy", false),
        case("jewel", false)
    )]
    fn test_is_heterogram(w: &str, is_heterogram: bool) {
        let word = Word::from(w);

        assert_eq!(word.is_heterogram(), is_heterogram);
    }
}
