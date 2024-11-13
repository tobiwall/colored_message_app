use dashmap::DashMap as HashMap;
// use rocket::fs::relative;
use rocket::fs::NamedFile;
use rocket::futures::SinkExt;
use rocket::futures::StreamExt;
use rocket::info;
use rocket::response::content;
use rocket::*;
use rocket_ws as ws;
// use std::path::{Path, PathBuf};
use ::serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io;
use std::io::Error;
use std::io::Read;
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;

mod msac;

type LastMessages = Arc<HashMap<String, String>>;
type Channels = Arc<HashMap<String, msac::Channel>>;

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
    // let last_message = last_message.read().await;
    file = file.replace("StartValue", &last_message);

    Some(content::RawHtml(file))
}

#[get("/echo/<room>")]
async fn echo_socket<'r>(
    ws: ws::WebSocket,
    room: String,
    channels: &'r State<Channels>,
    last_messages: &'r State<LastMessages>,
    all_messages: &'r State<Arc<Mutex<Vec<SingleMessage>>>>,
) -> ws::Channel<'r> {
    let (tx, mut rx) = {
        // let mut channels = channels.write().await;
        let channel = channels
            .entry(room.clone())
            .or_insert_with(|| msac::Channel::new());
        channel.add().await
    };

    let messages = all_messages.lock().await.clone();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            for msg in &messages {
                if let Err(_) = stream
                    .send(rocket_ws::Message::Text(
                        serde_json::to_string(msg).unwrap(),
                    ))
                    .await
                {
                    break;
                }
            }

            {
                let last_message = last_messages
                    .entry(room.clone())
                    .or_insert_with(|| "180".to_string());
                if !last_message.is_empty() {
                    info!("sending last message: {}", last_message.value());
                    if let Err(_) = stream
                        .send(rocket_ws::Message::Text(last_message.clone()))
                        .await
                    {
                        return Ok(());
                    }
                }
            }

            loop {
                select! {
                    message = stream.next() => {
                        if let Some(message) = message {
                            let message = message.unwrap();

                            // check, if the message is a string
                            if let rocket_ws::Message::Text(message) = message {
                                tx.send(message.to_string()).await.unwrap();

                                match serde_json::from_str::<SingleMessage>(&message) {
                                    Ok(new_message) => {
                                        let mut messages = all_messages.lock().await;
                                    messages.push(new_message.clone());
                                    save_message_to_file(&messages).unwrap();
                                },
                                Err(_) => {
                                    match serde_json::from_str::<i32>(&message) {
                                        Ok(number) => {
                                            // Handle the number (e.g., store it or use it in some way)
                                            println!("Received a number: {}", number);
                                            save_color(number.to_string()).unwrap();
                                        },
                                        // If it's neither a `SingleMessage` nor an integer, log the error
                                        Err(e) => {
                                            println!("Failed to parse message: {}", e);
                                            continue;
                                        }
                                    }
                                }
                                };
                            }
                        } else {
                            break;
                        }
                    },
                    message = rx.recv() => {
                        if let Some(message) = message {
                            if let Err(_) = stream.send(rocket_ws::Message::Text(message)).await {
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

// struct User {
//     name: String,
//     password: String,
// }

#[derive(Clone, Serialize, Deserialize, Debug)]
struct SingleMessage {
    user: String,
    message: String,
}

fn save_message_to_file(messages: &Vec<SingleMessage>) -> Result<(), Error> {
    let file = match File::create("messages.json") {
        Ok(f) => {
            println!("Added new file");
            f
        }
        Err(e) => {
            println!("Faild to create a messages.json");
            return Err(e);
        }
    };

    if let Err(e) = serde_json::to_writer(file, &messages) {
        println!("Failed to write the message into file");
        return Err(e.into());
    };
    Ok(())
}

fn save_color(color: String) -> Result<(), Error> {
    let file = match File::create("color.json") {
        Ok(f) => f,
        Err(e) => {
            println!("Faild to create a messages.json");
            return Err(e);
        }
    };

    if let Err(e) = serde_json::to_writer(file, &color) {
        return Err(e.into());
    }

    Ok(())
}

fn read_messages_from_file() -> Result<Vec<SingleMessage>, io::Error> {
    let mut file = File::open("messages.json")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let messages = serde_json::from_str(&content)?;
    println!("This is the content{content}");
    Ok(messages)
}

// fn read_color_from_file() -> Result<Vec<SingleMessage>, io::Error> {
//     let mut file = File::open("color.json")?;
//     let mut content = String::new();
//     file.read_to_string(&mut content)?;
//     let messages = serde_json::from_str(&content)?;
//     println!("This is the content{content}");
//     Ok(messages)
// }

#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let all_messages = Arc::new(Mutex::new(read_messages_from_file().unwrap()));
    let rocket = rocket::build()
        .manage(all_messages)
        .mount("/", rocket::routes![serve, index, echo_socket]);

    // manage a mpmc channel for strings
    let channels: Channels = Arc::new(HashMap::new());
    let rocket = rocket.manage(channels);
    // manage a sting for the last sent message
    let last_messages: LastMessages = Arc::new(HashMap::new());
    let rocket = rocket.manage(last_messages);

    Ok(rocket.into())
}
