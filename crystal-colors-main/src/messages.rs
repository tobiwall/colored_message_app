use crate::schema::messages;
use crate::{database_handling, save_color_to_file, DBConnection, DbPool};
use chrono::NaiveDateTime;
use diesel::pg::PgConnection;
use diesel::{prelude::*, Insertable, Queryable};

use anyhow::Result;
use futures::SinkExt;
use rocket::State;
use rocket::*;
use rocket_ws::stream::DuplexStream;

#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(tag = "type")]
pub enum Message {
    Login { name: String, password: String },
    NewUser { name: String, password: String },
    Color { value: String },
    Message { user_id: i32, message: String },
}

pub async fn handle_message(
    message: &Message,
    pool: &State<DbPool>,
    tx: &tokio::sync::mpsc::Sender<String>,
    stream: &DuplexStream,
) {
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    match message {
        Message::Login { name, password } => {
            database_handling::handle_login(name, password, conn, stream).await
        }
        Message::NewUser { name, password } => {
            crate::users::create_user(&conn, name, password)
        }
        Message::Color { value } => save_color_to_file(&value).await.unwrap(),
        Message::Message { user_id, message } => {
            stream
                .send(rocket_ws::Message::Text(
                    serde_json::to_string(
                        &database_handling::save_messages(*user_id, message, conn)
                            .await
                            .unwrap(),
                    )
                    .unwrap(),
                ))
                .await;
        }
    }
}

// Create a new user
#[derive(Queryable, Insertable, Debug, Clone, ::serde::Deserialize, ::serde::Serialize)]
#[table_name = "messages"]
pub struct DBMessage {
    pub id: i32,
    pub user_id: i32,
    pub message: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct InsertMessage<'a> {
    pub user_id: &'a i32,
    pub message: &'a str,
}

pub fn insert_message_to_db(
    connection: &PgConnection,
    user_id: i32,
    message: &str,
) -> Result<DBMessage, diesel::result::Error> {
    let message = InsertMessage {
        user_id: &user_id,
        message,
    };

    diesel::insert_into(messages::table)
        .values(&message)
        .get_result(connection)
}
