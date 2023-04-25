use core::cmp::Ordering;

use crate::{SyllabifiedCandidate, SyllableIndex};
use similar::{DiffOp, TextDiff};

pub trait Constraint {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize;
}

pub trait ConstraintDebug: Constraint + std::fmt::Debug {}

pub struct RankedConstraint {
    pub rank: usize,
    pub constraint: Box<dyn ConstraintDebug>,
}

impl std::fmt::Debug for RankedConstraint {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "RankedConstraint {{ rank: {:?}, constraint: {:?} }}\n",
            self.rank, self.constraint
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident(pub SyllabifiedCandidate);

impl Constraint for Ident {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
        let self_str: String = self.clone().0.into();
        let surface_str: String = surface.into();

        let diff = TextDiff::from_graphemes::<String>(&self_str, &surface_str);

        diff.ops()
            .iter()
            .filter(|op| matches!(op, DiffOp::Replace { .. }))
            .count()
    }
}

impl ConstraintDebug for Ident {}

#[derive(Debug, Clone, PartialEq)]
pub struct Dep(pub SyllabifiedCandidate);

impl Constraint for Dep {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
        let self_str: String = self.clone().0.into();
        let surface_str: String = surface.into();

        let diff = TextDiff::from_graphemes::<String>(&self_str, &surface_str);

        diff.ops()
            .iter()
            .filter(|op| matches!(op, DiffOp::Insert { .. }))
            .count()
    }
}

impl ConstraintDebug for Dep {}

#[derive(Debug, Clone, PartialEq)]
pub struct Onset;

impl Constraint for Onset {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
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

impl ConstraintDebug for Onset {}

#[derive(Debug, Clone, PartialEq)]
pub struct SonSeqPr;

impl Constraint for SonSeqPr {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
        surface
            .form
            .iter()
            // a hack to ignore accent marks
            //
            // .next().unwrap() should never panic here bc that's only possible if the initial
            // candidate input string is empty, and if that's true, then the iterator will be empty
            .map(|seg| match seg.char.chars().next().unwrap() {
                'e' | 'ɛ' | 'o' | 'ɔ' => 1,
                'u' => 2,
                'i' => 3,
                _ => 0,
            })
            .sum()
    }
}

impl ConstraintDebug for SonSeqPr {}

#[derive(Debug, Clone, PartialEq)]
pub struct Syllabify;

impl Constraint for Syllabify {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
        surface
            .form
            .iter()
            .filter(|seg| seg.syllable_index == SyllableIndex::None)
            .count()
    }
}

impl ConstraintDebug for Syllabify {}

#[derive(Debug)]
pub struct Max(pub SyllabifiedCandidate);

impl Constraint for Max {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
        (self.0.form.len() - surface.form.len()) * 3
    }
}

impl ConstraintDebug for Max {}

#[derive(Debug)]
pub struct MaxOnsetSonSeqPr(pub SyllabifiedCandidate);

impl Constraint for MaxOnsetSonSeqPr {
    fn evaluate(&self, surface: SyllabifiedCandidate) -> usize {
        let max = Max(self.0.clone());
        let onset = Onset;
        let son_seq_pr = SonSeqPr;

        max.evaluate(surface.clone())
            + onset.evaluate(surface.clone())
            + son_seq_pr.evaluate(surface.clone())
    }
}

impl ConstraintDebug for MaxOnsetSonSeqPr {}
