pub trait VecRet<T> {
    fn push_ret(&mut self, t: T) -> &mut Vec<T>;
    fn append_ret(&mut self, t: &mut Vec<T>) -> &mut Vec<T>;
}

impl<T> VecRet<T> for Vec<T> {
    fn push_ret(&mut self, t: T) -> &mut Vec<T> {
        self.push(t);
        self
    }

    fn append_ret(&mut self, t: &mut Vec<T>) -> &mut Vec<T> {
        self.append(t);
        self
    }
}

fn append_to_all<A: Clone>(elem: A, vecs: Vec<Vec<A>>) -> Vec<Vec<A>> {
    vecs.iter()
        .map(|elems| {
            vec![elem.to_owned()]
                .append_ret(&mut elems.to_vec())
                .to_owned()
        })
        .collect()
}

pub fn permute_delete<A: Clone>(list: &Vec<A>) -> Vec<Vec<A>> {
    match list.as_slice() {
        [] => panic!(),
        [elem] => vec![vec![elem.to_owned()], vec![]],
        [elem, elems @ ..] => append_to_all(elem.to_owned(), permute_delete(&elems.to_vec()))
            .append_ret(&mut permute_delete(&elems.to_vec()))
            .to_vec(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_permute_1() {
        let vec = vec!['a', 'b', 'c', 'd'];
        let ret_value = permute_delete(&vec);
        let expected = vec![
            vec!['a', 'b', 'c', 'd'],
            vec!['a', 'b', 'c'],
            vec!['a', 'b', 'd'],
            vec!['a', 'b'],
            vec!['a', 'c', 'd'],
            vec!['a', 'c'],
            vec!['a', 'd'],
            vec!['a'],
            vec!['b', 'c', 'd'],
            vec!['b', 'c'],
            vec!['b', 'd'],
            vec!['b'],
            vec!['c', 'd'],
            vec!['c'],
            vec!['d'],
            vec![],
        ];
        assert_eq!(expected, ret_value)
    }
}
