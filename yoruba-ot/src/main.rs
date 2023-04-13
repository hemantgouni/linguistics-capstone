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
    syllable_index: SyllableIndex,
    seg_type: SegmentType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum SyllableIndex {
    Onset,
    Nucleus,
    Coda,
    None,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum SegmentType {
    Vowel,
    Consonant,
}

impl SyllabifiedCandidate {
    fn delete(mut self) -> Self {
        if !self.form.is_empty() {
            self.form.remove(self.rng.gen_range(0..self.form.len()));
        }
        self
    }
}

fn get_seg_type(grapheme: &str) -> SegmentType {
    if VOWELS.contains(&grapheme) {
        SegmentType::Vowel
    } else {
        SegmentType::Consonant
    }
}

impl From<&str> for SyllabifiedCandidate {
    fn from(str: &str) -> SyllabifiedCandidate {
        let mut graphemes: Vec<Segment> = str
            .graphemes(true)
            .map(|grapheme| Segment {
                char: grapheme.to_owned(),
                syllable_index: SyllableIndex::None,
                seg_type: get_seg_type(grapheme),
            })
            .collect();

        // grouping affricates together as non-separate segments
        for idx in 0..(graphemes.len() - 1) {
            if graphemes[idx].char == "d\u{361}" && graphemes[idx + 1].char == "ʒ" {
                let char_to_concat = graphemes[idx + 1].char.clone();
                graphemes[idx].char.push_str(&char_to_concat);
                graphemes.remove(idx + 1);
            }
        }

        SyllabifiedCandidate {
            form: syllabify(graphemes),
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
    fn morph_type(&self, seg_type: SyllableIndex) -> Segment {
        Segment {
            char: self.char.clone(),
            syllable_index: seg_type,
            seg_type: self.seg_type.clone(),
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
                segment.morph_type(SyllableIndex::Nucleus)
            } else {
                segment.clone()
            }
        })
        .collect()
}

fn mark_onsets(candidate: Vec<Segment>) -> Vec<Segment> {
    let mut annotated: (SyllableIndex, Vec<Segment>) = candidate.iter().rev().fold(
        (SyllableIndex::None, Vec::new()),
        |(prev_seg_type, mut segs), seg| match (prev_seg_type, seg.to_owned().syllable_index) {
            (SyllableIndex::Nucleus, SyllableIndex::None) => (
                SyllableIndex::Onset,
                segs.push_ret(seg.morph_type(SyllableIndex::Onset))
                    .to_owned(),
            ),
            _ => (
                seg.syllable_index.to_owned(),
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
        .map(|seg| match seg.syllable_index {
            SyllableIndex::None => seg.morph_type(SyllableIndex::Coda),
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
            .filter(|seg| seg.syllable_index == SyllableIndex::Nucleus)
            .count();

        let onsets = surface
            .form
            .iter()
            .filter(|seg| seg.syllable_index == SyllableIndex::Onset)
            .count();

        onsets - syllabi
    }
}

struct SonSeqPr;

impl Constraint for SonSeqPr {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize {
        surface
            .form
            .iter()
            // a hack to ignore accent marks
            //
            // .next().unwrap() should never panic here bc that's only possible if the initial
            // candidate input string is empty, and if that's true, then the iterator will be empty
            .map(|seg| match dbg!(seg.char.chars().next().unwrap()) {
                'e' | 'ɛ' | 'o' | 'ɔ' => 1,
                'u' => 2,
                'i' => 3,
                _ => 0,
            })
            .sum()
    }
}

fn main() {
    let syllabified_candidate: SyllabifiedCandidate = dbg!("d͡ʒuigi".into());
    dbg!(SonSeqPr.evaluate(syllabified_candidate));
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

    #[test]
    fn test_ssp_1() {
        let syllabified_candidate: SyllabifiedCandidate = "owókíowó".into();
        assert_eq!(SonSeqPr.evaluate(syllabified_candidate), 7);
    }

    #[test]
    fn test_ssp_2() {
        let syllabified_candidate: SyllabifiedCandidate = "".into();
        assert_eq!(SonSeqPr.evaluate(syllabified_candidate), 0);
    }
}
