use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Results from [unionize_set].
#[derive(Debug)]
pub struct UnionizeSetResult<'a, Key, Value> {
    pub sets: HashMap<&'a Key, &'a HashSet<Value>>,
    pub outliers: HashSet<Value>,
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
        .copied() // TODO: remove copy
        .collect::<HashSet<_>>();

    let outliers = target
        .difference(&union_of_qualifiers)
        .copied() // TODO: remove copy
        .collect::<HashSet<_>>();

    UnionizeSetResult {
        sets: optimized_qualifiers,
        outliers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unionize_set_works_with_superset_of_all() {
        let target = HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let preexisting_sets = HashMap::from([
            ("empty set", HashSet::from([])),
            ("not a subset", HashSet::from([1, 32, 5, 2, 6])),
            (
                "exact match",
                HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            ),
        ]);

        let UnionizeSetResult { sets, outliers } = unionize_set(&target, &preexisting_sets);
        assert_eq!(outliers.len(), 0);
        assert_eq!(
            sets,
            HashMap::from([(
                &"exact match",
                &HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
            )])
        );
    }

    #[test]
    fn unionize_set_works() {
        let target = HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let preexisting_sets = HashMap::from([
            ("empty set", HashSet::from([])), // gets optimized away
            ("not a subset", HashSet::from([1, 32, 5, 2, 6])), // gets disqualified
            ("subset of A", HashSet::from([1, 2, 3, 4])), // gets optimized away
            ("A", HashSet::from([1, 2, 3, 4, 5, 8])), // kept
            ("other subset of A", HashSet::from([4, 1, 8])), // gets optimized away
            ("B", HashSet::from([5, 9])),     // kept
        ]);

        let UnionizeSetResult { sets, outliers } = unionize_set(&target, &preexisting_sets);
        assert_eq!(outliers, HashSet::from([6, 7, 10]));
        assert_eq!(
            sets,
            HashMap::from([
                (&"A", &HashSet::from([1, 2, 3, 4, 5, 8])),
                (&"B", &HashSet::from([5, 9]))
            ])
        );
    }
}
