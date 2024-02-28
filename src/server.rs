use std::future::IntoFuture;

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

    #[error("closed")]
    Closed,
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

    pub fn authorization(&self) -> Option<&str> {
        self.parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .map(|v| v.to_str().ok())
            .flatten()
    }

    pub fn remote_addr(&self) -> &std::net::SocketAddr {
        &self.remote_addr
    }
}

impl std::future::IntoFuture for Connecting {
    type Output = Result<crate::Connection, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async {
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
                .map(|v| v.sdp)
                .unwrap_or_else(|| "".to_string());

            self.answer_sdp_tx
                .send(answer_sdp)
                .map_err(|_| Error::Closed)?;

            Ok(self.connection)
        })
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
                    .layer(tower_http::timeout::TimeoutLayer::new(
                        std::time::Duration::from_secs(30),
                    ))
                    .into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .into_future()
            .await
        })(),
        connecting_rx,
    )
}
