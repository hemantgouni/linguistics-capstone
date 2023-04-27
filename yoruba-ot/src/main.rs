mod constraint;
mod utils;

use crate::constraint::{
    Constraint, Dep, Ident, Max, MaxFinalV, MaxInitialV, Onset, RankedConstraint, SonSeqPr,
    Syllabify,
};
use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

use utils::{permute_delete, VecRet};

// string -> syllabified candidate -> random deletions (all winners generated via deletions) ->
// eval against constraints

// remove accent marks before processing bc we don't have the accented variants of the vowels here?
// are there other limitations that prevent us from keeping accents?
//
// also, i think we look at the first element of the grapheme, right? which should be the bare
// vowel

const VOWELS: [&str; 7] = ["o", "ɛ", "ɔ", "i", "u", "a", "e"];

#[derive(Debug, Clone, PartialEq)]
pub struct SyllabifiedCandidate {
    form: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq)]
struct Segment {
    char: String,
    syllable_index: SyllableIndex,
    seg_type: SegmentType,
    morpheme_index: UnderlyingIndex,
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

#[derive(Debug, Clone, Eq, PartialEq)]
enum UnderlyingIndex {
    Initial,
    Middle,
    Final,
}

impl SyllabifiedCandidate {
    fn permute(&self) -> Vec<Self> {
        permute_delete(&self.form)
            .iter()
            .map(|form| SyllabifiedCandidate {
                form: syllabify(form.to_owned()),
            })
            .collect()
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
        let mut underlying_final: Vec<usize> = str
            .grapheme_indices(true)
            .filter_map(|(index, grapheme)| {
                if grapheme == "-" {
                    Some(index - 1)
                } else {
                    None
                }
            })
            .collect();

        if str.graphemes(true).count() != 0 {
            underlying_final.push(str.graphemes(true).count() - 1);
        }

        let mut underlying_initial: Vec<usize> = str
            .grapheme_indices(true)
            .filter_map(|(index, grapheme)| {
                if grapheme == "-" {
                    Some(index + 1)
                } else {
                    None
                }
            })
            .collect();

        underlying_initial.push(0);

        let mut graphemes: Vec<Segment> = str
            .grapheme_indices(true)
            .filter_map(|(index, grapheme)| match grapheme {
                "-" => None,
                _ => Some(Segment {
                    char: grapheme.to_owned(),
                    syllable_index: SyllableIndex::None,
                    seg_type: get_seg_type(grapheme),
                    morpheme_index: if underlying_final.contains(&index) {
                        UnderlyingIndex::Final
                    } else if underlying_initial.contains(&index) {
                        UnderlyingIndex::Initial
                    } else {
                        UnderlyingIndex::Middle
                    },
                }),
            })
            .collect();

        // grouping affricates together as non-separate segments
        // can we do this more cleanly?
        if !graphemes.is_empty() {
            for idx in 0..graphemes.len() - 1 {
                if (graphemes[idx].char == "d" && graphemes[idx + 1].char == "ʒ")
                    || (graphemes[idx].char == "g" && graphemes[idx + 1].char == "b")
                {
                    let char_to_concat = graphemes[idx + 1].char.clone();
                    graphemes[idx].char.push_str(&char_to_concat);
                    graphemes.remove(idx + 1);
                }
            }
        }

        SyllabifiedCandidate {
            form: syllabify(graphemes),
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
            morpheme_index: self.morpheme_index.clone(),
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
        .fold(
            (SyllableIndex::None, Vec::new()),
            |(prev_seg_type, mut segs), seg| match (prev_seg_type, seg.syllable_index.to_owned()) {
                (SyllableIndex::Nucleus, SyllableIndex::None) => (
                    SyllableIndex::Coda,
                    segs.push_ret(seg.morph_type(SyllableIndex::Coda))
                        .to_owned(),
                ),
                _ => (
                    seg.syllable_index.to_owned(),
                    segs.push_ret(seg.to_owned()).to_owned(),
                ),
            },
        )
        .1
}

fn syllabify(candidate: Vec<Segment>) -> Vec<Segment> {
    // TODO: clean this up and move it into the impl for SyllabifiedCandidate
    mark_codas(mark_onsets(mark_vowels(
        candidate
            .iter()
            .map(|seg| Segment {
                char: seg.char.clone(),
                syllable_index: SyllableIndex::None,
                seg_type: seg.seg_type.clone(),
                morpheme_index: seg.morpheme_index.clone(),
            })
            .collect(),
    )))
}

fn evaluate(
    underlying_candidate: SyllabifiedCandidate,
    mut constraints: Vec<RankedConstraint>,
) -> Vec<SyllabifiedCandidate> {
    let surface_forms: Vec<SyllabifiedCandidate> = underlying_candidate.permute();

    constraints.sort_by(|const1, const2| const1.rank.cmp(&const2.rank));

    let grouped_constraints: Vec<Vec<&RankedConstraint>> = constraints
        .iter()
        .group_by(|constraint| constraint.rank)
        .into_iter()
        .map(|(_, group)| group.collect())
        .collect::<Vec<Vec<&RankedConstraint>>>();

    grouped_constraints
        .iter()
        .fold(surface_forms, |forms, constraint| match forms.len() {
            0 => panic!("No forms to evaluate!"),
            1 => forms,
            _ => {
                let rankings: Vec<(SyllabifiedCandidate, usize)> = forms
                    .iter()
                    .map(|form| (form.to_owned(), constraint.evaluate(form.to_owned())))
                    .collect();

                #[cfg(debug_assertions)]
                rankings
                    .iter()
                    .map(|(cand, vios)| {
                        println!(
                            "{:?} has {:?} violations of {:#?}",
                            String::from(cand.to_owned()),
                            vios,
                            constraint
                        )
                    })
                    // dropping a bunch of references; we just need this to consume the iterator,
                    // since they're lazy
                    .for_each(drop);

                let min: usize = rankings
                    .iter()
                    .min_by(|(_, vio1), (_, vio2)| vio1.cmp(vio2))
                    .expect("Iterator was empty!")
                    .1;

                let next: Vec<SyllabifiedCandidate> = rankings
                    .iter()
                    .filter(|(_, vio_count)| vio_count == &min)
                    .map(|(cand, _)| cand.to_owned())
                    .collect();

                if next.is_empty() {
                    forms
                } else {
                    next
                }
            }
        })
}

fn main() {
    use std::io::{stdin, stdout, Write};

    let mut buffer = String::new();

    loop {
        print!("> ");

        stdout().flush().unwrap();

        stdin().read_line(&mut buffer).unwrap();

        let cand: SyllabifiedCandidate = buffer.trim_end().into();

        println!(
            "{:?}",
            evaluate(
                cand.clone(),
                vec![
                    RankedConstraint {
                        rank: 0,
                        constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 1,
                        constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 1,
                        constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 2,
                        constraint: Box::new(Onset) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 2,
                        constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 2,
                        constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 3,
                        constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                    },
                    RankedConstraint {
                        rank: 4,
                        constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                    },
                ],
            )
            .iter()
            .map(|cand| String::from(cand.to_owned()))
            .collect::<Vec<String>>()
        );

        buffer.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_ident_1() {
        let cand1: SyllabifiedCandidate = "owókíowó".into();
        let cand2: SyllabifiedCandidate = "ówakíówó".into();

        let ident = Ident(cand1);

        assert_eq!(ident.evaluate(cand2), 3);
    }

    #[test]
    fn test_dep_1() {
        let syllabified_candidate: SyllabifiedCandidate = dbg!("owoktwiowo".into());
        assert_eq!(Dep(syllabified_candidate.clone()).evaluate("".into()), 0)
    }

    #[test]
    fn test_ssp_1() {
        let syllabified_candidate: SyllabifiedCandidate = "owókíowó".into();
        assert_eq!(SonSeqPr.evaluate(syllabified_candidate), 8);
    }

    #[test]
    fn test_ssp_2() {
        let syllabified_candidate: SyllabifiedCandidate = "".into();
        assert_eq!(SonSeqPr.evaluate(syllabified_candidate), 0);
    }

    #[test]
    fn test_onset_1() {
        let syllabified_candidate: SyllabifiedCandidate = dbg!("owoktwiowo".into());
        assert_eq!(Onset.evaluate(syllabified_candidate), 6);
    }

    #[test]
    fn test_syllabify_constraint() {
        let syllabified_candidate: SyllabifiedCandidate = dbg!("owoktwiowo".into());
        assert_eq!(Syllabify.evaluate(syllabified_candidate), 1);
    }

    #[test]
    fn test_evaluate_1() {
        let cand: SyllabifiedCandidate = "owo-ki-owo".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["owokowo"])
    }

    #[test]
    fn test_evaluate_2() {
        let cand: SyllabifiedCandidate = "ɔmɔ-ki-ɔmɔ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["ɔmɔkɔmɔ"])
    }

    #[test]
    fn test_evaluate_3() {
        let cand: SyllabifiedCandidate = "se-olu".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["solu"])
    }

    #[test]
    fn test_evaluate_4() {
        let cand: SyllabifiedCandidate = "dʒɛ-ede".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["dʒede"])
    }

