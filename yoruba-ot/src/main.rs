use rand::{rngs::StdRng, Rng, SeedableRng};
use unicode_segmentation::UnicodeSegmentation;

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
    fn evaluate(candidate: Candidate) -> usize;
}

// impl Constraint {

// }

fn main() {
    let mut cand = Candidate {
        form: "owókíowó".to_string(),
        rng: StdRng::seed_from_u64(7777777),
    };
    println!("{:#?}", cand.delete().delete().delete().delete());
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
}
