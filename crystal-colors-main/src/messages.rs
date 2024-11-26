
use chrono::NaiveDateTime;
use diesel::{prelude::*, Queryable, Insertable};
use diesel::pg::PgConnection;
use crystal_colors::schema::messages;


// Create a new user
#[derive(Queryable, Insertable, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[table_name = "messages"]
pub struct DBMessage {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub message: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct InsertMessage<'a> {
    pub user_id: &'a i32,
    pub name: &'a str,
    pub message: &'a str,
}

pub fn create_message(
    connection: &PgConnection,
    user: String,
    user_id: i32,
    message: &str,
) -> Result<DBMessage, diesel::result::Error> {

    let message = InsertMessage {
        user_id: &user_id,
        name: &user,
        message,
    };

    diesel::insert_into(messages::table).values(&message).get_result(connection)
}