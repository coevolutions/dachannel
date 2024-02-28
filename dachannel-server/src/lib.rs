use datachannel_facade::platform::native::ConfigurationExt as _;
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
    pub fn connection_builder(&self) -> &dachannel::ConnectionBuilder {
        &self.connection_builder
    }

    /// The HTTP Authorization header, if any.
    pub fn authorization(&self) -> Option<&str> {
        self.parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
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
    config.set_bind(
        state.bind_addr.ip(),
        state.bind_addr.port(),
        state.bind_addr.port(),
    );

    let connection_builder = dachannel::Connection::builder(config).map_err(|e| {
        log::error!("failed to create connection: {e}");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (answer_sdp_tx, answer_sdp_rx) = tokio::io::duplex(4096);
    state
        .connecting_tx
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
    connecting_tx: async_channel::Sender<Connecting>,
}

/// Start the server on a listener.
pub async fn serve(
    listener: tokio::net::TcpListener,
    backlog: usize,
) -> (
    impl std::future::Future<Output = Result<(), std::io::Error>>,
    async_channel::Receiver<Connecting>,
) {
    let (connecting_tx, connecting_rx) = async_channel::bounded(backlog);
    (
        (move || async move {
            let bind_addr = listener.local_addr()?;
            axum::serve(
                listener,
                axum::Router::new()
                    .route("/", axum::routing::post(offer))
                    .with_state(std::sync::Arc::new(AppState {
                        bind_addr,
                        connecting_tx: connecting_tx.clone(),
                    }))
                    .layer(
                        tower_http::cors::CorsLayer::new()
                            .allow_headers([axum::http::header::AUTHORIZATION])
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