    #[test]
    fn test_evaluate_5() {
        let cand: SyllabifiedCandidate = "dʒo-ɛwu".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["dʒɛwu"])
    }

    #[test]
    fn test_evaluate_6() {
        let cand: SyllabifiedCandidate = "ra-ɔgɛdɛ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["ragɛdɛ"])
    }

    #[test]
    fn test_evaluate_7() {
        let cand: SyllabifiedCandidate = "ni-oko".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["noko"])
    }

    #[test]
    fn test_evaluate_8() {
        let cand: SyllabifiedCandidate = "si-ɔd͡ʒa".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["sɔd͡ʒa"])
    }

    #[test]
    fn test_evaluate_9() {
        let cand: SyllabifiedCandidate = "gbe-inɔ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["gbenɔ"])
    }

    #[test]
    fn test_evaluate_10() {
        let cand: SyllabifiedCandidate = "gba-iʃɛ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["gbaʃɛ"])
    }

    #[test]
    fn test_evaluate_11() {
        let cand: SyllabifiedCandidate = "wo-ilɛ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["wolɛ"])
    }

    #[test]
    fn test_evaluate_12() {
        let cand: SyllabifiedCandidate = "dʒi-aʃɔ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["dʒaʃɔ"])
    }

    #[test]
    fn test_evaluate_13() {
        let cand: SyllabifiedCandidate = "dʒu-igi".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["dʒugi"])
    }

    #[test]
    fn test_evaluate_14() {
        let cand: SyllabifiedCandidate = "lu-ilɛ".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["lulɛ"])
    }

    #[test]
    fn test_evaluate_15() {
        let cand: SyllabifiedCandidate = "bu-ɔba".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["bɔba"])
    }

    #[test]
    fn test_evaluate_16() {
        let cand: SyllabifiedCandidate = "ru-epo".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["repo"])
    }

    #[test]
    fn test_evaluate_17() {
        let cand: SyllabifiedCandidate = "bu-omi".into();
        let surface_forms = evaluate(
            cand.clone(),
            vec![
                RankedConstraint {
                    rank: 0,
                    constraint: Box::new(Syllabify) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Ident(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 1,
                    constraint: Box::new(Dep(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Onset) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(SonSeqPr) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 2,
                    constraint: Box::new(Max(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 3,
                    constraint: Box::new(MaxInitialV(cand.clone())) as Box<dyn Constraint>,
                },
                RankedConstraint {
                    rank: 4,
                    constraint: Box::new(MaxFinalV(cand)) as Box<dyn Constraint>,
                },
            ],
        )
        .iter()
        .map(|cand| String::from(cand.to_owned()))
        .collect::<Vec<String>>();

        assert_eq!(surface_forms, vec!["bomi"])
    }
}
