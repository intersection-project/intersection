use crate::schema::guilds;
use diesel::prelude::*;

#[derive(Queryable, Debug)]
pub struct Guild {
    pub id: String,
    pub prefix: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = guilds)]
pub struct NewGuild<'a> {
    pub id: &'a str,
    pub prefix: Option<&'a str>,
}

impl<'a> From<NewGuild<'a>> for Guild {
    fn from(val: NewGuild<'a>) -> Self {
        Guild {
            id: val.id.to_string(),
            prefix: val.prefix.map(|s| s.to_string()),
        }
    }
}
