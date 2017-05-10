// Copyright 2017 Thorben Kroeger.
// Dual-licensed MIT and Apache 2.0 (see LICENSE files for details).

pub struct GroupByFn<'a, T, F>
where
    F: Fn(&T, &T) -> bool,
    T: 'a,
{
    data: &'a [T],
    idx_first: usize,
    compare: F,
}

impl<'a, T, F> GroupByFn<'a, T, F>
where
    F: Fn(&T, &T) -> bool,
{
    fn new(data: &'a [T], compare: F) -> GroupByFn<'a, T, F> {
        GroupByFn {
            data: data,
            idx_first: 0,
            compare: compare,
        }
    }
}

pub fn group_by_fn<'a, T, F>(data: &'a [T], compare: F) -> GroupByFn<'a, T, F>
where
    F: Fn(&T, &T) -> bool,
{
    GroupByFn::new(data, compare)
}

impl<'a, T, F> Iterator for GroupByFn<'a, T, F>
where
    F: Fn(&T, &T) -> bool,
    T: 'a,
{
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() == 0 {
            return None;
        }
        if self.idx_first >= self.data.len() {
            return None;
        }

        // reference to first element in current group
        let first = &self.data[self.idx_first];

        // go over all elements coming after that first element
        for i in self.idx_first + 1..self.data.len() {
            let current = &self.data[i];
            if !(self.compare)(&*current, &*first) {
                // new group
                let group_start = self.idx_first;
                self.idx_first = i;
                return Some(&self.data[group_start..i]);
            }
        }
        let idx_first = self.idx_first;
        self.idx_first = self.data.len();
        return Some(&self.data[idx_first..self.data.len()]);
    }
}

#[test]
fn test_grouping_empty() {
    let v: Vec<u32> = vec![];
    let groups = GroupByFn::new(&v, |x, y| x == y);

    assert_eq!(groups.count(), 0);
}

#[test]
fn test_grouping_single_int() {
    let v = vec![1];
    let groups = GroupByFn::new(&v, |x, y| x == y);

    let a: Vec<Vec<_>> = groups.map(|g| Vec::from(g)).collect();
    let e = vec![vec![1]];
    assert_eq!(a, e);
}

#[test]
fn test_grouping_ints() {
    let v = vec![1, 1, 1, 2, 3, 3, 3];
    let groups = GroupByFn::new(&v, |x, y| x == y);

    let a: Vec<Vec<_>> = groups.map(|g| Vec::from(g)).collect();
    let e = vec![vec![1, 1, 1], vec![2], vec![3, 3, 3]];
    assert_eq!(a, e);
}

#[test]
fn test_grouping_strings() {
    let v = vec!["aa", "aa", "bbb", "bbb", "c", "c", "c"];
    let groups = GroupByFn::new(&v, |x, y| x == y);
    let a: Vec<Vec<_>> = groups.map(|g| Vec::from(g)).collect();
    let e = vec![vec!["aa", "aa"], vec!["bbb", "bbb"], vec!["c", "c", "c"]];
    assert_eq!(a, e);
}
