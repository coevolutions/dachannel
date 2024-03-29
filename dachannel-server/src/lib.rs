use datachannel_facade::platform::native::ConfigurationExt as _;
use futures::SinkExt as _;
use http_body_util::BodyExt as _;
use tokio::io::AsyncWriteExt;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("dachannel: {0}")]
    Dachannel(#[from] dachannel::Error),

    #[error("axum: {0}")]
    Axum(#[from] axum::Error),

    #[error("malformed body")]
    MalformedBody,

    #[error("closed")]
    Closed,
}

/// A Future that is an in-progress connection attempt from a remote client.
///
/// This Future may be awaited on to complete the connection, or dropped to abort it.
pub struct Connecting {
    parts: axum::http::request::Parts,
    remote_addr: std::net::SocketAddr,
    connection_builder: dachannel::ConnectionBuilder,
    body: axum::body::Body,
    answer_sdp_tx: Option<tokio::io::DuplexStream>,
}

impl Connecting {
    /// The new connection. Any DataChannels can be configured here before completing the future.
    pub fn create_data_channel(
        &self,
        label: &str,
        options: dachannel::DataChannelOptions,
    ) -> Result<dachannel::Channel, dachannel::Error> {
        self.connection_builder.create_data_channel(label, options)
    }

    /// The HTTP Authorization header, if any.
    pub fn header(&self, key: impl axum::http::header::AsHeaderName) -> Option<&str> {
        self.parts
            .headers
            .get(key)
            .map(|v| v.to_str().ok())
            .flatten()
    }

    /// The remote address connecting to the HTTP server. This may or may not be the remote address of the DataChannel.
    pub fn remote_addr(&self) -> &std::net::SocketAddr {
        &self.remote_addr
    }
}

impl std::future::IntoFuture for Connecting {
    type Output = Result<dachannel::Connection, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move {
            let offer_sdp = String::from_utf8(self.body.collect().await?.to_bytes().to_vec())
                .map_err(|_| Error::MalformedBody)?;

            let conn = self.connection_builder.build();

            conn.set_remote_description(&dachannel::Description {
                type_: dachannel::SdpType::Offer,
                sdp: offer_sdp,
            })
            .await?;
            conn.set_local_description(dachannel::SdpType::Answer)
                .await?;
            conn.ice_candidates_gathered().await;

            let answer_sdp = conn
                .local_description()?
                .map(|v| v.sdp)
                .unwrap_or_else(|| "".to_string());

            self.answer_sdp_tx
                .take()
                .unwrap()
                .write_all(answer_sdp.as_bytes())
                .await
                .map_err(|_| Error::Closed)?;

            Ok(conn)
        })
    }
}

async fn offer(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    axum::extract::ConnectInfo(remote_addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    req: axum::extract::Request,
) -> Result<impl axum::response::IntoResponse, axum::http::StatusCode> {
    let (parts, body) = req.into_parts();

    let mut config: dachannel::Configuration = Default::default();
    config.ice_servers = state.ice_servers.clone();
    config.set_bind(
        state.bind_addr.ip(),
        state.bind_addr.port(),
        state.bind_addr.port(),
    );
    config.set_enable_ice_udp_mux(true);

    let connection_builder = dachannel::Connection::builder(config).map_err(|e| {
        log::error!("failed to create connection: {e}");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (answer_sdp_tx, answer_sdp_rx) = tokio::io::duplex(4096);
    state
        .connecting_tx
        .lock()
        .await
        .send(Connecting {
            parts,
            remote_addr,
            connection_builder,
            body,
            answer_sdp_tx: Some(answer_sdp_tx),
        })
        .await
        .map_err(|_e| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(axum::body::Body::from_stream(
        tokio_util::io::ReaderStream::new(answer_sdp_rx),
    ))
}

struct AppState {
    bind_addr: std::net::SocketAddr,
    ice_servers: Vec<dachannel::IceServer>,
    connecting_tx: tokio::sync::Mutex<futures::channel::mpsc::Sender<Connecting>>,
}

pub struct ServeOptions {
    ice_servers: Vec<dachannel::IceServer>,
    backlog: usize,
}

impl ServeOptions {
    pub fn new() -> Self {
        Self {
            ice_servers: vec![],
            backlog: 128,
        }
    }

    pub fn ice_servers(mut self, ice_servers: Vec<dachannel::IceServer>) -> Self {
        self.ice_servers = ice_servers;
        self
    }

    pub fn backlog(mut self, backlog: usize) -> Self {
        self.backlog = backlog;
        self
    }

    pub fn serve(
        self,
        listener: tokio::net::TcpListener,
    ) -> (
        impl std::future::Future<Output = Result<(), std::io::Error>>,
        futures::channel::mpsc::Receiver<Connecting>,
    ) {
        let (connecting_tx, connecting_rx) = futures::channel::mpsc::channel(self.backlog);
        (
            (move || async move {
                let bind_addr = listener.local_addr()?;
                axum::serve(
                    listener,
                    axum::Router::new()
                        .route("/", axum::routing::post(offer))
                        .with_state(std::sync::Arc::new(AppState {
                            bind_addr,
                            ice_servers: self.ice_servers,
                            connecting_tx: tokio::sync::Mutex::new(connecting_tx),
                        }))
                        .layer(
                            tower_http::cors::CorsLayer::new()
                                .allow_headers([
                                    axum::http::header::AUTHORIZATION,
                                    "*".try_into().unwrap(),
                                ])
                                .allow_methods([axum::http::Method::POST])
                                .allow_origin(tower_http::cors::Any),
                        )
                        .layer(tower_http::limit::RequestBodyLimitLayer::new(4096))
                        .into_make_service_with_connect_info::<std::net::SocketAddr>(),
                )
                .await
            })(),
            connecting_rx,
        )
    }
}
