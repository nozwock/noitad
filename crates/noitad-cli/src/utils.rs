use std::collections::HashMap;

pub fn group_equal_by_key<T, U>(arr: &Vec<T>, key: impl Fn(&T) -> U) -> HashMap<U, Vec<&T>>
where
    U: std::hash::Hash + std::cmp::Eq,
{
    let mut groups: HashMap<U, Vec<&T>> = HashMap::new();

    for item in arr.iter() {
        groups.entry(key(item)).or_insert_with(Vec::new).push(item);
    }

    groups
}
