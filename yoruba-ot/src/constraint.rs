use crate::{SyllabifiedCandidate, SyllableIndex};
use similar::{DiffOp, TextDiff};

pub trait Constraint {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize;
}

pub struct Ident(pub SyllabifiedCandidate);

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

pub struct Dep(pub SyllabifiedCandidate);

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

pub struct Onset;

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

        syllabi - onsets
    }
}

pub struct SonSeqPr;

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

pub struct Syllabify;

impl Constraint for Syllabify {
    fn evaluate(self, surface: SyllabifiedCandidate) -> usize {
        surface
            .form
            .iter()
            .filter(|seg| seg.syllable_index == SyllableIndex::None)
            .count()
    }
}
