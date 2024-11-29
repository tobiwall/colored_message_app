use crate::schema::messages;
use crate::users::SignupResult;
use crate::{database_handling, save_color_to_file, DBConnection, DbPool};
use chrono::NaiveDateTime;
use diesel::pg::PgConnection;
use diesel::{prelude::*, Insertable, Queryable};

use anyhow::Result;
use futures::SinkExt;
use rocket::State;
use rocket::*;
use rocket_ws::stream::DuplexStream;

#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
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
    stream: &mut DuplexStream,
) {
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    match message {
        Message::Login { name, password } => {
            if let Err(e) = stream
                .send(rocket_ws::Message::Text(
                    serde_json::to_string(
                        &database_handling::handle_login(name, password, conn).await,
                    )
                    .unwrap(),
                ))
                .await
            {
                println!("Error sending message {:?}: {:?}", message, e);
            }
        }
        Message::NewUser { name, password } => {
            let result = crate::users::create_user(&conn, name, password);
            let response = match result {
                SignupResult::Success(user_id) => serde_json::json!({
                    "type": "NewUserResponse",
                    "user_id": user_id,
                })
                .to_string(),
                SignupResult::Failure(error_message) => serde_json::json!({
                    "type": "NewUserResponse",
                    "error": error_message,
                })
                .to_string(),
            };
            if let Err(e) = stream.send(rocket_ws::Message::Text(response)).await {
                println!("Error sending message {:?}: {:?}", message, e);
            }
        }
        Message::Color { value } => save_color_to_file(&value).await.unwrap(),
        Message::Message { user_id, message } => {
            let saved_message = database_handling::save_messages(*user_id, message, conn)
                .await
                .unwrap();
            let message_with_type = serde_json::json!({
                "type": "MessageResponse",
                "user": saved_message.user_id,
                "chat_message": saved_message.message,
                "msg_id": saved_message.msg_id,
            });
            if let Err(e) = tx
                .send(rocket_ws::Message::Text(message_with_type.to_string()).to_string())
                .await
            {
                println!("Error sending message {:?}: {:?}", message, e);
            }
        }
    };
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
