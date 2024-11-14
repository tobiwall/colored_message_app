use dashmap::DashMap as HashMap;
// use rocket::fs::relative;
use rocket::fs::NamedFile;
use rocket::futures::SinkExt;
use rocket::futures::StreamExt;
use rocket::response::content;
use rocket::*;
use rocket_ws as ws;
// use std::path::{Path, PathBuf};
use ::serde::{Deserialize, Serialize};
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
    all_messages: &'r State<Arc<Mutex<Vec<Message>>>>,
    color: &'r State<Arc<Mutex<String>>>,
) -> ws::Channel<'r> {
    let (tx, mut rx) = {
        let channel = channels
            .entry(room.clone())
            .or_insert_with(msac::Channel::new);
        channel.add().await
    };

    let messages = all_messages.lock().await.clone();
    let current_color = color.lock().await.clone();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            for msg in &messages {
                if (stream
                    .send(rocket_ws::Message::Text(
                        serde_json::to_string(msg).unwrap(),
                    ))
                    .await).is_err()
                {
                    break;
                }
            }

            {
                if (stream
                    .send(rocket_ws::Message::Text(current_color.clone()))
                    .await).is_err()
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
                                if let Ok(new_message) = serde_json::from_str::<Message>(&message) {
                                    tx.send(message.to_string()).await.unwrap();
                                    let mut messages = all_messages.lock().await;
                                    messages.push(new_message.clone());
                                    save_message_to_file(&messages).unwrap();
                                } else if let Ok(new_color) = message.parse::<String>() {
                                    tx.send(new_color.clone()).await.unwrap();
                                    let mut color_watch = color.lock().await;
                                    *color_watch = new_color.clone();
                                    save_color(new_color).unwrap();
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

// struct User {
//     name: String,
//     password: String,
// }

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Message {
    user: String,
    message: String,
}

fn save_message_to_file(messages: &Vec<Message>) -> Result<(), Error> {
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
    let file =
        File::create("color.json").inspect_err(|_| println!("Failed to create message.json"))?;

    // if let Err(e) = serde_json::to_writer(file, &color) {
    //     return Err(e.into());
    // }
    serde_json::to_writer(file, &color)?;

    Ok(())
}

fn read_messages_from_file() -> Result<Vec<Message>, io::Error> {
    let mut file = File::open("messages.json")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let messages = serde_json::from_str(&content)?;
    println!("This is the content{content}");
    Ok(messages)
}

fn read_color_from_file() -> Result<String, io::Error> {
    let mut file = File::open("color.json")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let color = serde_json::from_str(&content)?;
    println!("This is the color number{content}");
    Ok(color)
}

#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let all_messages = Arc::new(Mutex::new(read_messages_from_file().unwrap()));
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
