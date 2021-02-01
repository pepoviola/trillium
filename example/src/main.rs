use askama::Template;
use futures_lite::prelude::*;
use myco::{Conn, Sequence};
use myco_askama::AskamaConnExt;
use myco_cookies::Cookies;
use myco_logger::DevLogger;
use myco_router::{Router, RouterConnExt};
use myco_sessions::{MemoryStore, SessionConnExt, Sessions};
use myco_static::Static;
use myco_websockets::{Message, WebSocket, WebSocketConnection};

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

fn main() {
    env_logger::init();

    myco_smol_server::run(
        "localhost:8080",
        Sequence::new()
            .and(DevLogger)
            .and(Cookies)
            .and(Sessions::new(
                MemoryStore::new(),
                b"01234567890123456789012345678901123",
            ))
            .and(|conn: Conn| async move {
                let count = conn.session().get::<usize>("count").unwrap_or_default();
                conn.send_header("request-count", count.to_string())
                    .with_session("count", count + 1)
            })
            .and(
                Router::new()
                    .get("/always-hi", "hi")
                    .post("/", |mut conn: Conn| async move {
                        let body = conn.request_body().await.read_string().await.unwrap();
                        conn.ok(format!("request body: {}", body))
                    })
                    .get("/template/:name", |conn: Conn| async move {
                        if let Some(name) = conn.param("name").map(String::from) {
                            conn.render(HelloTemplate { name: &name })
                        } else {
                            conn
                        }
                    })
                    .get(
                        "/ws",
                        WebSocket::new(|mut wsc: WebSocketConnection| async move {
                            while let Some(Ok(Message::Text(input))) = wsc.next().await {
                                let output: String = input.chars().rev().collect();
                                wsc.send_string(format!("{} | {}", &input, &output)).await;
                            }
                        }),
                    )
                    .get("/", |conn: Conn| async move { conn.ok("hello world") })
                    .get("/hello/:planet", |conn: Conn| async move {
                        if let Some(planet) = conn.param("planet") {
                            let response = format!("hello, {}", planet);
                            conn.ok(response)
                        } else {
                            conn
                        }
                    })
                    .get("/static/*", Static::new("/static/", ".")),
            ),
    );
}
