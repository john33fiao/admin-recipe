use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, time::Duration};
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tempfile::TempDir;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{Instant, sleep_until, timeout, timeout_at},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Error, Message, client::IntoClientRequest, http::Request},
};

const INITIAL_MS: u64 = 500;
const GRACE_MS: u64 = 250;
const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(1);
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Real loopback server with the same router, watchdog, and graceful shutdown
/// used by the executable. The accelerated policy is confined to these tests.
struct TestApp {
    state: AppState,
    address: SocketAddr,
    started: Instant,
    server: Option<JoinHandle<std::io::Result<()>>>,
    watchdog: Option<JoinHandle<()>>,
    // Drop after the state/connection and the joined server tasks.
    _directory: TempDir,
}

impl TestApp {
    async fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(directory.path()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let started = Instant::now();
        let state = AppState::new(
            store,
            address.to_string(),
            "test-secret".into(),
            Policy {
                ping_ms: 30,
                stale_ms: 150,
                grace_ms: GRACE_MS,
                initial_ms: INITIAL_MS,
                resume_gap_ms: 10_000,
            },
        );
        let server_state = state.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(server_state.clone()))
                .with_graceful_shutdown(async move { server_state.cancelled().await })
                .await
        });
        let watchdog_state = state.clone();
        let watchdog = tokio::spawn(async move { watchdog_state.watchdog().await });
        Self {
            state,
            address,
            started,
            server: Some(server),
            watchdog: Some(watchdog),
            _directory: directory,
        }
    }

    fn request(&self, origin: &str, token: &str) -> Request<()> {
        let mut request = format!("ws://{}/api/heartbeat", self.address)
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("origin", origin.parse().unwrap());
        request.headers_mut().insert(
            "cookie",
            format!("hansang_{}={token}", self.address.port())
                .parse()
                .unwrap(),
        );
        request
    }

    async fn tab(&self) -> LiveTab {
        let request = self.request(&format!("http://{}", self.address), "test-secret");
        let (socket, response) = timeout(IO_TIMEOUT, connect_async(request))
            .await
            .expect("WebSocket upgrade timed out")
            .expect("valid app session must upgrade");
        assert_eq!(response.status(), 101);
        let mut tab = LiveTab::start(socket);
        // A received protocol Ping proves the server registered the connection.
        tab.next_ping().await;
        tab
    }

    async fn finish(mut self) {
        self.state.shutdown();
        timeout(IO_TIMEOUT, self.state.wait_for_sockets())
            .await
            .expect("server WebSocket tasks did not finish");
        timeout(IO_TIMEOUT, self.server.take().unwrap())
            .await
            .expect("HTTP server did not stop")
            .unwrap()
            .unwrap();
        timeout(IO_TIMEOUT, self.watchdog.take().unwrap())
            .await
            .expect("watchdog did not stop")
            .unwrap();
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // A failed assertion must not leave a listening server behind.
        self.state.shutdown();
        if let Some(server) = &self.server {
            server.abort();
        }
        if let Some(watchdog) = &self.watchdog {
            watchdog.abort();
        }
    }
}

struct LiveTab {
    pings: mpsc::UnboundedReceiver<Instant>,
    close: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<(), Error>>>,
}

impl LiveTab {
    fn start(mut socket: Socket) -> Self {
        let (send_ping, pings) = mpsc::unbounded_channel();
        let (close, mut closing) = oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut closing => {
                        socket.close(None).await?;
                        // Observe the close reply before returning: the server
                        // removes this tab from its lifecycle before replying.
                        while let Some(message) = socket.next().await {
                            match message {
                                Ok(Message::Close(_)) | Err(Error::ConnectionClosed) => break,
                                Err(error) => return Err(error),
                                _ => {},
                            }
                        }
                        return Ok(());
                    }
                    message = socket.next() => match message {
                        Some(Ok(Message::Ping(_))) => {
                            // tungstenite queues protocol Pong on reading Ping;
                            // flushing sends it to the real server immediately.
                            socket.flush().await?;
                            let _ = send_ping.send(Instant::now());
                        }
                        Some(Ok(Message::Close(_))) | None => return Ok(()),
                        Some(Err(Error::ConnectionClosed)) => return Ok(()),
                        Some(Err(error)) => return Err(error),
                        _ => {},
                    }
                }
            }
        });
        Self {
            pings,
            close: Some(close),
            task: Some(task),
        }
    }

    async fn next_ping(&mut self) -> Instant {
        timeout(IO_TIMEOUT, self.pings.recv())
            .await
            .expect("server stopped sending WebSocket Ping")
            .expect("heartbeat socket closed before the expected Ping")
    }

    async fn remain_live_past_grace(&mut self, app: &TestApp) {
        // Observe Pings beyond both the stale deadline and the last-tab grace,
        // including a complete watchdog interval after grace would expire.
        let until = Instant::now() + Duration::from_millis(GRACE_MS + 200);
        loop {
            tokio::select! {
                biased;
                _ = app.state.cancelled() => panic!("server stopped while an authenticated tab answered Pings"),
                received = self.next_ping() => if received >= until { break; },
            }
        }
    }

    async fn close(mut self) {
        self.close.take().unwrap().send(()).unwrap();
        timeout(IO_TIMEOUT, self.task.take().unwrap())
            .await
            .expect("client close handshake timed out")
            .unwrap()
            .expect("client close handshake failed");
    }
}

