
use chrono::NaiveDateTime;
use diesel::{prelude::*, Queryable, Insertable};
use diesel::pg::PgConnection;
use crystal_colors::schema::messages;


// Create a new user
#[derive(Queryable, Insertable, Debug)]
#[table_name = "messages"]
pub struct Message {
    pub id: i32,
    pub name: String,
    pub message: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct NewMessage<'a> {
    pub name: &'a str,
    pub message: &'a str,
}

pub fn create_message(
    connection: &PgConnection,
    name: &str,
    message: &str,
) -> Result<Message, diesel::result::Error> {

    let new_message = NewMessage {
        name,
        message,
    };

    diesel::insert_into(messages::table).values(&new_message).get_result(connection)
}