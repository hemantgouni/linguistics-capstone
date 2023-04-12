#![allow(dead_code)]

use rand::{rngs::StdRng, Rng, SeedableRng};
use similar::{DiffOp, TextDiff};
use unicode_segmentation::UnicodeSegmentation;

// string -> syllabified candidate -> random deletions (all winners generated via deletions) ->
// eval against constraints

const VOWELS: [&'static str; 7] = ["o", "ɛ", "ɔ", "i", "u", "a", "e"];

#[derive(Debug, Clone)]
struct SyllabifiedCandidate {
    form: Vec<Segment>,
    rng: StdRng,
}

#[derive(Debug, Clone)]
struct Segment {
    char: String,
    char_type: SegmentType,
}

#[derive(Debug, Clone)]
enum SegmentType {
    Onset,
    Vowel,
    Coda,
}

// fn syllabify()

#[derive(Debug, Clone)]
struct Candidate {
    form: String,
    rng: StdRng,
}

impl Candidate {
    fn indices(&self) -> Vec<usize> {
        self.form
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .collect::<Vec<usize>>()
    }

    // &mut because we want to change the rng when we delete
    fn delete(&mut self) -> Self {
        let mut self_copy = self.clone();

        let index = if self_copy.indices().len() > 0 {
            self_copy.indices()[self.rng.gen_range(0..self_copy.indices().len())]
        } else {
            0
        };

        self_copy.form = self_copy
            .form
            .grapheme_indices(true)
            .filter(|(char_index, _)| *char_index != index)
            .map(|(_, str)| str)
            .collect::<String>();

        self_copy
    }
}

trait Constraint {
    fn evaluate(self, surface: Candidate) -> usize;
}

struct Ident(Candidate);

impl Constraint for Ident {
    fn evaluate(self, surface: Candidate) -> usize {
        let diff = TextDiff::from_graphemes(&self.0.form, &surface.form);

        diff.ops()
            .iter()
            .filter(|op| match op {
                DiffOp::Replace { .. } => true,
                _ => false,
            })
            .count()
    }
}

struct Dep(Candidate);

impl Constraint for Dep {
    fn evaluate(self, surface: Candidate) -> usize {
        let diff = TextDiff::from_graphemes(&self.0.form, &surface.form);

        diff.ops()
            .iter()
            .filter(|op| match op {
                DiffOp::Insert { .. } => true,
                _ => false,
            })
            .count()
    }
}

struct Onset(Candidate);

// impl Constraint for Onset {
//     fn evaluate(self, surface: Candidate) -> usize {
//         let diff = TextDiff::from_graphemes(&self.0.form, &surface.form);

//     }
// }

fn main() {
    let cand1 = Candidate {
        form: "owókíowó".to_string(),
        rng: StdRng::seed_from_u64(7777777),
    };
    let cand2 = Candidate {
        form: "ówakíówó".to_string(),
        rng: StdRng::seed_from_u64(7777777),
    };
    let ident = Ident(cand1);
    println!("{:#?}", ident.evaluate(dbg!(cand2)));
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_delete_1() {
        let cand = Candidate {
            form: "test".to_string(),
            rng: StdRng::seed_from_u64(7777777),
        }
        .delete();

        assert_eq!(cand.form, "tst");
    }

    #[test]
    fn test_delete_2() {
        let cand = Candidate {
            form: "test".to_string(),
            rng: StdRng::seed_from_u64(7777777),
        }
        .delete()
        .delete()
        .delete()
        .delete()
        .delete();

        assert_eq!(cand.form, "");
    }

    #[test]
    fn test_delete_3() {
        let cand = Candidate {
            form: "owókíowó".to_string(),
            rng: StdRng::seed_from_u64(7777777),
        }
        .delete()
        .delete()
        .delete()
        .delete();

        assert_eq!(cand.form, "owwó");
    }

    #[test]
    fn test_delete_4() {
        let cand = Candidate {
            form: "owókíowó".to_string(),
            rng: StdRng::seed_from_u64(7777777),
        }
        .delete()
        .delete()
        .delete()
        .delete()
        .delete()
        .delete()
        .delete()
        .delete();

        assert_eq!(cand.form, "");
    }

    #[test]
    fn test_ident_1() {
        let cand1 = Candidate {
            form: "owókíowó".to_string(),
            rng: StdRng::seed_from_u64(7777777),
        };

        let cand2 = Candidate {
            form: "ówakíówó".to_string(),
            rng: StdRng::seed_from_u64(7777777),
        };

        let ident = Ident(cand1);

        assert_eq!(ident.evaluate(cand2), 3);
    }
}
