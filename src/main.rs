use std::net::SocketAddr;

use axum::{Router, body::Bytes, response::IntoResponse, routing::get};
use chrono::Duration;
use icalendar::{Calendar, Component, Event, EventLike as _};
use tokio::net::TcpListener;
use tracing::debug;

mod bot;
mod scramble;

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::mpsc::channel(20);

    let app = Router::new()
        .route("/calendar/{id}", get(calendar))
        .route("/", get(index))
        .route("/icon-white.png", get(logo))
        .with_state(tx);

    tokio::spawn(bot::start(rx));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    debug!("Listening on {}", addr);
    axum::serve(
        TcpListener::bind(&addr).await.expect("bind to :3000"),
        app.into_make_service(),
    )
    .await
    .expect("should start server");
}

async fn calendar(
    axum::extract::Path(encoded_id): axum::extract::Path<String>,
    axum::extract::State(tx): axum::extract::State<tokio::sync::mpsc::Sender<bot::BotRequest>>,
) -> impl IntoResponse {
    let id = match scramble::decode(&encoded_id) {
        Some(id) => id,
        None => {
            return axum::response::Response::builder()
                .status(400)
                .body("Invalid calendar ID".to_string())
                .expect("should build response");
        }
    };

    let mut calendar = Calendar::new();

    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel(1);

    tx.send(bot::BotRequest::Calendar(id, cmd_tx))
        .await
        .expect("should send calendar request");

    let Ok((guild_name, events)) = cmd_rx
        .recv()
        .await
        .expect("should receive calendar response")
    else {
        // return 404
        return axum::response::Response::builder()
            .status(404)
            .body("Guild not found".to_string())
            .expect("should build response");
    };

    for event in events {
        calendar.push({
            let mut ev = Event::new();
            ev.uid(&event.id.to_string());
            ev.summary(&event.name).starts(event.start_time.to_utc());
            if let Some(end_time) = event.end_time {
                ev.ends(end_time.to_utc());
            }
            if let Some(description) = &event.description {
                ev.description(description);
            }
            if let Some(metadata) = &event.metadata
                && let Some(location) = &metadata.location
            {
                ev.location(location);
            }
            ev.done()
        });
    }
    let body = calendar
        .name(&guild_name)
        .ttl(&Duration::try_minutes(30).expect("duration valid"))
        .to_string();
    axum::response::Response::builder()
        .status(200)
        // .header("Content-Type", "text/calendar")
        .body(body)
        .expect("should build response")
}

async fn index() -> impl IntoResponse {
    axum::response::Response::builder()
        .status(200)
        .body(include_str!("../index.html").to_string())
        .expect("should build response")
}

async fn logo() -> Bytes {
    Bytes::from_static(include_bytes!("../icon-white.png"))
}
