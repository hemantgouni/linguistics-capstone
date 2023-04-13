#![allow(dead_code)]

mod utils;

use rand::{rngs::StdRng, Rng, SeedableRng};
use similar::{DiffOp, TextDiff};
use unicode_segmentation::UnicodeSegmentation;

use utils::PushRet;

// string -> syllabified candidate -> random deletions (all winners generated via deletions) ->
// eval against constraints

const VOWELS: [&str; 7] = ["o", "ɛ", "ɔ", "i", "u", "a", "e"];

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

#[derive(Debug, Clone, Eq, PartialEq)]
enum SegmentType {
    Onset,
    Nucleus,
    Coda,
    None,
}

impl SyllabifiedCandidate {
    fn delete(mut self) -> Self {
        if !self.form.is_empty() {
            self.form.remove(self.rng.gen_range(0..self.form.len()));
        }
        self
    }
}

impl From<&str> for SyllabifiedCandidate {
    fn from(str: &str) -> SyllabifiedCandidate {
        SyllabifiedCandidate {
            form: syllabify(
                str.graphemes(true)
                    .map(|grapheme| Segment {
                        char: grapheme.to_owned(),
                        char_type: SegmentType::None,
                    })
                    .collect(),
            ),
            rng: StdRng::seed_from_u64(7777777),
        }
    }
}

impl From<SyllabifiedCandidate> for String {
    fn from(cand: SyllabifiedCandidate) -> String {
        cand.form.iter().map(|seg| seg.char.to_owned()).collect()
    }
}

impl Segment {
    fn morph_type(&self, seg_type: SegmentType) -> Segment {
        Segment {
            char: self.char.clone(),
            char_type: seg_type,
        }
    }
}

fn mark_vowels(candidate: Vec<Segment>) -> Vec<Segment> {
    // mark vowels
    // then extend onsets before vowels
    // then mark codas in remaining spaces after vowels
    // all segments should now be marked as onset, vowel, or coda

    candidate
        .iter()
        .map(|segment| {
            if VOWELS.contains(&segment.char.as_str()) {
                segment.morph_type(SegmentType::Nucleus)
            } else {
                segment.clone()
            }
        })
        .collect()
}

fn mark_onsets(candidate: Vec<Segment>) -> Vec<Segment> {
    let mut annotated: (SegmentType, Vec<Segment>) = candidate.iter().rev().fold(
        (SegmentType::None, Vec::new()),
        |(prev_seg_type, mut segs), seg| match (prev_seg_type, seg.to_owned().char_type) {
            (SegmentType::Nucleus, SegmentType::None) => (
                SegmentType::Onset,
                segs.push_ret(seg.morph_type(SegmentType::Onset)).to_owned(),
            ),
            _ => (
                seg.char_type.to_owned(),
                segs.push_ret(seg.to_owned()).to_owned(),
            ),
        },
    );

    annotated.1.reverse();

    annotated.1
}

fn mark_codas(candidate: Vec<Segment>) -> Vec<Segment> {
    candidate
        .iter()
        .map(|seg| match seg.char_type {
            SegmentType::None => seg.morph_type(SegmentType::Coda),
            _ => seg.to_owned(),
        })
        .collect()
}

fn syllabify(candidate: Vec<Segment>) -> Vec<Segment> {
    mark_codas(mark_onsets(mark_vowels(candidate)))
}

trait Constraint {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize;
}

struct Ident(SyllabifiedCandidate);

impl Constraint for Ident {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize {
        let self_str: String = self.0.into();
        let surface_str: String = surface.into();

        let diff = TextDiff::from_graphemes::<String>(&self_str, &surface_str);

        diff.ops()
            .iter()
            .filter(|op| matches!(op, DiffOp::Replace { .. }))
            .count()
    }
}

struct Dep(SyllabifiedCandidate);

impl Constraint for Dep {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize {
        let self_str: String = self.0.into();
        let surface_str: String = surface.into();

        let diff = TextDiff::from_graphemes::<String>(&self_str, &surface_str);

        diff.ops()
            .iter()
            .filter(|op| matches!(op, DiffOp::Insert { .. }))
            .count()
    }
}

struct Onset;

impl Constraint for Onset {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize {
        let syllabi = surface
            .form
            .iter()
            .filter(|seg| seg.char_type == SegmentType::Nucleus)
            .count();

        let onsets = surface
            .form
            .iter()
            .filter(|seg| seg.char_type == SegmentType::Onset)
            .count();

        onsets - syllabi
    }
}

fn main() {
    let syllabified_candidate: SyllabifiedCandidate = "vowels".into();
    dbg!(syllabified_candidate);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_delete_1() {
        let cand: SyllabifiedCandidate = "test".into();

        let cand_str: String = cand.delete().into();

        assert_eq!(cand_str, "tst");
    }

    #[test]
    fn test_delete_2() {
        let cand: SyllabifiedCandidate = "test".into();

        let cand_str: String = cand.delete().delete().delete().delete().delete().into();

        assert_eq!(cand_str, "");
    }

    #[test]
    fn test_delete_3() {
        let cand: SyllabifiedCandidate = "owókíowó".into();

        let cand_str: String = cand.delete().delete().delete().delete().into();

        assert_eq!(cand_str, "wówó");
    }

    #[test]
    fn test_delete_4() {
        let cand: SyllabifiedCandidate = "owókíowó".into();

        let cand_str: String = cand
            .delete()
            .delete()
            .delete()
            .delete()
            .delete()
            .delete()
            .delete()
            .delete()
            .into();

        assert_eq!(cand_str, "");
    }

    #[test]
    fn test_ident_1() {
        let cand1: SyllabifiedCandidate = "owókíowó".into();
        let cand2: SyllabifiedCandidate = "ówakíówó".into();

        let ident = Ident(cand1);

        assert_eq!(ident.evaluate(cand2), 3);
    }
}
