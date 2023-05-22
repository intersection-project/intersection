use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Results from [unionize_set].
#[derive(Debug, PartialEq)]
pub struct UnionizeSetResult<'a, Key, Value>
where
    Key: PartialEq + Eq + Hash,
    Value: PartialEq + Eq + Hash,
{
    /// The keys within `preexisting_sets` that were used to create the output set
    pub sets: HashSet<&'a Key>,
    /// Those values not included when you calculate the union of all sets in `sets` versus `target`
    pub outliers: HashSet<&'a Value>,
}

/// Represent a set as the union of many other pre-existing sets
///
/// This function solves the so-called "Intersection Set Reduction Problem" as described by #16.
///
/// Given two inputs, a "target" set and many "pre-existing" sets, return those sets within the
/// "pre-existing" list and one "outliers" set such that when the union of all of the returned pre-existing
/// sets and the outliers are taken, it exactly equals the target set.
///
/// This function takes a HashMap of keys to pre-existing sets and returns keys from that HashMap.
///
/// You may be confused as to how this function is used within Intersection: It's quite simple, actually.
/// Given a list of Discord users to @-mention (the target set) and every role within the server (the
/// pre-existing sets), determine which roles (the output set of pre-existing sets) and members (the outliers)
/// to mention. The goal is to return as few mentions in the message as possible to keep it short, so this
/// function achieves that goal. The "optimal" solution for any given problem is where the total number
/// of returned sets plus the total number of outliers is as small as possible. You can read more about the
/// actual problem in issue #16, which also describes the many methods of implementing it.
///
/// This has not been proven to be the exact most optimal solution, and that's not the primary goal currently.
/// The main goal of this function is to be as fast and performant as possible, while still providing an _almost_
/// complete solution. You can read about all of the possible methods that were considered for this algorithm in
/// issue #16 and PR #18.
///
// TODO: What's the time complexity of this?
// TODO: Is this a _perfect_ solution? Proof would be nice.
///
/// Again, this is a best-effort optimization and some cases might be missed. Please contribute or let us know
/// if you're able to find an edge case we haven't considered.
pub fn unionize_set<'a, Key, Value>(
    target: &'a HashSet<Value>,
    preexisting_sets: &'a HashMap<Key, HashSet<Value>>,
) -> UnionizeSetResult<'a, Key, Value>
where
    Key: PartialEq + Eq + Hash + Copy,
    Value: PartialEq + Eq + Hash + Copy,
{
    // A clone of every one of the preexisting sets, excluding those that aren't subsets of target
    let mut cloned_sets = preexisting_sets
        .iter()
        .filter(|(_, set)| set.is_subset(target))
        .map(|(k, v)| (*k, v.clone()))
        .collect::<HashMap<_, _>>();
    let mut cloned_target = target.clone();
    let mut output_keys: HashSet<Key> = HashSet::new();

    while cloned_sets.values().any(|set| !set.is_empty()) {
        let max_size = cloned_sets
            .values()
            .map(|set| set.len())
            .max()
            .expect("cloned_sets is empty"); // This should never happen, as the .any() call would fail

        let sets_with_max_size = cloned_sets
            .iter()
            .filter(|(_, set)| set.len() == max_size)
            .collect::<Vec<_>>();

        // Depending on how many sets we have...
        // All of the sets in sets_with_max_size have the same length.
        // We must pick ONE set to work with. There is a few possible cases for the interaction between
        // two sets:
        // - EQUALITY: Both sets are exactly equal (a == b)
        // - OVERLAP: Some items are shared between each set (cardinality(a intersection b) != 0)
        // - DISTINCTION: No items are shared between each set (card(a intersection b) == 0)
        // If we can find at least one DISTINCT set from all of the "conflicting" sets (those in
        // sets_with_max_size) then we can use any of those sets. If there is no distinct set, then
        // we will select whichever set has the most unique elements in it (the set with the least
        // overlapping elements). If there is a NON-ZERO tie, any selection works. If all sets unique counts are 0,
        // we can choose any set because they must all be equal.
        let selected_set = match sets_with_max_size.len() {
            // REACHABILITY: This is unreachable as the .max() call would return None
            0 => unreachable!(),
            1 => {
                // There is no conflict, we can use this set!
                // this cannot panic as len>1 implies that there is at least one element
                sets_with_max_size[0]
            }
            _ => {
                // A set is distinct if it is not a subset of the union of all other sets within
                // sets_with_max_size.

                // Get the union of all sets within sets_with_max_size except for a specific index
                let union_of_all_sets_except = |index: usize| {
                    sets_with_max_size
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != index)
                        .map(|(_, (_, set))| set)
                        .copied()
                        .flatten()
                        .collect::<HashSet<_>>()
                };

                let is_distinct = |index: usize| {
                    !sets_with_max_size[index]
                        .1
                        .iter()
                        .collect::<HashSet<_>>()
                        .is_subset(&union_of_all_sets_except(index))
                };

                // Find the first distinct set, if any
                let first_distinct_set = sets_with_max_size
                    .iter()
                    .enumerate()
                    .find(|(i, _)| is_distinct(*i));

                match first_distinct_set {
                    // If there is a distinct set, use it
                    Some((i, _)) => sets_with_max_size[i],
                    // Otherwise, use the set with the most unique elements
                    None => *sets_with_max_size
                        .iter()
                        .max_by_key(|(_, set)| {
                            set.iter()
                                .filter(|&x| !union_of_all_sets_except(0).contains(x))
                                .count()
                        })
                        .expect("sets_with_max_size is empty"), // This should never happen, as the .any() call would fail
                }
            }
        };

        // selected_set is now the set we'd like to use. we can add it to our output:
        output_keys.insert(*selected_set.0);

        let cloned = selected_set.1.clone();

        // Remove all of the values in selected_set.1 from every set in cloned sets
        cloned_sets = cloned_sets
            .into_iter()
            .map(|(k, set)| (k, set.difference(&cloned).copied().collect()))
            .collect();
        cloned_target = cloned_target.difference(&cloned).copied().collect();
    }

    // Outliers = cloned_target
    // Output sets = output_keys
    UnionizeSetResult {
        // Map each key back to its original in preexisting_sets - this is just to convert from
        // Key to &Key without returning data owned by this fn
        sets: output_keys
            .into_iter()
            .map(|key| preexisting_sets.get_key_value(&key).unwrap().0)
            .collect(),
        // Ditto
        outliers: cloned_target
            .into_iter()
            .map(|value| {
                target
                    .iter()
                    .find(|&x| *x == value)
                    .expect("value not in target")
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Target: {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12}
    /// Input set 0: {1, 2, 3}
    /// Input set 1: {4, 5, 6}
    /// Input set 2: {7, 8, 9}
    /// Output sets: [R0, R1, R2]
    /// Output outliers: {10, 11, 12}
    #[test]
    fn unionize_set_works_with_no_overlap() {
        let target = (1..=12).collect::<HashSet<_>>();
        let preexisting_sets = HashMap::from([
            ("1..=3", HashSet::from([1, 2, 3])),
            ("4..=6", HashSet::from([4, 5, 6])),
            ("7..=9", HashSet::from([7, 8, 9])),
        ]);

        assert_eq!(
            unionize_set(&target, &preexisting_sets),
            UnionizeSetResult {
                sets: HashSet::from([&"1..=3", &"4..=6", &"7..=9"]),
                outliers: HashSet::from([&10, &11, &12])
            }
        );
    }

    /// Target: {1, 2, 3}
    /// No input sets
    /// Output sets: []
    /// Output outliers: {1, 2, 3}
    #[test]
    fn unionize_set_works_given_no_input_sets() {
        let target = HashSet::from([1, 2, 3]);
        let preexisting_sets: HashMap<&str, HashSet<i32>> = HashMap::new();

        assert_eq!(
            unionize_set(&target, &preexisting_sets),
            UnionizeSetResult {
                sets: HashSet::new(),
                outliers: HashSet::from([&1, &2, &3])
            }
        );
    }

    /// Target: {1, 2, 3}
    /// Input set 0: {2, 3, 7}
    /// Output sets: []
    /// Output outliers: {1, 2, 3}
    #[test]
    fn unionize_set_ignores_non_subsets_of_target() {
        let target = HashSet::from([1, 2, 3]);
        let preexisting_sets = HashMap::from([("not a subset", HashSet::from([2, 3, 7]))]);

        assert_eq!(
            unionize_set(&target, &preexisting_sets),
            UnionizeSetResult {
                sets: HashSet::new(),
                outliers: HashSet::from([&1, &2, &3])
            }
        );
    }

    /// Target: {1, 2, 3}
    /// Input set 0: {1, 2, 3}
    /// Input set 1: {1, 2, 3}
    /// Output sets: [R0] or [R1] (either solution is correct)
    /// Output outliers: {}
    #[test]
    fn unionize_set_works_with_equal_sets() {
        let target = HashSet::from([1, 2, 3]);
        let preexisting_sets = HashMap::from([
            ("A", HashSet::from([1, 2, 3])),
            ("B", HashSet::from([1, 2, 3])),
        ]);

        let UnionizeSetResult { sets, outliers } = unionize_set(&target, &preexisting_sets);
        assert_eq!(outliers.len(), 0);
        assert_eq!(sets.len(), 1);
        assert!(sets == HashSet::from([&"A"]) || sets == HashSet::from([&"B"]));
    }

    /// Target: {1, 2, 3, 4, 5}
    /// Input set 0: {1, 2, 3, 4}
    /// Input set 1: {2, 3}
    /// Input set 2: {4, 5}
    /// Output sets: [R0, R2] (R1 is redundant)
    /// Output outliers: {}
    #[test]
    fn unionize_set_removes_redundant_sets() {
        let target = HashSet::from([1, 2, 3, 4, 5]);
        let preexisting_sets = HashMap::from([
            ("A", HashSet::from([1, 2, 3, 4])),
            ("B", HashSet::from([2, 3])),
            ("C", HashSet::from([4, 5])),
        ]);

        assert_eq!(
            unionize_set(&target, &preexisting_sets),
            UnionizeSetResult {
                sets: HashSet::from([&"A", &"C"]),
                outliers: HashSet::new()
            }
        );
    }

    /// Target: {1, 2, 3, 4}
    /// Input set 0: {1, 2}
    /// Input set 1: {2, 3}
    /// Input set 2: {3, 4}
    /// Output sets: [R0, R2] (R1 is redundant)
    /// Output outliers: {}
    #[test]
    fn unionize_set_works_with_overlap() {
        let target = HashSet::from([1, 2, 3, 4]);
        let preexisting_sets = HashMap::from([
            ("A", HashSet::from([1, 2])),
            ("B", HashSet::from([2, 3])),
            ("C", HashSet::from([3, 4])),
        ]);

        assert_eq!(
            unionize_set(&target, &preexisting_sets),
            UnionizeSetResult {
                sets: HashSet::from([&"A", &"C"]),
                outliers: HashSet::new()
            }
        );
    }
}
