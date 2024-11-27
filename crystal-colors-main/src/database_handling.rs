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

#[derive(serde::Serialize, Debug)]
pub struct LoginResult {
    pub success: bool,
    user_id: i32,
    pub message: String,
}

pub async fn handle_login(
    name: String,
    password: String,
    conn: DBConnection,
    tx: &tokio::sync::mpsc::Sender<String>,
) {
    let password_db = get_user_password_db(name.clone(), &conn);
    let user_id = get_user_id(name, conn);
    let login_result: LoginResult;
    match password_db {
        Ok(Some(res)) => {
            let verify = check_password(&password, &res);
            if verify == Ok(()) {
                println!("Your login is successfully completed");
                login_result = LoginResult {
                    success: true,
                    user_id,
                    message: "Your login is successfully completed".to_string(),
                }
            } else {
                println!("Incorrect password");
                login_result = LoginResult {
                    success: false,
                    user_id: -1,
                    message: "Incorrect password".to_string(),
                }
            }
        }
        Ok(None) => {
            println!("User not found");
            login_result = LoginResult {
                success: false,
                user_id: -1,
                message: "User not found".to_string(),
            }
        }
        Err(e) => {
            println!("Get user failed: {}", e);
            login_result = LoginResult {
                success: false,
                user_id: -1,
                message: format!("Get user failed: {}", e),
            }
        }
    }
    if let Err(e) = tx
        .send(
            serde_json::json!({
                "type": "LoginResponse",
                "success": login_result.success,
                "user_id": login_result.user_id,
                "login_message": login_result.message
            })
            .to_string(),
        )
        .await
    {
        println!("Failed to send message: {e}");
    }
}

fn get_user_password_db(name: String, conn: &DBConnection) -> Result<Option<String>, anyhow::Error> {
    let user_from_db = get_user(name.clone(), conn)?;
    if let Some(user) = user_from_db {
        return Ok(Some(user.password));
    }
    Ok(None)
}

fn get_user_id(name: String, conn: DBConnection) -> i32 {
    let user_from_db = get_user(name, &conn);
    if let Ok(Some(user)) = user_from_db {
        return user.id;
    }
    -1
}

pub fn find_user_name(user_id: i32, conn: &DBConnection) -> Result<String, diesel::result::Error> {
    use diesel::query_dsl::methods::SelectDsl;
    use crystal_colors::schema::users::dsl::*;
    users
        .filter(id.eq(user_id))
        .select(name)
        .first::<String>(conn)
}

fn get_user(user_name: String, conn: &DBConnection) -> Result<Option<User>, diesel::result::Error> {
    use crystal_colors::schema::users::dsl::*;
    users
        .filter(name.eq(user_name))
        .load::<User>(conn)
        .map(|mut res| res.pop())
}

pub async fn save_messages(
    user_id: i32,
    message: String,
    connection: DBConnection,
    tx: &tokio::sync::mpsc::Sender<String>,
) {
    let frontend_message: FrontendMessage;
    match create_message(&connection, user_id, &message) {
        Ok(message) => {
            frontend_message = FrontendMessage {
                user_id,
                message: message.message,
            }
        }
        Err(e) => {
            frontend_message = FrontendMessage {
                user_id: -1,
                message: e.to_string(),
            };
            println!("This is the save_messages error {e}")
        }
    };
    let user_name = find_user_name(user_id, &connection).unwrap();
    if let Err(e) = tx
        .send(
            serde_json::json!({
                "type": "MessageResponse",
                "chat_message": frontend_message.message,
                "user_id": frontend_message.user_id,
                "user_name": user_name,
            })
            .to_string(),
        )
        .await
    {
        println!("Failed to send message: {e}");
    }
}

struct Signup {
    success: bool,
    message: String,
}

pub async fn save_new_user(
    name: String,
    password: String,
    connection: DBConnection,
    tx: &tokio::sync::mpsc::Sender<String>,
) {
    let signup_result: Signup = match create_user(&connection, &name, &password) {
        Ok(user) => {
            println!("Created user: {}", user.name);
            Signup {
                success: true,
                message: "Created user".to_string(),
            }
        }
        Err(err) => {
            println!("Error: {}", err);
            Signup {
                success: false,
                message: "User with this name already exists.".to_string(),
            }
        }
    };
    if let Err(e) = tx
        .send(
            serde_json::json!({
                "type": "NewUserResponse",
                "success": signup_result.success,
                "signup_message": signup_result.message
            })
            .to_string(),
        )
        .await
    {
        println!("Failed to send message: {e}");
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FrontendMessage {
    pub user_id: i32,
    pub message: String,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FrontendMessageTest {
    pub user_id: String,
    pub user: String,
    pub message: String,
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
            user_id: msg.user_id,
            message: msg.message,
        })
        .collect()
}

pub fn set_hash_password(password: &str) -> String {
    auth::password::hash_password(password).unwrap()
}
