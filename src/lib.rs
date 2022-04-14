//! `prompter` is a command line tool that helps you choose the next word in a game
//! of [Wordle](https://www.nytimes.com/games/wordle/index.html) - just like a promper
//! in a theater tells the actors what to say next in case they forget.
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
/// Error type to handle errors in the user's input
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
/// A constraint that encodes information about a given character and a given position in a [`Word`]
pub enum Constraint {
    /// The given character is at the given position.
    AtPos(usize, char),
    /// The given character is *not* at the given position but somewhere else in the word.
    NotAtPos(usize, char),
    /// The given character is not in the word at any position.
    Absent(char),
}

#[derive(Debug, PartialEq, Eq, Hash)]
/// A set of [`Constraint`]s that can be used to filter the [`Word`]s in a [`Wordlist`]
pub struct ConstraintSet {
    /// Set of constraints. Each index in the `Vec` corresponds to a position in the word.
    constraints: Vec<Constraint>,
    /// List of characters that have been found to be present in the word.
    present_chars: Vec<char>,
}

impl ConstraintSet {
    /// Returns an iterator over the constraints in the set.
    pub fn iter(&self) -> ::std::slice::Iter<Constraint> {
        self.constraints.iter()
    }

    #[allow(clippy::needless_collect)]
    /// Returns true if the given `word` complies to all the constraints in the set.
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

    /// Returns `true` if the `ConstraintSet` encodes a correct guess, i.e. all the characters
    /// are at the correct position (corresponds to the code `GGGGG`).
    pub fn correct_word(&self) -> bool {
        self.iter().all(|c| matches!(c, Constraint::AtPos(_, _)))
    }
}

impl TryFrom<(&str, &str)> for ConstraintSet {
    type Error = InputError;

    /// Try to create a `ConstraintSet` from an input word and string representing a color code.
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
                '_' => Constraint::Absent(c),
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
/// A candidate or mystery word in a Wordle game
pub struct Word(String);

impl Word {
    /// Returns `true` if the word contains the given character.
    pub fn contains(&self, c: char) -> bool {
        self.0.contains(c)
    }

    /// Returns the the character at the given `index` in the word.
    pub fn char(&self, index: usize) -> char {
        self.0.chars().nth(index).unwrap()
    }

    /// Returns an iterator over the characters in the word.
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0.chars()
    }

    /// Returns a string representing the color code that Wordle would present
    /// for a target word `w`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prompter::Word;
    /// let w1 = Word::from("crate");
    /// let w2 = Word::from("space");
    ///
    /// assert_eq!(w1.match_code(&w2), "Y_G_G");
    /// assert_eq!(w2.match_code(&w1), "__GYG");
    /// ```
    pub fn match_code(&self, w: &Word) -> String {
        self.chars()
            .zip(w.chars())
            .map(|(c1, c2)| {
                if c1 == c2 {
                    'G'
                } else if w.contains(c1) {
                    'Y'
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Computes the number of different color codes that are assigned to the `Word`
    /// when matched against every other word in the wordlist.
    pub fn filter_potential(&self, wordlist: &Wordlist) -> usize {
        let constraints: HashSet<_> = wordlist.iter().map(|w| self.match_code(w)).collect();

        constraints.len()
    }
}

impl<S: AsRef<str>> From<S> for Word {
    /// Creates a `Word` from a type that can automatically be dereferenced into a `str`.
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
/// A list of [`Word`]s
pub struct Wordlist(Vec<Word>);

impl Wordlist {
    /// Loads the default wordlist from a file.
    pub fn load() -> Self {
        include_str!("words.txt").lines().map(Word::from).collect()
    }

    /// Returns the number of words in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if no words are in the list.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over references to the words in the list.
    pub fn iter(&self) -> ::std::slice::Iter<Word> {
        self.0.iter()
    }

    /// Returns an iterator over the words in the list that comply to the given `constraints`.
    pub fn filter(self, constraints: &ConstraintSet) -> impl Iterator<Item = Word> + '_ {
        self.into_iter().filter(|w| constraints.is_match(w))
    }

    /// Ranks the words in the list by their [`filter_potential`] and returns an iterator
    /// over pairs of word references and scores. The return values are sorted by the score
    /// in descending order. Two words with the same score will be sorted lexicographically.
    ///
    /// [`filter_potential`]: Word::filter_potential
    pub fn rank_words(&self) -> impl Iterator<Item = (&Word, usize)> {
        self.iter()
            .map(|w| (w, w.filter_potential(self)))
            .sorted_by(|a, b| (b.1).cmp(&a.1))
    }

    /// Removes the given `word` from the list if it exists.
    pub fn remove(&mut self, word: &str) {
        if let Some(index) = self.iter().position(|w| w.0 == word) {
            self.0.remove(index);
        }
    }
}

impl<P: AsRef<Path>> From<P> for Wordlist {
    /// Loads a wordlist from a text file.
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
        case("abcde", "_____", "fghij", true),
        case("choir", "____Y", "wrung", true),
        case("child", "_YYY_", "light", true),
        case("stole", "YYG_G", "those", true),
        case("raise", "__GG_", "moist", true),
        case("slate", "_GYYY", "pleat", true),
        case("blast", "_GY_G", "aloft", true),
        case("raise", "Y___Y", "elder", true),
        case("brink", "YYYY_", "robin", true),
        case("phase", "_GGYG", "shake", true),
        case("armor", "GGYY_", "aroma", true),
        case("canal", "GG__Y", "caulk", true),
        case("robot", "YY__Y", "thorn", true),
        case("nylon", "___YG", "thorn", true),
        case("tacit", "G____", "thorn", true),
        case("crate", "__YG_", "haste", false)
    )]
    fn test_is_match(input: &str, code: &str, target: &str, is_match: bool) {
        let constraint_set = ConstraintSet::try_from((input, code)).unwrap();

        assert_eq!(constraint_set.is_match(&Word::from(target)), is_match);
    }
}
