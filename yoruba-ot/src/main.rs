use rand::{rngs::StdRng, Rng, SeedableRng};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
struct Candidate {
    form: String,
    rng: StdRng,
}

impl Candidate {
    fn len(&self) -> usize {
        self.form.graphemes(true).collect::<Vec<&str>>().len()
    }

    fn delete(&self) -> Self {
        let mut self_copy = self.clone();

        let index = if self_copy.len() > 0 {
            self_copy.rng.gen_range(0..self_copy.len())
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

    fn epenthesize(&self) -> Self {
        let mut self_copy = self.clone();
        let index = self_copy.rng.gen_range(0..self_copy.form.len());
        self_copy.form.remove(index);
        self_copy
    }
}

fn main() {
    let rng = StdRng::seed_from_u64(7777777);
    let cand = Candidate {
        form: "test".to_string(),
        rng,
    };
    println!("{:?}", cand.delete().delete().delete());
}

#[cfg(test)]
mod test {
    use super::*;
    use once_cell::sync::Lazy;

    static RNG: Lazy<StdRng> = Lazy::new(|| StdRng::seed_from_u64(7777777));

    #[test]
    fn test_delete_1() {
        let cand = Candidate {
            form: "test".to_string(),
            rng: RNG.clone(),
        }
        .delete();

        assert_eq!(cand.form, "tst");
    }

    #[test]
    fn test_delete_2() {
        let cand = Candidate {
            form: "test".to_string(),
            rng: RNG.clone(),
        }
        .delete()
        .delete()
        .delete()
        .delete()
        .delete();

        assert_eq!(cand.form, "");
    }
}
