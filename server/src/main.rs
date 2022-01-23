use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use shared::{get_channel_id, Chatroom};
use std::collections::HashMap;
use std::error::Error;
use tokio::runtime::Runtime;
use url::Url;

const URLS: [&str; 1] = ["http://0.0.0.0:3000"];

#[derive(Deserialize)]
struct ChatroomQuery {
    search: String,
}

async fn get_chatrooms(Query(query): Query<ChatroomQuery>) -> Json<Vec<Chatroom>> {
    let terms: Vec<&str> = query.search.split(" ").collect();
    let instances = locate_instances(&URLS, &terms);

    let client = reqwest::Client::new();

    let mut chatrooms = Vec::new();

    for (url, terms) in instances {
        let response = client
            .post(url.join("chatrooms").unwrap())
            .json(&terms)
            .send()
            .await
            .unwrap()
            .json::<HashMap<String, (u32, u32)>>()
            .await
            .unwrap();
        for (term, (chatroom_id, count)) in response {
            let mut url = url.join("ws").unwrap();
            url.set_scheme("ws").unwrap();

            chatrooms.push(Chatroom {
                term,
                chatroom_id,
                num_users: count,
                url: url.join("ws").unwrap().to_string(),
            });
        }
    }

    Json(chatrooms)
}

fn locate_instances<'a>(urls: &'a [&'a str], terms: &'a [&'a str]) -> Vec<(Url, Vec<&'a str>)> {
    let mut instances = Vec::new();
    for url in urls {
        instances.push((Url::parse(url).unwrap(), Vec::new()));
    }

    for term in terms {
        let hash = get_channel_id(*term);
        let index = hash as usize % urls.len();
        instances[index].1.push(*term);
    }

    instances
}

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = Runtime::new()?;

    let app = Router::new().route("/chatrooms", get(get_chatrooms));

    runtime.block_on(async {
        axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::locate_instances;
    use url::Url;

    #[test]
    fn locate_instances_returns_instances() {
        let urls = [
            "http://instance1.example.com",
            "http://instance2.example.com",
        ];
        let terms = ["a", "b", "c", "d", "e", "f", "g", "h"];
        let instances = locate_instances(&urls, &terms);
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].0, Url::parse(urls[0]).unwrap());
        assert_eq!(instances[0].1.len(), 5);
        assert_eq!(instances[1].0, Url::parse(urls[1]).unwrap());
        assert_eq!(instances[1].1.len(), 3);
    }
}
