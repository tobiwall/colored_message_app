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
use std::sync::Arc;
use tokio::select;

mod msac;

type LastMessages = Arc<HashMap<String, String>>;
type Channels = Arc<HashMap<String, msac::Channel>>;

// #[get("/<path..>")]
// pub async fn serve(path: PathBuf) -> Option<NamedFile> {
//     let mut path = Path::new(relative!("assets")).join(path);
//     if path.is_dir() {
//         path.push("index.html");
//     }

//     NamedFile::open(path).await.ok()
// }

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
) -> ws::Channel<'r> {
    let (tx, mut rx) = {
        // let mut channels = channels.write().await;
        let channel = channels
            .entry(room.clone())
            .or_insert_with(|| msac::Channel::new());
        channel.add().await
    };
    ws.channel(move |mut stream| {
        Box::pin(async move {
            {
                // let mut last_messages = last_messages.write().await;
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
                                last_messages.insert(room.clone(), message.clone());
                            }
                            // stream.send(rocket_ws::Message::Text(message.to_string())).await.unwrap();
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

#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let rocket = rocket::build().mount("/", rocket::routes![serve, index, echo_socket]);

    // manage a mpmc channel for strings
    // let channel = msac::Channel::new();
    let channels: Channels = Arc::new(HashMap::new());
    let rocket = rocket.manage(channels);
    // manage a sting for the last sent message
    // let last_message = Arc::new(RwLock::new(String::new()));
    let last_messages: LastMessages = Arc::new(HashMap::new());
    let rocket = rocket.manage(last_messages);

    Ok(rocket.into())
}
