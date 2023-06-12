use bitvec::prelude::*;
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
///
/// ## Panics
///
/// Panics if the total number of unique Values in preexisting_sets and target is greater than usize::MAX.
pub fn unionize_set<'a, Key, Value>(
    target: &'a HashSet<Value>,
    preexisting_sets: &'a HashMap<Key, HashSet<Value>>,
) -> UnionizeSetResult<'a, Key, Value>
where
    Key: PartialEq + Eq + Hash + Copy,
    Value: PartialEq + Eq + Hash + Copy,
{
    // There's a fuzz test below that you can run (#[ignore]d by default) to try HUGE data sizes,
    // but we don't run by default as it takes 45+ seconds to run

    // Filter out those preexisting_sets that aren't subsets of target
    // FIXME: This step takes around 8 seconds with the large fuzz test that's found below.
    //        Probably the is_subset?
    let filtered_preexisting_sets = preexisting_sets
        .iter()
        .filter(|(_, set)| set.is_subset(target))
        .collect::<HashMap<_, _>>();

    // This function takes the un-named and unknown time complexity approach that we believe (not
    // yet proven) is optimal from issue #16. This is a best-effort optimization and some cases
    // may miss some edge cases.

    // As a quick summary, this implementation works by picking the largest set from 'preexisting_sets' (filtered
    // so that only those sets that are a subset of our target exist)
    // (we will talk about what to do when there is multiple sets with the same length later) and adding
    // it to our output. Then, we remove those values in that set from every other set in our clone of 'preexisting_sets'
    // and from our clone of 'target'. We repeat this process until all sets are empty. The remaining values in
    // 'target' are our outliers.
    //
    // If we encounter two or more sets with the same size, we choose whichever one has the most elements
    // that are unique to ONLY that set relative to all of the other sets with the same length. If there
    // is a tie here, either element may be chosen. Here's some examples:
    //
    // Target  * * * *
    // ---------------
    //  Set 0  * *
    //  Set 1    * *
    //  Set 2      * *
    //
    // The optimal choice here is sets 0 and 2. Choosing set 1 will not work as it will result in the
    // non-optimal solution of [0, 1, 2] as the output sets...
    //
    // First iteration, select set 1 as spoken above
    // New state: target=[A, D], set 0=[A], set 1=[], set 2=[D]
    // Second iteration, select set 0 or 1, ... the ending result will be all 3 sets. The optimal
    // solution is only set 0 and 2.
    //
    // In our case, we simply select whichever set has the most UNIQUE elements ('x': not unique)
    //
    // Target  * * * *
    // ---------------
    //  Set 0  * x
    //  Set 1    x x
    //  Set 2      x *
    //
    // Then we can see that set 0 or set 2 have the most unique elements and select either one, leading
    // to the correct solution.
    //
    // The other issue with the old approach existed when two sets are equal. In this case, there is
    // still a tie with the "unique" counts and we can select either one (even though the values are
    // both 0). This is fine, as the sets are equal and we can select either one.
    // That's the gist of it. Again, read issue #16 for more information.

    // Before trying to work on this code, examine the original solution from PR #18.
    // This function is very optimization heavy. One of the main optimizations we will be making here
    // is using a bitfield to represent the pre-existing sets to make the "difference" operation
    // that is performed later much more efficient:
    //
    // target: 0, 1, 2, 3
    // set 0:     1, 2
    // set 1:     1, 2, 3
    //
    // real representations:
    // target: 1111
    // set 0:  0110
    // set 1:  0111
    //
    // then, once we choose set 1 we can simply just use filter(set) = set & ~set 1 to remove
    // all of those elements (0b1111 & ~0b0100 = 0b1011, it's like a not operation)
    // because the total number is unknown at compile time, we use a "BitVec" which is like a dynamic
    // sized bitfield. This is implemented in the bitvec crate.
    //
    // In our case, we're not using numbers, they're IDs. We must first create a mapping between
    // Key and usize. This creates an issue where if the total number of unique Value-s in all of the
    // sets and target is greater than usize::MAX, we'll need to panic.

    // First, build the mappings between a Value and some i32.
    // FIXME: This step takes 10 seconds with the large fuzz test which is #[ignore]d below
    let mut next_id: usize = 0;
    let (value_to_index, index_to_value) = filtered_preexisting_sets
        .values()
        .copied()
        .flatten()
        .chain(target)
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|value| {
            let id = next_id;
            next_id += 1;
            ((value, id), (id, value))
        })
        .unzip::<_, _, HashMap<_, _>, HashMap<_, _>>();

    // We can now construct the bitfield for each set and target. Let's start by building target.
    // First, we can create an all-zeroed bitfield of some size:
    let mut target_bitfield = {
        let mut bitfield = bitvec![0; next_id];

        // Now, for every value in target, we set it to 1 in our bitfield
        for value in target {
            let index = value_to_index
                .get(value)
                .expect("value not in value_to_index");
            bitfield.set(*index, true);
        }

        bitfield
    };

    // And now, we can map every preexisting_set to a bitfield...
    // FIXME: This step takes 10 seconds with the large fuzz test which is #[ignore]d below
    let mut preexisting_set_bitfields = filtered_preexisting_sets
        .iter()
        .map(|(key, set)| {
            (key, {
                let mut bitfield = bitvec![0; next_id];

                for value in set.iter() {
                    let index = value_to_index
                        .get(value)
                        .expect("value not in value_to_index");
                    bitfield.set(*index, true);
                }

                bitfield
            })
        })
        .collect::<HashMap<_, _>>();

    // The keys that will be returned in the end
    let mut output_keys: HashSet<Key> = HashSet::new();

    while preexisting_set_bitfields
        .values()
        .any(|bitfield| bitfield.any())
    {
        // First, we find whatever the highest number of 1s in any bitfield is
        let max_size = preexisting_set_bitfields
            .values()
            .map(|bitfield| bitfield.count_ones())
            .max()
            .expect("preexisting_set_bitfields is empty"); // This should never happen, as the .any() call would return false

        // Then, we find all of the bitfields that have that many 1s
        let bitfields_with_max_size = preexisting_set_bitfields
            .iter()
            .filter(|(_, bitfield)| bitfield.count_ones() == max_size)
            .collect::<Vec<_>>();

        // Depending on how many bitfields we have with the maximum size, there's a few different
        // ways we could handle this.
        //
        // We must choose only ONE set to work with. There's 3 possibilities for the interaction
        // between two sets:
        // - EQUALITY (A == B): Both sets are exactly equal
        // - OVERLAP (cardinality(a intersection b) != 0): There is some overlap between two sets
        //   but they are not exactly equal or different.
        // - DISTINCTION (cardinality(a intersection b) == 0): There is no overlap between two sets
        //   at all.
        //
        // First of all, let's handle the most common case of there being no conflict:
        let selected_set = match bitfields_with_max_size.len() {
            // This is unreachable as the .max() call must have had to fail (there's no way that suddenly
            // no set has that length)
            0 => unreachable!(),
            // The most common case: There is no conflict, we can use this set!
            1 => bitfields_with_max_size[0],
            // Anything else...
            _ => {
                // First of all, if there is any distinct set within our conflicting sets, we should
                // choose any one of those sets.
                // A set is "distinct" if the intersection of that set and the union of all conflicting
                // sets has a cardinality (length) of 0.
                // First, we can set something up to find the union of all bitfields in conflicting_sets
                // except for a specific index:
                let union_of_all_bitfields_except = |index: usize| {
                    bitfields_with_max_size
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != index)
                        .map(|(_, (_, v))| v)
                        .fold(bitvec![0; next_id], |acc, bitfield| acc | *bitfield)
                };

                // We can then determine if a set is distinct...
                let is_distinct = |index: usize| {
                    // The order here appears flipped because BitVec has an impl for BitVec & &BitVec
                    // but not &BitVec & BitVec
                    (union_of_all_bitfields_except(index) & bitfields_with_max_size[index].1)
                        .count_ones()
                        == 0
                };

                // Find the first distinct set, if any
                let first_distinct_set = bitfields_with_max_size
                    .iter()
                    .enumerate()
                    .find(|(i, _)| is_distinct(*i));

                match first_distinct_set {
                    // If there is a distinct set, use it
                    Some((i, _)) => bitfields_with_max_size[i],

                    None => {
                        // Otherwise, we find whichever set has the most elements unique to just
                        // that set relative to the conflicting sets and choose that one.
                        // The elements unique to a set A given B and C is just A & ~(B | C).
                        *bitfields_with_max_size
                            .iter()
                            .enumerate()
                            .max_by_key(|(index, (_, bitfield))| {
                                // The order here appears flipped because BitVec has an impl for BitVec & &BitVec
                                // but not &BitVec & BitVec
                                (!union_of_all_bitfields_except(*index) & *bitfield).count_ones()
                            })
                            .expect("bitfields_with_max_size is empty")
                            .1 // unreachable as the unreachable!() at 0 above would be called
                    }
                }
            }
        };

        output_keys.insert(***selected_set.0);

        // Now, we set the target bitfield to itself minus the values in selected_set.1:
        // TODO: Should we avoid cloning here? Excessive benchmark tests don't show this as a bottleneck
        // Cloning here is required for the '!' operator to work
        target_bitfield &= !selected_set.1.clone();
        let new_preexisting_set_bitfields = preexisting_set_bitfields
            .iter()
            .map(|(key, bitfield)| (*key, !selected_set.1.clone() & bitfield))
            .collect::<HashMap<_, _>>();
        preexisting_set_bitfields = new_preexisting_set_bitfields;
    }

    // Outliers = remaining in target
    // Output sets = output_keys
    UnionizeSetResult {
        // Map each key back to its original in preexisting_sets - this is just to convert from
        // Key to &Key without returning data owned by this fn
        sets: output_keys
            .into_iter()
            .map(|key| preexisting_sets.get_key_value(&key).unwrap().0)
            .collect(),
        // Map each number in the new target bitfield back to a reference to its value from target.
        // This is first done by converting our BitVec to a an iterator over all of the indices,
        // then using the id-to-key map and resolving it back to a reference within target.
        outliers: target_bitfield
            .iter_ones()
            .map(|i| {
                target
                    .get(index_to_value[&i])
                    .expect("target did not contain an outlier")
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

    // Fuzz test with random data. A lot of it (250 sets, 500_000 users)
    // Interestingly enough, this is almost instant on release builds but
    // very very slow on debug builds.
    #[test]
    #[ignore = "extremely slow to run, only run when needed (45+ seconds!)"]
    fn fuzz() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Target set is all numbers 0..=500000
        let target = (0..=500_000).collect::<HashSet<_>>();

        let map = (0..=250) // 250 pre-existing sets
            .into_iter()
            .map(|role| {
                (role, {
                    // Each containing the numbers between these two random points:
                    let lower_bound = rng.gen_range(0..=500_000);
                    let higher_bound = rng.gen_range(lower_bound..=500_000);

                    (lower_bound..=higher_bound).collect::<HashSet<_>>()
                })
            })
            .collect::<HashMap<_, _>>();

        std::hint::black_box(unionize_set(&target, &map));
    }
}
