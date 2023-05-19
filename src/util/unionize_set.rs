use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Results from [unionize_set].
#[derive(Debug, PartialEq)]
pub struct UnionizeSetResult<'a, Key, Value>
where
    Key: PartialEq + Eq + Hash,
    Value: PartialEq + Eq + Hash,
{
    pub sets: HashSet<&'a Key>,
    pub outliers: HashSet<&'a Value>,
}

/// "Optimize" a set, representing it as the union of pre-existing sets and one "outliers" set.
///
/// Given the following:
/// - A target set to represent
/// - A HashMap of pre-existing HashSets
///
/// This function will return two HashSets:
/// - A set of keys within the HashMap
/// - The "outliers" set
///
/// The purpose of this function is to take the input set and calculate which pre-existing sets
/// which when the union of all of them along with the outliers set is calculated, will exactly
/// equal the target set. This is better described using an example from this project: Intersection
/// calculates a set of user IDs that must be mentioned, but the long message that might be a result
/// of a long DRQL query would not be very user-friendly. This function is passed the target users,
/// and a HashMap of every role in the server, and it outputs the roles that can be mentioned to
/// closely represent the target users in as few mentions as possible. In some cases, there will not
/// be a perfect representation, and the outliers set represents those that were not included.
///
/// This function is a best-effort optimization, and speed is a CRITICAL priority, meaning some
/// small details may not be accounted for in the interest of rare cases. The output from this
/// function may change without notice.
///
/// TODO: Example
pub fn unionize_set<'a, Key, Value>(
    target: &'a HashSet<Value>,
    preexisting_sets: &'a HashMap<Key, HashSet<Value>>,
) -> UnionizeSetResult<'a, Key, Value>
where
    Key: PartialEq + Eq + Hash + Copy,
    Value: PartialEq + Eq + Hash + Copy,
{
    // First, we will determine which sets in preexisting_sets even *qualify* to be used in the output.
    // The condition here is simple: A qualifying set must be a subset of the target.
    let qualifying_sets = preexisting_sets
        .iter()
        .filter(|(_, set)| set.is_subset(target))
        .collect::<HashMap<_, _>>();

    // Next, we remove so-called "redundant qualifiers." A qualifier is considered redundant if
    // another qualifying set is a superset of it -- because if we take the union, all of the values
    // in this set will be included in one of the other sets, making this set unnecessary.
    let optimized_qualifiers = qualifying_sets
        .iter()
        .filter(|(key, value)| {
            // Filter out any values where this predicate is true for any value within qualifying_sets...
            !(qualifying_sets.iter().any(|(other_key, other_value)| {
                // ...except for the value itself.
                // FIXME: In the case that two qualifiers have identical member lists,
                //        (e.g.: @{everyone} where @everyone are all online so @everyone==@here),
                //        this will remove *both* sets. this is not catastrophic, as optimization
                //        is sorta "best-effort" and the outliers will be caught, but it's nice
                //        if we can fix this
                *key != other_key && value.is_subset(other_value)
            }))
        })
        .map(|(&a, &b)| (a, b))
        .collect::<HashMap<_, _>>();

    // Next, we calculate the union of all of the sets we qualified! This is used in calculating
    // outliers, later.
    let union_of_qualifiers = optimized_qualifiers
        .values()
        .copied() // TODO: remove copy
        .flatten()
        .collect::<HashSet<_>>();

    let outliers = target
        .iter()
        .collect::<HashSet<_>>()
        .difference(&union_of_qualifiers)
        .copied() // TODO: remove copy
        .collect::<HashSet<_>>();

    UnionizeSetResult {
        sets: optimized_qualifiers.into_keys().collect::<HashSet<_>>(),
        outliers,
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
