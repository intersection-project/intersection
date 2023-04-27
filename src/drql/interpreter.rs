use async_recursion::async_recursion;
use std::{collections::HashSet, future::Future, hash::Hash};

pub enum ReducerOp<User> {
    Difference(Box<ReducerOp<User>>, Box<ReducerOp<User>>),
    Intersection(Box<ReducerOp<User>>, Box<ReducerOp<User>>),
    Union(Box<ReducerOp<User>>, Box<ReducerOp<User>>),
    User(User),
}

/// Takes a [ReducerOp] and a function that takes the user-defined type and iterates over every
/// value within it, and returns a [Future] that resolves to a [HashSet] of the values after
/// calculating them as a set. This is best explained with an example:
///
/// ```
/// use drql::interpreter::{ReducerOp, interpret};
///
/// let tree = ReducerOp::Union(
///     Box::new(ReducerOp::User(HashSet::from([1]))),
///     Box::new(ReducerOp::Difference(
///         Box::new(ReducerOp::User(HashSet::from([2, 3, 4]))),
///         Box::new(ReducerOp::User(HashSet::from([3]))),
///     )),
/// );
///
/// struct UserData;
///
/// async fn f(input: HashSet<u32>, _: &UserData) -> Result<HashSet<u32>, !> {
///     Ok(input)
/// }
///
/// assert_eq!(
///     interpret(tree, &f, &UserData).await,
///     Ok(HashSet::from([1, 2, 4])
/// );
/// ```
///
/// The "user data" is passed into `f` for all calls.
#[must_use]
#[async_recursion]
pub async fn interpret<'user_data, User, Output, F, FnFut, UserData, E>(
    node: ReducerOp<User>,
    f: &F,
    data: &'user_data UserData,
) -> Result<HashSet<Output>, E>
where
    User: Send + Sync,
    F: Fn(User, &'user_data UserData) -> FnFut + Send + Sync,
    FnFut: Future<Output = Result<HashSet<Output>, E>> + Send,
    UserData: 'user_data + Send + Sync,
    Output: Eq + Hash + Copy + Send + Sync,
    E: Send + Sync,
{
    Ok(match node {
        ReducerOp::Difference(l, r) => interpret(*l, f, data)
            .await?
            .difference(&interpret(*r, f, data).await?)
            .copied()
            .collect::<HashSet<_>>(),
        ReducerOp::Intersection(l, r) => interpret(*l, f, data)
            .await?
            .intersection(&interpret(*r, f, data).await?)
            .copied()
            .collect::<HashSet<_>>(),
        ReducerOp::Union(l, r) => interpret(*l, f, data)
            .await?
            .union(&interpret(*r, f, data).await?)
            .copied()
            .collect::<HashSet<_>>(),
        ReducerOp::User(u) => f(u, data).await?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reducers_work_as_expected() {
        let tree = ReducerOp::Union(
            Box::new(ReducerOp::User(HashSet::from([1]))),
            Box::new(ReducerOp::Difference(
                Box::new(ReducerOp::User(HashSet::from([2, 3, 4]))),
                Box::new(ReducerOp::User(HashSet::from([3]))),
            )),
        );

        struct UserData;

        async fn f(input: HashSet<u32>, _: &UserData) -> Result<HashSet<u32>, ()> {
            Ok(input)
        }

        assert_eq!(
            interpret(tree, &f, &UserData).await,
            Ok(HashSet::from([1, 2, 4]))
        );
    }
}
