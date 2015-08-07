#![feature(test)]

/// The beginnings of a fuzzy matcher library. The algorithm is heavily inspired
/// by YouCompleteMe by Val Markovic.

extern crate test;
extern crate bit_set;

use bit_set::BitSet;
use std::collections::HashSet;
use std::ascii::AsciiExt;
use std::cmp;
use std::hash;

const NUM_CHARS: u8 = 127;

// TODO(sirver): YCM's heuristics are more powerful than what we have implemented here. But this is
// a shitty first draft that is enough to outline the functionality I want.

pub struct Candidate {
    text: String,
    query_bitset: BitSet,
}

impl cmp::PartialEq for Candidate {
    fn eq(&self, other: &Candidate) -> bool {
        self.text == other.text
    }
}

impl cmp::Eq for Candidate {}

impl hash::Hash for Candidate {
   fn hash<H>(&self, state: &mut H) where H: hash::Hasher {
       self.text.hash(state)
   }
}

impl Candidate {
    pub fn new(text: &str) -> Self {
        Candidate {
            text: text.to_string(),
            query_bitset: make_query_bitset(text),
        }
    }

    fn matches_query_bitset(&self, bitset: &BitSet) -> bool {
        bitset.is_subset(&self.query_bitset)
    }
}

#[derive(Debug)]
pub struct QueryResult<'a> {
    text: &'a str,
    score: usize,
}

impl<'a> cmp::PartialEq for QueryResult<'a> {
    fn eq(&self, other: &QueryResult) -> bool {
        self.score == other.score
    }
}

impl<'a> cmp::Eq for QueryResult<'a> {}

impl<'a> cmp::PartialOrd for QueryResult<'a> {
      fn partial_cmp(&self, other: &QueryResult) -> Option<cmp::Ordering> {
          self.score.partial_cmp(&other.score)
      }
}

impl<'a> cmp::Ord for QueryResult<'a> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.score.cmp(&other.score)
    }
}


pub struct CandidateSet {
    candidates: HashSet<Candidate>,
}

impl CandidateSet {
    pub fn new() -> Self {
        CandidateSet {
            candidates: HashSet::new(),
        }
    }

    pub fn insert(&mut self, text: &str) {
        self.candidates.insert(Candidate::new(text));
    }

    pub fn query<'a>(&'a self, query: &str, match_case: MatchCase) -> Vec<QueryResult<'a>> {
        let query_bitset = make_query_bitset(query);

        let mut results = Vec::new();
        for candidate in &self.candidates {
            if !candidate.matches_query_bitset(&query_bitset) {
                continue;
            }

            if let Some(score) = is_subsequence(&candidate.text, query, match_case) {
                results.push(QueryResult {
                    text: &candidate.text,
                    score: score,
                })
            }
        }
        results.sort();
        results
    }

}

pub fn letter_to_index(letter: u8) -> usize {
    (letter % NUM_CHARS) as usize
}

pub fn make_query_bitset(s: &str) -> BitSet {
    let mut bitset = BitSet::with_capacity(NUM_CHARS as usize);
    for c in s.chars() {
        if !c.is_ascii() {
            continue;
        }
        bitset.insert(letter_to_index(c.to_ascii_lowercase() as u8));
    }
    bitset
}

/// MatchCase when comparing strings or not.
#[derive(Clone,Copy)]
pub enum MatchCase {
    Yes,
    No,
}

/// Returns true if `a` is a subseqence of `b`. Returns a value to rate the match,
/// higher is worse or None if there was no match.
// TODO(sirver): This is kinda the first algorithm I came up with. YCM seems to be
// doing something more sophisticated which is likely faster.
pub fn is_subsequence(candidate: &str, query: &str, match_case: MatchCase) -> Option<usize> {
    let mut score = 0;
    let mut query_iter = query.chars().peekable();
    for (index, c) in candidate.chars().enumerate() {
        if !c.is_ascii() {
            continue;
        }
        let advance = match query_iter.peek() {
            Some(q) => match match_case {
                MatchCase::Yes => *q == c,
                MatchCase::No => q.to_ascii_lowercase() == c.to_ascii_lowercase(),
            },
            None => return Some(score),
        };
        if advance {
            score += index;
            query_iter.next();
        }
    }
    match query_iter.peek() {
        Some(_) => None,
        None => Some(score),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_query_bitset() {
        let candidate = Candidate::new("foobaaar");

        assert!(candidate.matches_query_bitset(&make_query_bitset("foobaar")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("foobaaar")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("fobar")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("rabof")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("bfroa")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("fbr")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("r")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("bbb")));
        assert!(candidate.matches_query_bitset(&make_query_bitset("")));

        assert!(!candidate.matches_query_bitset(&make_query_bitset("foobare")));
        assert!(!candidate.matches_query_bitset(&make_query_bitset("gggg")));
        assert!(!candidate.matches_query_bitset(&make_query_bitset("x")));
        assert!(!candidate.matches_query_bitset(&make_query_bitset("nfoobar")));
        assert!(!candidate.matches_query_bitset(&make_query_bitset("fbrmmm")));
    }

    #[bench]
    fn bench_query_bitset(b: &mut Bencher) {
        let candidate = Candidate::new("foobaaar");
        let to_test = make_query_bitset("foobaaar");
        b.iter(|| {
            assert!(candidate.matches_query_bitset(&to_test));
        })
    }

    #[bench]
    fn bench_make_query_bitset(b: &mut Bencher) {
        b.iter(|| {
            make_query_bitset("fooobaaaaraaaarara");
        })
    }

    #[test]
    fn test_is_subsequence() {
        let candidate = "foobaaar";
        assert!(is_subsequence(candidate, "foobar", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "foobaaar", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "foOBAaar", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "FOOBAAAR", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "fobar", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "fbr", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "f", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "F", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "o", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "O", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "a", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "r", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "b", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "bar", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "oa", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "obr", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "oar", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "oo", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "aaa", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "AAA", MatchCase::No).is_some());
        assert!(is_subsequence(candidate, "", MatchCase::No).is_some());
    }

    #[test]
    fn test_is_not_subsequence() {
        let candidate = "foobaaar";
        assert!(is_subsequence(candidate, "foobra", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "frb", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "brf", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "x", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "9", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "-", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "~", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, " ", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "rabof", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "oabfr", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "ooo", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "baaara", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "ffoobaaar", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "xfoobaaar", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, " foobaaar", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "foobaaar ", MatchCase::No).is_none());
        assert!(is_subsequence(candidate, "ff", MatchCase::No).is_none());
    }

    #[test]
    fn smoke_test_candidate_set() {
        let mut candidates = CandidateSet::new();

        candidates.insert("FooBarBlub");
        candidates.insert("foobarblub");
        candidates.insert("surpriseExtreem");
        candidates.insert("barblub");

        {
            let results = candidates.query("fbb", MatchCase::No);
            assert_eq!(2, results.len());
        }

        {
            let results = candidates.query("bb", MatchCase::No);
            assert_eq!(3, results.len());
            assert_eq!("barblub", results[0].text);
        }

        {
            let results = candidates.query("sxee", MatchCase::No);
            assert_eq!(1, results.len());
        }
    }
}
