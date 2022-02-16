pub fn filter_potential(&self, wordlist: &Wordlist) -> usize {
    wordlist
        .iter()
        .map(|w| self.match_code(w))
        .sorted()
        .dedup()
        .count()

    //constraints.len()
}

pub fn match_codes(&self, w: &Word) -> (String, String) {
    self.chars()
        .zip(w.chars())
        .map(|(c1, c2)| {
            if c1 == c2 {
                ('G', 'G')
            } else {
                match (w.contains(c1), self.contains(c2)) {
                    (true, true) => ('Y', 'Y'),
                    (true, false) => ('Y', 'B'),
                    (false, true) => ('B', 'Y'),
                    (false, false) => ('B', 'B'),
                }
            }
        })
        .unzip()
}

pub fn rank_words(&self) -> impl Iterator<Item = (&Word, usize)> {
    let mut map = HashMap::new();

    for (i, w1) in self.iter().enumerate() {
        for (j, w2) in self.iter().enumerate() {
            if i < j {
                //let (code1, code2) = w1.match_codes(w2);
                let code1 = w1.match_code(w2);
                let code2 = w2.match_code(w1);

                let codes1 = map.entry(w1).or_insert_with(HashSet::new);
                codes1.insert(code1);

                let codes2 = map.entry(w2).or_insert_with(HashSet::new);
                codes2.insert(code2);
            }
        }
    }

    map.into_iter()
        .map(|(k, v)| (k, v.len()))
        .sorted_by(|a, b| b.1.cmp(&a.1))

    //self.iter()
    //    .map(|w| (w, w.filter_potential(self)))
    //    .sorted_by(|a, b| b.1.cmp(&a.1))
}
