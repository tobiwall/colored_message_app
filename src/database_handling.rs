use crate::messages::insert_message_to_db;
use crate::messages::DBMessage;
use crate::password::*;
use crate::users;
use diesel::query_dsl::methods::FilterDsl;
use diesel::ExpressionMethods;
use diesel::Queryable;
use diesel::RunQueryDsl;
use diesel::{pg::PgConnection, r2d2::ConnectionManager};
use r2d2::Pool;
use r2d2::PooledConnection;
use serde::Deserialize;
use serde::Serialize;
use std::io;
use users::User;

type DbPool = Pool<ConnectionManager<PgConnection>>;
type DBConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Serialize, Debug)]
pub enum LoginResult {
    /// Login was successful. Contains the user id.
    Success(i32),
    /// Login was unsuccessful. Contains the error message.
    Failure(String),
}

pub async fn handle_login(name: &str, password: &str, conn: DBConnection) -> LoginResult {
    let password_db = get_user(name, &conn);
    match password_db {
        Ok(Some(res)) => {
            let verify = check_password(password, &res.password);
            if verify == Ok(()) {
                println!("Your login is successfully completed");
                LoginResult::Success(res.id)
            } else {
                println!("Incorrect password");
                LoginResult::Failure("Incorrect password".to_string())
            }
        }
        Ok(None) => {
            println!("User not found");
            LoginResult::Failure("User not found".to_string())
        }
        Err(e) => {
            println!("Get user failed: {}", e);
            LoginResult::Failure(format!("Get user failed: {}", e))
        }
    }
}

fn get_user(user_name: &str, conn: &DBConnection) -> Result<Option<User>, diesel::result::Error> {
    use crate::schema::users::dsl::*;
    users
        .filter(name.eq(user_name))
        .load::<User>(conn)
        .map(|mut res| res.pop())
}

#[derive(Queryable, Debug, serde::Serialize, Clone)]
pub struct FrontendUser {
    pub id: i32,
    pub name: String,
}

pub fn get_all_users(pool: &DbPool) -> Result<Vec<FrontendUser>, diesel::result::Error> {
    use crate::schema::users::dsl::*;
    use diesel::query_dsl::methods::SelectDsl;
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    users.select((id, name)).load::<FrontendUser>(&conn)
}

pub async fn save_messages(
    user_id: i32,
    message: &str,
    connection: DBConnection,
) -> Result<FrontendMessage, io::Error> {
    match insert_message_to_db(&connection, user_id, message) {
        Ok(message) => Ok(FrontendMessage {
            user_id,
            message: message.message,
            msg_id: message.id,
        }),
        Err(e) => {
            println!("This is the save_messages error {e}");
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to save message",
            ))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FrontendMessage {
    pub user_id: i32,
    pub message: String,
    pub msg_id: i32,
}

impl From<DBMessage> for FrontendMessage {
    fn from(msg: DBMessage) -> Self {
        FrontendMessage {
            user_id: msg.user_id,
            message: msg.message,
            msg_id: msg.id,
        }
    }
}

pub fn get_message_db(pool: &DbPool) -> Result<Vec<FrontendMessage>, io::Error> {
    let message_from_db = get_messages(pool).unwrap();
    let message_new = convert_messages(message_from_db);
    Ok(message_new)
}

pub fn get_messages(pool: &DbPool) -> Result<Vec<DBMessage>, diesel::result::Error> {
    use crate::schema::messages::dsl::*;
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    messages.load::<DBMessage>(&conn)
}

fn convert_messages(messages: Vec<DBMessage>) -> Vec<FrontendMessage> {
    messages.into_iter().map(|x| x.into()).collect()
}

pub fn get_messages_range(
    pool: &DbPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<FrontendMessage>, diesel::result::Error> {
    use crate::schema::messages::dsl::*;
    use diesel::prelude::*;
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    let message_from_db = messages
        .order(id.desc())
        .limit(limit)
        .offset(offset)
        .load::<DBMessage>(&conn)
        .unwrap();
    let message_new = convert_messages(message_from_db);
    Ok(message_new)
}
