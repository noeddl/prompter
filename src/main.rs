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
        println!("{} candidate words left", wordlist.len());

        let start = std::time::Instant::now();

        for (w, score) in wordlist.rank_words().take(10) {
            println!("{} ({})", w, score);
        }

        let duration = start.elapsed();
        println!("Time elapsed for word ranking: {:?}", duration);

        println!("\nPlease enter the next word.");
        let word = user_input();

        println!("\nPlease enter Wordle's answer.");
        let colors = user_input();

        let constraints = ConstraintSet::try_from((word.as_ref(), colors.as_ref()));
        println!("{:?}", constraints);

        wordlist = Wordlist::from_iter(wordlist.filter(&constraints.unwrap()));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Constraint {
    AtPos((usize, char)),
    NotAtPos((usize, char)),
    Absent(char),
}

impl Constraint {
    pub fn values(i: usize, c: char) -> impl Iterator<Item = Constraint> {
        [
            Constraint::AtPos((i, c)),
            Constraint::NotAtPos((i, c)),
            Constraint::Absent(c),
        ]
        .into_iter()
    }

    pub fn is_match(&self, word: &Word) -> bool {
        use Constraint::*;

        match self {
            AtPos((i, c)) => word.char(*i) == *c,
            NotAtPos((i, c)) => word.char(*i) != *c && word.contains(*c),
            Absent(c) => !word.contains(*c),
        }
    }

    pub fn matching_variant(w: &Word, i: usize, c: char) -> Constraint {
        if !w.contains(c) {
            Constraint::Absent(c)
        } else if w.char(i) == c {
            Constraint::AtPos((i, c))
        } else {
            Constraint::NotAtPos((i, c))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ConstraintSet(Vec<Constraint>);

impl ConstraintSet {
    pub fn is_match(&self, word: &Word) -> bool {
        self.0.iter().all(|c| c.is_match(word))
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

    pub fn all_constraints(&self) -> impl Iterator<Item = impl Iterator<Item = Constraint>> + '_ {
        self.0.char_indices().map(|(i, c)| Constraint::values(i, c))
    }

    pub fn all_constraint_combinations(&self) -> impl Iterator<Item = Vec<Constraint>> {
        self.all_constraints()
            .map(|iter| iter.collect::<Vec<_>>())
            .collect::<Vec<_>>()
            .into_iter()
            .multi_cartesian_product()
    }

    pub fn matching_constraint_set(&self, w: &Word) -> ConstraintSet {
        let vec: Vec<_> = self
            .0
            .char_indices()
            .map(|(i, c)| Constraint::matching_variant(w, i, c))
            .collect();

        ConstraintSet(vec)
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
                    'B'
                }
            })
            .collect()
    }

    pub fn filter_potential(&self, wordlist: &Wordlist) -> usize {
        let constraints: HashSet<_> = wordlist.iter().map(|w| self.match_code(w)).collect();

        constraints.len() + self.distinct_chars().count()
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
        self.into_iter().filter(|w| constraints.is_match(w))
    }

    pub fn filter_ref<'a>(
        &'a self,
        constraints: &'a ConstraintSet,
    ) -> impl Iterator<Item = &'a Word> + '_ {
        self.iter().filter(|w| constraints.is_match(w))
    }

    pub fn best_next_word(&self) -> Option<(&Word, usize)> {
        self.iter()
            .map(|w| (w, w.filter_potential(self)))
            .max_by(|a, b| a.1.cmp(&b.1))
    }

    pub fn rank_words(&self) -> impl Iterator<Item = (&Word, usize)> {
        self.iter()
            .map(|w| (w, w.filter_potential(self)))
            .sorted_by(|a, b| b.1.cmp(&a.1))
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

    #[test]
    fn test_all_constraints() {
        use Constraint::*;

        let word = Word::from("hej");

        let obs: Vec<Vec<_>> = word.all_constraints().map(|iter| iter.collect()).collect();

        let exp = vec![
            vec![AtPos((0, 'h')), NotAtPos((0, 'h')), Absent('h')],
            vec![AtPos((1, 'e')), NotAtPos((1, 'e')), Absent('e')],
            vec![AtPos((2, 'j')), NotAtPos((2, 'j')), Absent('j')],
        ];

        assert_eq!(obs, exp);
    }

    #[test]
    fn test_all_constraint_combinations() {
        use Constraint::*;

        let word = Word::from("hi");

        let exp = vec![
            vec![AtPos((0, 'h')), AtPos((1, 'i'))],
            vec![AtPos((0, 'h')), NotAtPos((1, 'i'))],
            vec![AtPos((0, 'h')), Absent('i')],
            vec![NotAtPos((0, 'h')), AtPos((1, 'i'))],
            vec![NotAtPos((0, 'h')), NotAtPos((1, 'i'))],
            vec![NotAtPos((0, 'h')), Absent('i')],
            vec![Absent('h'), AtPos((1, 'i'))],
            vec![Absent('h'), NotAtPos((1, 'i'))],
            vec![Absent('h'), Absent('i')],
        ];

        assert_eq!(word.all_constraint_combinations().collect::<Vec<_>>(), exp);
    }
}
