use crystal_colors::auth;
use crystal_colors::auth::password::check_password;
use ::r2d2::PooledConnection;
use dashmap::DashMap as HashMap;
use messages::create_message;
use messages::DBMessage;
use rocket::fs::NamedFile;
use rocket::futures::SinkExt;
use rocket::futures::StreamExt;
use rocket::response::content;
use rocket::*;
use rocket_ws as ws;
use ::serde::{Deserialize, Serialize};
use anyhow::Result;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use once_cell::sync::Lazy;
use r2d2::Pool;
use serde_json::Value;
use users::User;
use std::fs::File;
use std::io;
use std::io::Error;
use std::io::Read;
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;

pub mod messages;
pub mod msac;
pub mod users;

use users::create_user;

type LastMessages = Arc<HashMap<String, String>>;
type Channels = Arc<HashMap<String, msac::Channel>>;
type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(::serde::Deserialize)]
#[serde(tag = "type")]
enum IncomingMessage {
    Login { name: String, password: String },
    NewUser { name: String, password: String },
    Color { value: String },
    Message { user: String, message: String },
}

async fn handle_message(message: String) {
    let conn = POOL.get().expect("Failed to get connection from pool");

    match serde_json::from_str::<IncomingMessage>(&message) {
        Ok(IncomingMessage::Login { name, password }) => handle_login(name, password),
        Ok(IncomingMessage::NewUser { name, password }) => save_new_user(name, password, conn),
        Ok(IncomingMessage::Color { value }) => save_color(value).unwrap(),
        Ok(IncomingMessage::Message { user, message }) => save_messages(user, message, conn),
        Err(e) => println!("Error parsing: {e}"),
    }
}

fn handle_login(name: String, password: String) {
    let password_db = get_user_password_db(name);
    match password_db {
        Ok(Some(res)) => {
            let verify = check_password(&password, &res);
            if verify == Ok(()) {
                println!("Your login is successfully completed");
            } else {
                println!("Incorrect password")
            }
        }
        Ok(None) => println!("User not found"),
        Err(e) => println!("Get user failed: {}", e),
    }
}

static POOL: Lazy<DbPool> = Lazy::new(|| {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool")
});

fn save_messages(
    user: String,
    message: String,
    connection: r2d2::PooledConnection<ConnectionManager<PgConnection>>,
) {
    match create_message(&connection, &user, &message) {
        Ok(messages) => println!("This are the messages {:?}", messages),
        Err(e) => println!("This is the save_messages error {e}"),
    }
}

fn save_new_user(
    name: String,
    password: String,
    connection: r2d2::PooledConnection<ConnectionManager<PgConnection>>,
) {
    match create_user(&connection, &name, &password) {
        Ok(user) => println!("Created user: {}", user.name),
        Err(err) => println!("Error: {}", err),
    }
}

fn get_messages() -> Result<Vec<DBMessage>, diesel::result::Error> {
    use crystal_colors::schema::messages::dsl::*;
    let conn: PooledConnection<ConnectionManager<PgConnection>> =
        POOL.get().expect("Failed to get connection from pool");
    messages.load::<DBMessage>(&conn)
}

fn get_user(user_name: String) -> Result<Option<User>, diesel::result::Error> {
    use crystal_colors::schema::users::dsl::*;
    let conn: PooledConnection<ConnectionManager<PgConnection>> =
        POOL.get().expect("Failed to get connection from pool");
    users.filter(name.eq(user_name)).load::<User>(&conn).map(|mut res| res.pop())
}

#[get("/main.js")]
pub async fn serve() -> Option<NamedFile> {
    NamedFile::open("assets/main.js").await.ok()
}

#[get("/room/<room>")]
pub async fn index(
    room: String,
    last_messages: &State<LastMessages>,
) -> Option<content::RawHtml<String>> {
    let mut file = match std::fs::read_to_string("assets/index.html") {
        Ok(file) => file,
        Err(_) => return None,
    };

    // get last message for room. If it does not exist, replace "StartValue" with 180
    let last_message = last_messages
        .get(&room)
        .map(|x| x.clone())
        .unwrap_or("180".to_string());
    // replace "StartValue" with last message
    file = file.replace("StartValue", &last_message);

    Some(content::RawHtml(file))
}