impl Drop for LiveTab {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_client_shutdown_uses_initial_deadline_on_the_real_server() {
    timeout(TEST_TIMEOUT, async {
        let app = TestApp::start().await;
        timeout(IO_TIMEOUT, app.state.cancelled()).await.unwrap();
        assert!(app.started.elapsed() >= Duration::from_millis(INITIAL_MS));
        app.finish().await;
    })
    .await
    .expect("lifecycle integration test exceeded five seconds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_tab_closing_keeps_the_other_alive_then_last_close_starts_grace() {
    timeout(TEST_TIMEOUT, async {
        let app = TestApp::start().await;
        let first = app.tab().await;
        let mut second = app.tab().await;
        first.close().await;
        second.remain_live_past_grace(&app).await;
        let closing_at = Instant::now();
        second.close().await;
        timeout(IO_TIMEOUT, app.state.cancelled()).await.unwrap();
        assert!(closing_at.elapsed() >= Duration::from_millis(GRACE_MS));
        app.finish().await;
    })
    .await
    .expect("lifecycle integration test exceeded five seconds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_reconnect_cancels_the_pending_shutdown() {
    timeout(TEST_TIMEOUT, async {
        let app = TestApp::start().await;
        app.tab().await.close().await;
        timeout(IO_TIMEOUT, app.state.wait_for_sockets())
            .await
            .unwrap();
        let mut refreshed = app.tab().await;
        refreshed.remain_live_past_grace(&app).await;
        refreshed.close().await;
        timeout(IO_TIMEOUT, app.state.cancelled()).await.unwrap();
        app.finish().await;
    })
    .await
    .expect("lifecycle integration test exceeded five seconds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_origin_and_cookie_do_not_extend_the_initial_lifetime() {
    timeout(TEST_TIMEOUT, async {
        let app = TestApp::start().await;
        // Continue rejected upgrades across the initial deadline. If either
        // resets initial/grace timing, this traffic postpones shutdown beyond
        // the bounded wait. Cancellation wins over a shutdown-time HTTP 503.
        let mut rejected = 0;
        let probes = async {
            let mut next_attempt = app.started + Duration::from_millis(350);
            loop {
                sleep_until(next_attempt).await;
                for (origin, token, expected_status) in [
                    ("https://untrusted.invalid".to_owned(), "test-secret", 403),
                    (format!("http://{}", app.address), "wrong-secret", 401),
                ] {
                    let result = timeout(IO_TIMEOUT, connect_async(app.request(&origin, token)))
                        .await
                        .expect("rejected WebSocket handshake timed out");
                    match result {
                        Err(Error::Http(response)) => assert_eq!(response.status(), expected_status),
                        other => panic!("expected rejected upgrade, got {other:?}"),
                    }
                    rejected += 1;
                }
                next_attempt += Duration::from_millis(50);
            }
        };
        tokio::select! {
            biased;
            result = timeout_at(app.started + Duration::from_millis(750), app.state.cancelled()) => {
                result.expect("rejected clients extended the initial connection deadline");
            }
            _ = probes => unreachable!("upgrade probes run until server cancellation"),
        }
        assert!(rejected >= 2, "both invalid sessions must be rejected before shutdown");
        assert!(app.started.elapsed() >= Duration::from_millis(INITIAL_MS));
        app.finish().await;
    })
    .await
    .expect("lifecycle integration test exceeded five seconds");
}
