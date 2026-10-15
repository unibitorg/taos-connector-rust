#[cfg(feature = "rustls")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use taos_ws::ClientConfig;
use tokio::net::TcpListener;
use ws_tool::{codec::AsyncDeflateCodec, stream::AsyncStream};

#[tokio::test]
async fn default_nodelay_on_initial_and_new_connections() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = async {
            for _ in 0..2 {
                let (stream, _) = listener.accept().await?;
                let _websocket = tokio_tungstenite::accept_async(stream).await?;
            }
            Ok::<_, anyhow::Error>(())
        };
        let client = async {
            for _ in 0..2 {
                let mut config = ClientConfig::default();
                let mut websocket = config
                    .async_connect_with(format!("ws://{address}/ws"), AsyncDeflateCodec::check_fn)
                    .await?;
                match websocket.stream_mut().get_ref() {
                    AsyncStream::Raw(stream) => {
                        assert!(
                            stream.nodelay()?,
                            "TD TCP connections must enable TCP_NODELAY"
                        );
                    }
                    _ => anyhow::bail!("loopback ws unexpectedly used TLS"),
                }
            }
            Ok::<_, anyhow::Error>(())
        };
        tokio::try_join!(server, client)?;
        Ok::<_, anyhow::Error>(())
    })
    .await?
}

#[cfg(feature = "rustls")]
#[tokio::test]
async fn default_nodelay_is_set_before_tls_handshake() -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let observed = Arc::new(AtomicBool::new(false));
        let configured = Arc::clone(&observed);
        let mut config = ClientConfig::default();
        let mut configure_socket = config.set_socket_fn;
        config.set_socket_fn = Box::new(move |stream| {
            configure_socket(stream)?;
            assert!(stream.nodelay()?, "TCP_NODELAY must be enabled before TLS");
            configured.store(true, Ordering::SeqCst);
            Ok(())
        });
        let server = async {
            let (stream, _) = listener.accept().await?;
            drop(stream); // Deliberately stop before TLS; no test certificate or trust changes.
            Ok::<_, anyhow::Error>(())
        };
        let client = async {
            let result = config
                .async_connect_with(format!("wss://{address}/ws"), AsyncDeflateCodec::check_fn)
                .await;
            assert!(
                result.is_err(),
                "closed peer must not complete a TLS handshake"
            );
            assert!(observed.load(Ordering::SeqCst));
            Ok::<_, anyhow::Error>(())
        };
        tokio::try_join!(server, client)?;
        Ok::<_, anyhow::Error>(())
    })
    .await?
}