#[get("/echo/<room>")]
async fn echo_socket<'r>(
    ws: ws::WebSocket,
    room: String,
    channels: &'r State<Channels>,
    last_messages: &'r State<LastMessages>,
    all_messages: &'r State<Arc<Mutex<Vec<FrontendMessage>>>>,
    color: &'r State<Arc<Mutex<String>>>,
) -> ws::Channel<'r> {
    let (tx, mut rx) = {
        let channel = channels
            .entry(room.clone())
            .or_default();
        channel.add().await
    };

#[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct ColorStruct {
        r#type: String,
        value: String,
    }

    let messages = all_messages.lock().await.clone();
    let mut current_color = color.lock().await.clone();
    let color_type = serde_json::from_str::<Value>(&current_color).unwrap();
    if color_type["type"] == "Color" {
        current_color = color_type["value"].as_str().unwrap().to_string();
    }
    let color_message = ColorStruct {r#type: "Color".to_string(), value: current_color};
    let json_color = serde_json::to_string(&color_message).unwrap();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            for msg in &messages {
                if (stream
                    .send(rocket_ws::Message::Text(
                        serde_json::to_string(msg).unwrap(),
                    ))
                    .await)
                    .is_err()
                {
                    break;
                }
            }
            {
                if (stream
                    .send(rocket_ws::Message::Text(json_color.clone()))
                    .await)
                    .is_err()
                {
                    return Ok(());
                }
            }

            loop {
                select! {
                    message = stream.next() => {
                        if let Some(message) = message {
                            let message = message.unwrap();

                            // check, if the message is a string
                            if let rocket_ws::Message::Text(message) = message {
                                if let Ok(new_message) = serde_json::from_str::<FrontendMessage>(&message) {
                                    tx.send(message.to_string()).await.unwrap();
                                    let mut messages = all_messages.lock().await;
                                    messages.push(new_message.clone());
                                    handle_message(message).await;
                                } else {
                                    let new_color = message.parse::<String>().unwrap();
                                    tx.send(new_color.clone()).await.unwrap();
                                    let mut color_watch = color.lock().await;
                                    *color_watch = new_color.clone();
                                    handle_message(new_color).await;
                                }
                            
                            }
                        } else {
                            break;
                        }
                    },
                    message = rx.recv() => {
                        if let Some(message) = message {
                            if (stream.send(rocket_ws::Message::Text(message)).await).is_err() {
                                break;
                            }
                        }
                    }
                }
            }

            // remove the connection from the channel
            let channel = channels.get_mut(&room).unwrap();
            if channel.remove().await {
                println!("removing channel for room {}", room);
                channels.remove(&room);
                last_messages.remove(&room);
            }
            Ok(())
        })
    })
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct FrontendMessage {
    user: String,
    message: String,
}

fn save_color(color: String) -> Result<(), Error> {
    let file =
        File::create("color.json").inspect_err(|_| println!("Failed to create message.json"))?;
    serde_json::to_writer(file, &color)?;
    println!("Color {color}");
    Ok(())
}

fn read_color_from_file() -> Result<String, io::Error> {
    let mut file = File::open("color.json")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let color = serde_json::from_str(&content)?;
    Ok(color)
}

fn get_message_db() -> Result<Vec<FrontendMessage>, io::Error> {
    let message_from_db = get_messages().unwrap();
    let message_new = convert_messages(message_from_db);
    Ok(message_new)
}

fn get_user_password_db(name: String) -> Result<Option<String>, anyhow::Error> {
    let user_from_db = get_user(name)?;
        if let Some(user) = user_from_db {
            return Ok(Some(user.password));
        }
        Ok(None)
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

fn set_hash_password(password: &str) -> String {
    auth::password::hash_password(password).unwrap()
}

#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let all_messages = Arc::new(Mutex::new(get_message_db().unwrap()));
    let color = Arc::new(Mutex::new(read_color_from_file().unwrap()));
    let rocket = rocket::build()
        .manage(all_messages)
        .manage(color)
        .mount("/", rocket::routes![serve, index, echo_socket]);
    // manage a mpmc channel for strings
    let channels: Channels = Arc::new(HashMap::new());
    let rocket = rocket.manage(channels);
    // manage a sting for the last sent message
    let last_messages: LastMessages = Arc::new(HashMap::new());
    let rocket = rocket.manage(last_messages);
    Ok(rocket.into())
}
