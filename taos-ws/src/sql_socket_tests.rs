use std::cell::RefCell;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::io::BufStream;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{handshake::server::Request, Message};
use ws_tool::stream::AsyncStream;

use crate::{query::WsTaos, TaosBuilder};

tokio::task_local! {
    static SOCKET_OPTIONS: RefCell<Vec<bool>>;
}

// Observe the actual client socket at the production WS handshake boundary.
// No configuration is replaced and no socket option is changed here.
pub(super) fn observe_socket(stream: &BufStream<AsyncStream>) {
    let _ = SOCKET_OPTIONS.try_with(|options| {
        let AsyncStream::Raw(socket) = stream.get_ref() else {
            panic!("local WS test must observe the raw client TCP socket");
        };
        options.borrow_mut().push(socket.nodelay().unwrap());
    });
}

async fn sql_connection_nodelay(fallback: bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let builder =
            TaosBuilder::from_dsn(format!("ws://{}", listener.local_addr().unwrap())).unwrap();
        let server = async {
            if fallback {
                let (socket, _) = listener.accept().await.unwrap();
                let result =
                    tokio_tungstenite::accept_hdr_async(socket, |request: &Request, _response| {
                        assert_eq!(request.uri().path(), "/ws");
                        Err(http::Response::builder()
                            .status(404)
                            .body(Some("Not Found".to_owned()))
                            .unwrap())
                    })
                    .await;
                assert!(result.is_err(), "first endpoint must reject the upgrade");
            }
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws =
                tokio_tungstenite::accept_hdr_async(socket, |request: &Request, response| {
                    assert_eq!(
                        request.uri().path(),
                        if fallback { "/rest/ws" } else { "/ws" }
                    );
                    Ok(response)
                })
                .await
                .unwrap();
            for (action, reply) in [
                (
                    "version",
                    r#"{"code":0,"message":"","action":"version","version":"3.3.5.8"}"#,
                ),
                ("conn", r#"{"code":0,"message":"","action":"conn"}"#),
            ] {
                let request = ws.next().await.unwrap().unwrap();
                let request: serde_json::Value =
                    serde_json::from_str(request.to_text().unwrap()).unwrap();
                assert_eq!(request["action"], action);
                ws.send(Message::Text(reply.to_owned())).await.unwrap();
            }
        };
        let client = SOCKET_OPTIONS.scope(RefCell::new(Vec::new()), async {
            let client = WsTaos::from_wsinfo(&builder).await.unwrap();
            let options = SOCKET_OPTIONS.with(|options| options.borrow().clone());
            drop(client);
            options
        });
        let ((), options) = tokio::join!(server, client);
        println!("real SQL WS handshake: fallback={fallback}, client TCP_NODELAY={options:?}");
        assert_eq!(
            options,
            vec![true; if fallback { 2 } else { 1 }],
            "SQL client TCP_NODELAY must be enabled on every connection"
        );
    })
    .await
    .expect("local SQL WS handshake must complete");
}

#[tokio::test]
async fn sql_primary_connection_enables_tcp_nodelay() {
    sql_connection_nodelay(false).await;
}

#[tokio::test]
async fn sql_fallback_connection_enables_tcp_nodelay() {
    sql_connection_nodelay(true).await;
}
