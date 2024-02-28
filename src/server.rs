use std::future::IntoFuture;

use axum::body::HttpBody as _;
use datachannel_facade::platform::native::ConfigurationExt as _;
use http_body_util::BodyExt as _;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("dachannel: {0}")]
    Dachannel(#[from] crate::Error),

    #[error("axum: {0}")]
    Axum(#[from] axum::Error),

    #[error("malformed body")]
    MalformedBody,

    #[error("missing local description")]
    MissingLocalDescription,
}

pub struct Connecting {
    parts: axum::http::request::Parts,
    remote_addr: std::net::SocketAddr,
    connection: crate::Connection,
    body: axum::body::Body,
    answer_sdp_tx: tokio::sync::oneshot::Sender<String>,
}

impl Connecting {
    pub fn connection(&self) -> &crate::Connection {
        &self.connection
    }

    pub fn headers(&self) -> &axum::http::HeaderMap {
        &self.parts.headers
    }

    pub fn remote_addr(&self) -> &std::net::SocketAddr {
        &self.remote_addr
    }

    pub async fn finish(self) -> Result<crate::Connection, Error> {
        let offer_sdp = String::from_utf8(self.body.collect().await?.to_bytes().to_vec())
            .map_err(|_| Error::MalformedBody)?;

        self.connection
            .set_remote_description(&crate::Description {
                type_: crate::SdpType::Offer,
                sdp: offer_sdp,
            })
            .await?;
        self.connection
            .set_local_description(crate::SdpType::Answer)
            .await?;
        self.connection.ice_candidates_gathered().await;

        let answer_sdp = self
            .connection
            .local_description()?
            .ok_or(Error::MissingLocalDescription)?
            .sdp;
        let _ = self.answer_sdp_tx.send(answer_sdp);

        Ok(self.connection)
    }
}

async fn offer(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    axum::extract::ConnectInfo(remote_addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    req: axum::extract::Request,
) -> Result<String, axum::http::StatusCode> {
    let (parts, body) = req.into_parts();

    let mut config: crate::Configuration = Default::default();
    config.set_bind(
        state.bind_addr.ip(),
        state.bind_addr.port(),
        state.bind_addr.port(),
    );

    let connection = crate::Connection::new(config).map_err(|e| {
        log::error!("failed to create connection: {e}");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if body.size_hint().upper().unwrap_or(u64::MAX) > 8 * 1024 {
        return Err(axum::http::StatusCode::PAYLOAD_TOO_LARGE);
    }

    let (answer_sdp_tx, answer_sdp_rx) = tokio::sync::oneshot::channel();
    state
        .connecting_tx
        .send(Connecting {
            parts,
            remote_addr,
            connection,
            body,
            answer_sdp_tx,
        })
        .await
        .map_err(|_e| axum::http::StatusCode::SERVICE_UNAVAILABLE)?;

    let answer_sdp = answer_sdp_rx
        .await
        .map_err(|_e| axum::http::StatusCode::FORBIDDEN)?;

    Ok(answer_sdp)
}

struct AppState {
    bind_addr: std::net::SocketAddr,
    connecting_tx: async_channel::Sender<Connecting>,
}

pub async fn listen(
    listener: tokio::net::TcpListener,
    backlog: usize,
) -> Result<
    (
        impl std::future::Future<Output = Result<(), std::io::Error>>,
        async_channel::Receiver<Connecting>,
    ),
    std::io::Error,
> {
    let (connecting_tx, connecting_rx) = async_channel::bounded(backlog);
    let bind_addr = listener.local_addr()?;

    Ok((
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
                .into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .into_future(),
        connecting_rx,
    ))
}
