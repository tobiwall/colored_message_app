use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2::Pool;
use r2d2::PooledConnection;
use rocket::fs::NamedFile;
use rocket::futures::SinkExt;
use rocket::futures::StreamExt;
use rocket::response::content;
use rocket::*;
use rocket_ws as ws;
use ::serde::{Deserialize, Serialize};
use anyhow::Result;
use serde_json::Value;
use std::fs::File;
use std::io;
use std::io::Error;
use std::io::Read;
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use dashmap::DashMap as HashMap;
use rocket::State;

pub mod messages;
pub mod msac;
pub mod database_handling;
pub mod users;


type Channels = Arc<HashMap<String, msac::Channel>>;
type LastMessages = Arc<HashMap<String, String>>;
type DbPool = Pool<ConnectionManager<PgConnection>>;
type DBConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(::serde::Deserialize)]
#[serde(tag = "type")]
enum IncomingMessage {
    Login { name: String, password: String },
    NewUser { name: String, password: String },
    Color { value: String },
    Message { user_id: String, user: String, message: String },
}

async fn handle_message(message: String, pool: &State<DbPool>, tx: &tokio::sync::mpsc::Sender<String>) {
    println!("This is the handle_message message: {message}");
    let conn: DBConnection = pool.get().expect("Failed to get connection from pool");
    match serde_json::from_str::<IncomingMessage>(&message) {
        Ok(IncomingMessage::Login { name, password }) => database_handling::handle_login(name, password, conn, tx).await,
        Ok(IncomingMessage::NewUser { name, password }) => database_handling::save_new_user(name, password, conn, tx).await,
        Ok(IncomingMessage::Color { value }) => save_color(value.clone(), tx, message).await.unwrap(),
        Ok(IncomingMessage::Message { user_id, user, message }) => database_handling::save_messages(user_id.parse().expect("Failed to parse the user_id"), user, message, conn, tx).await,
        Err(e) => println!("Error parsing: {e}"),
    }
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
    all_messages: &'r State<Arc<Mutex<Vec<database_handling::FrontendMessage>>>>,
    color: &'r State<Arc<Mutex<String>>>,
    pool: &'r State<DbPool>,
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
                println!("This is msg: {:?}", msg);
                if (stream
                    .send(rocket_ws::Message::Text(
                        serde_json::json!({
                            "type": "MessageResponse",
                            "chat_message": msg.message,
                            "user": msg.user
                        }).to_string(),
                    ))
                    .await)
                    .is_err()
                {
                    break;
                }
            }
            {
                if stream
                    .send(rocket_ws::Message::Text(json_color.clone()))
                    .await
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
                            println!("This is the loop message: {}", message);
                            if let rocket_ws::Message::Text(message) = message {
                                let mut message_with_userid: Value = serde_json::from_str(&message).unwrap();
                                if let Some(user_id_str) = message_with_userid["user_id"].as_str() {
                                    if let Ok(user_id) = user_id_str.parse::<i32>() {
                                        message_with_userid["user_id"] = serde_json::json!(user_id);
                                    }
                                }
                                let modified_message_str = serde_json::to_string(&message_with_userid).unwrap();
                                if let Ok(new_message) = serde_json::from_str::<database_handling::FrontendMessage>(&modified_message_str) {
                                    let mut messages = all_messages.lock().await;
                                    messages.push(new_message.clone());
                                    println!("This is the 1 message before handle_message: {message}");
                                    handle_message(message, pool, &tx).await;
                                } else {
                                    let json_value: Value = serde_json::from_str(&message).unwrap();
                                    if let Some(type_value) = json_value.get("type") {
                                        if let Some(type_str) = type_value.as_str() {
                                            if type_str == "Color" {
                                                let new_color = message.parse::<String>().unwrap();
                                                let mut color_watch = color.lock().await;
                                                *color_watch = new_color.clone();
                                                println!("This is the 2 message before handle_message: {new_color}");
                                                handle_message(new_color, pool, &tx).await;
                                            } else {
                                                println!("This is the 3 message before handle_message: {message}");
                                                handle_message(message, pool, &tx).await;
                                            }
                                        }
                                    }
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

async fn save_color(color: String, tx: &tokio::sync::mpsc::Sender<String>, message: String) -> Result<(), Error> {
    let file =
        File::create("color.json").inspect_err(|_| println!("Failed to create message.json"))?;
    serde_json::to_writer(file, &color)?;
    println!("Color {color}");
    tx.send(message.clone()).await.unwrap();
    Ok(())
}

fn read_color_from_file() -> Result<String, io::Error> {
    let mut file = File::open("color.json")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let color = serde_json::from_str(&content)?;
    Ok(color)
}

#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let pool: DbPool = {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool")
    };

    let all_messages = Arc::new(Mutex::new(database_handling::get_message_db(&pool).unwrap()));
    let color = Arc::new(Mutex::new(read_color_from_file().unwrap()));
    let channels: Channels = Arc::new(HashMap::new());
    let last_messages: LastMessages = Arc::new(HashMap::new());

    let rocket = rocket::build()
        .manage(pool)
        .manage(all_messages)
        .manage(color)
        .manage(channels)
        .manage(last_messages)
        .mount("/", rocket::routes![serve, index, echo_socket]);

    Ok(rocket.into())
}
