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

impl<'a> Into<Guild> for NewGuild<'a> {
    fn into(self) -> Guild {
        Guild {
            id: self.id.to_string(),
            prefix: self.prefix.map(|s| s.to_string()),
        }
    }
}
