use diesel::prelude::*;

#[derive(Queryable, Debug)]
pub struct Guild {
    pub id: String,
    pub prefix: Option<String>,
}
