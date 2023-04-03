use async_recursion::async_recursion;
use std::{collections::HashSet, future::Future, hash::Hash};

pub enum ReducerOp<User> {
    Difference(Box<ReducerOp<User>>, Box<ReducerOp<User>>),
    Intersection(Box<ReducerOp<User>>, Box<ReducerOp<User>>),
    Union(Box<ReducerOp<User>>, Box<ReducerOp<User>>),
    User(User),
}

#[async_recursion]
pub async fn run_reducers<'user_data, User, Output, F, FnFut, UserData, E>(
    node: ReducerOp<User>,
    f: &F,
    data: &'user_data UserData,
) -> Result<HashSet<Output>, E>
where
    User: Eq + Hash + Send + Sync,
    F: Fn(User, &'user_data UserData) -> FnFut + Send + Sync,
    FnFut: Future<Output = Result<HashSet<Output>, E>> + Send,
    UserData: 'user_data + Send + Sync,
    Output: Eq + Hash + Copy + Send + Sync,
    E: Send + Sync,
{
    Ok(match node {
        ReducerOp::Difference(l, r) => run_reducers(*l, f, data)
            .await?
            .difference(&run_reducers(*r, f, data).await?)
            .copied()
            .collect::<HashSet<_>>(),
        ReducerOp::Intersection(l, r) => run_reducers(*l, f, data)
            .await?
            .intersection(&run_reducers(*r, f, data).await?)
            .copied()
            .collect::<HashSet<_>>(),
        ReducerOp::Union(l, r) => run_reducers(*l, f, data)
            .await?
            .union(&run_reducers(*r, f, data).await?)
            .copied()
            .collect::<HashSet<_>>(),
        ReducerOp::User(u) => f(u, data).await?,
    })
}
