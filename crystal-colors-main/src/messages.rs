
use chrono::NaiveDateTime;
use diesel::{prelude::*, Queryable, Insertable};
use diesel::pg::PgConnection;
use crystal_colors::schema::messages;


// Create a new user
#[derive(Queryable, Insertable, Debug, Clone, serde::Deserialize)]
#[table_name = "messages"]
pub struct DBMessage {
    pub id: i32,
    pub name: String,
    pub message: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct InsertMessage<'a> {
    pub name: &'a str,
    pub message: &'a str,
}

pub fn create_message(
    connection: &PgConnection,
    name: &str,
    message: &str,
) -> Result<DBMessage, diesel::result::Error> {

    let message = InsertMessage {
        name,
        message,
    };

    diesel::insert_into(messages::table).values(&message).get_result(connection)
}