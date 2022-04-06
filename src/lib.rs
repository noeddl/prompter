use std::{
    collections::HashSet,
    error::Error,
    fmt,
    fs::File,
    io::{BufRead, BufReader},
    iter::FromIterator,
    path::Path,
};

use itertools::Itertools;

#[derive(Debug)]
pub enum InputError {
    InvalidColorCode(char),
    IncorrectWordLength(usize),
    IncorrectColorCodeLength(usize),
}

impl Error for InputError {}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use InputError::*;

        let s = match self {
            InvalidColorCode(c) => format!("Invalid color code character '{}'", c),
            IncorrectWordLength(len) => format!("Word must be {} characters long", len),
            IncorrectColorCodeLength(len) => format!("Color code must be {} characters long", len),
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
        include_str!("words.txt").lines().map(Word::from).collect()
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
