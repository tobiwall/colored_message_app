use crate::messages::create_message;
use crate::messages::DBMessage;
use crate::users;
use crate::users::create_user;
use crystal_colors;
use crystal_colors::auth;
use crystal_colors::auth::password::check_password;
use diesel::query_dsl::methods::FilterDsl;
use diesel::ExpressionMethods;
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

pub fn handle_login(name: String, password: String, conn: DBConnection) -> bool {
    let password_db = get_user_password_db(name, conn);
    match password_db {
        Ok(Some(res)) => {
            let verify = check_password(&password, &res);
            if verify == Ok(()) {
                println!("Your login is successfully completed");
                true
            } else {
                println!("Incorrect password");
                false
            }
        }
        Ok(None) => {
            println!("User not found");
            false
        }
        Err(e) => {
            println!("Get user failed: {}", e);
            false
        } 
    }
}

fn get_user_password_db(name: String, conn: DBConnection) -> Result<Option<String>, anyhow::Error> {
    let user_from_db = get_user(name, conn)?;
    if let Some(user) = user_from_db {
        return Ok(Some(user.password));
    }
    Ok(None)
}

fn get_user(user_name: String, conn: DBConnection) -> Result<Option<User>, diesel::result::Error> {
    use crystal_colors::schema::users::dsl::*;
    users
        .filter(name.eq(user_name))
        .load::<User>(&conn)
        .map(|mut res| res.pop())
}

pub fn save_messages(user: String, message: String, connection: DBConnection) {
    match create_message(&connection, &user, &message) {
        Ok(messages) => println!("This are the messages {:?}", messages),
        Err(e) => println!("This is the save_messages error {e}"),
    }
}

pub fn save_new_user(name: String, password: String, connection: DBConnection) {
    match create_user(&connection, &name, &password) {
        Ok(user) => println!("Created user: {}", user.name),
        Err(err) => println!("Error: {}", err),
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FrontendMessage {
    user: String,
    message: String,
}

pub fn get_message_db(pool: &DbPool) -> Result<Vec<FrontendMessage>, io::Error> {
    let message_from_db = get_messages(pool).unwrap();
    let message_new = convert_messages(message_from_db);
    Ok(message_new)
}

pub fn get_messages(pool: &DbPool) -> Result<Vec<DBMessage>, diesel::result::Error> {
    use crystal_colors::schema::messages::dsl::*;
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    messages.load::<DBMessage>(&conn)
}

fn convert_messages(messages: Vec<DBMessage>) -> Vec<FrontendMessage> {
    messages
        .into_iter()
        .map(|msg| FrontendMessage {
            user: msg.name,
            message: msg.message,
        })
        .collect()
}

pub fn set_hash_password(password: &str) -> String {
    auth::password::hash_password(password).unwrap()
}
