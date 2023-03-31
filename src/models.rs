use crate::schema::guilds;
use diesel::prelude::*;

#[derive(Queryable, Debug)]
pub struct GuildDBData {
    pub id: String,
    pub prefix: String,
}

#[derive(Insertable)]
#[diesel(table_name = guilds)]
pub struct NewGuild<'a> {
    pub id: &'a str,
    pub prefix: Option<&'a str>,
}
