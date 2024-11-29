#[macro_use] extern crate diesel;

use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use messages::{handle_message, Message};
use r2d2::{Pool, PooledConnection};
use rocket::fs::NamedFile;
use rocket::futures::{ SinkExt, StreamExt };
use rocket::response::content;
use rocket::*;
use rocket_ws as ws;
use anyhow::Result;
use std::fs::File;
use std::io;
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
pub mod schema;
pub mod password;


type Channels = Arc<HashMap<String, msac::Channel>>;
type LastMessages = Arc<HashMap<String, String>>;
pub type DbPool = Pool<ConnectionManager<PgConnection>>;
type DBConnection = PooledConnection<ConnectionManager<PgConnection>>;

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
    all_users: &'r State<Arc<Mutex<Vec<database_handling::FrontendUser>>>>,
    color: &'r State<Arc<Mutex<String>>>,
    pool: &'r State<DbPool>,
) -> ws::Channel<'r> {
    let (tx, mut rx) = {
        let channel = channels
            .entry(room.clone())
            .or_default();
        channel.add().await
    };

    let users = all_users.lock().await.clone();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            for user in &users {
                if (stream
                    .send(rocket_ws::Message::Text(
                        serde_json::json!({
                            "type": "AllUsers",
                            "user_id": user.id,
                            "user_name": user.name,
                        }).to_string(),
                    ))
                    .await)
                    .is_err()
                {
                    break;
                }
            }
            for msg in database_handling::get_messages_range(&pool, 2, 0).unwrap().iter() {
                if (stream
                    .send(rocket_ws::Message::Text(
                        serde_json::json!({
                            "type": "MessageResponse",
                            "user": msg.user_id,
                            "chat_message": msg.message,
                            "msg_id": msg.msg_id,
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
                    .send(rocket_ws::Message::Text(serde_json::to_string(
                            &Message::Color { value: color.lock().await.clone() }
                        ).expect("Failed to serialize color"))
                    ).await
                    .is_err()
                {
                    return Ok(());
                }
            }

            loop {
                select! {
                    Some(Ok(message)) = stream.next() => {
                        if let rocket_ws::Message::Text(message) = message {
                            match serde_json::from_str::<Message>(&message) {
                                Ok(Message::Color { value }) => {
                                    let mut color_watch = color.lock().await;
                                    *color_watch = value.clone();
                                    let message = Message::Color { value };
                                    tx.send(serde_json::to_string(&message).unwrap()).await.unwrap();
                                    handle_message(&message, pool, &tx, &mut stream).await;
                                }
                                Ok(message) => {
                                    handle_message(&message, pool, &tx, &mut stream).await;
                                }
                                Err(_) => {
                                    println!("Failed to parse message: {}", message);
                                }
                            }
                        }
                    }
                    Some(message) = rx.recv() => {
                        if (stream.send(rocket_ws::Message::Text(message)).await).is_err() {
                            break;
                        }
                    }
                    else => break,
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

async fn save_color_to_file(color: &str) -> Result<(), io::Error> {
    let file =
        File::create("color.json").inspect_err(|_| println!("Failed to create message.json"))?;
    serde_json::to_writer(file, color)?;
    Ok(())
}

fn read_color_from_file() -> Result<String, io::Error> {
    let mut file = File::open("color.json")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let color = serde_json::from_str(&content)?;
    Ok(color)
}

extern crate rocket;

use rocket::serde::json::Json;
use crate::database_handling::{get_messages_range, FrontendMessage};

#[get("/messages?<limit>&<offset>")]
async fn load_more_messages(
    pool: &State<DbPool>,
    limit: i64,
    offset: i64,
) -> Json<Vec<FrontendMessage>> {
    let messages = get_messages_range(pool, limit, offset).unwrap();
    Json(messages)
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

    let all_users = Arc::new(Mutex::new(database_handling::get_all_users(&pool).unwrap()));
    let color = Arc::new(Mutex::new(read_color_from_file().unwrap()));
    let channels: Channels = Arc::new(HashMap::new());
    let last_messages: LastMessages = Arc::new(HashMap::new());

    let rocket = rocket::build()
        .manage(pool)
        .manage(color)
        .manage(channels)
        .manage(last_messages)
        .manage(all_users)
        .mount("/", rocket::routes![serve, index, echo_socket, load_more_messages]);

    Ok(rocket.into())
}
