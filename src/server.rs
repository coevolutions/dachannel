use datachannel_facade::platform::native::ConfigurationExt as _;
use http_body_util::BodyExt as _;
use hyper::body::Body as _;

pub struct Server {
    connecting_rx: async_channel::Receiver<Connecting>,
}

struct ConnectingInner {
    connection: crate::Connection,
    body: axum::body::Body,
    answer_sdp_tx: tokio::sync::oneshot::Sender<Option<String>>,
}

pub struct Connecting {
    headers: hyper::HeaderMap,
    inner: Option<ConnectingInner>,
}

impl Connecting {
    pub fn connection(&self) -> &crate::Connection {
        &self.inner.as_ref().unwrap().connection
    }

    pub fn headers(&self) -> &hyper::HeaderMap {
        &self.headers
    }

    pub async fn finish(
        mut self,
    ) -> Result<crate::Connection, Box<dyn std::error::Error + Send + Sync>> {
        let inner = self.inner.take().unwrap();

        let offer_sdp = String::from_utf8(inner.body.collect().await?.to_bytes().to_vec())?;

        inner
            .connection
            .set_remote_description(&crate::Description {
                type_: crate::SdpType::Offer,
                sdp: offer_sdp,
            })
            .await?;
        inner
            .connection
            .set_local_description(crate::SdpType::Answer)
            .await?;
        inner.connection.ice_candidates_gathered().await;

        // TODO: Remove this unwrap?
        let answer_sdp = inner.connection.local_description()?.unwrap().sdp;
        let _ = inner.answer_sdp_tx.send(Some(answer_sdp));

        Ok(inner.connection)
    }
}

impl Drop for Connecting {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            let _ = inner.answer_sdp_tx.send(None);
        }
    }
}

async fn offer(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    req: axum::extract::Request,
) -> Result<String, axum::http::StatusCode> {
    let (parts, body) = req.into_parts();

    let mut config: crate::Configuration = Default::default();
    config.set_bind(
        state.bind_addr.ip(),
        state.bind_addr.port(),
        state.bind_addr.port(),
    );

    let conn = crate::Connection::new(config)
        .map_err(|_e| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    if body.size_hint().upper().unwrap_or(u64::MAX) > 8 * 1024 {
        return Err(hyper::StatusCode::PAYLOAD_TOO_LARGE);
    }

    let (answer_sdp_tx, answer_sdp_rx) = tokio::sync::oneshot::channel();
    state
        .connecting_tx
        .send(Connecting {
            inner: Some(ConnectingInner {
                connection: conn,
                body,
                answer_sdp_tx,
            }),
            headers: parts.headers,
        })
        .await
        .map_err(|_e| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let answer_sdp = if let Some(answer_sdp) = answer_sdp_rx
        .await
        .map_err(|_e| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
    {
        answer_sdp
    } else {
        return Err(hyper::StatusCode::FORBIDDEN);
    };

    Ok(answer_sdp)
}

struct AppState {
    bind_addr: std::net::SocketAddr,
    connecting_tx: async_channel::Sender<Connecting>,
}

impl Server {
    pub async fn listen(
        listener: tokio::net::TcpListener,
        backlog: usize,
    ) -> Result<Self, std::io::Error> {
        let bind_addr = listener.local_addr()?;

        let (connecting_tx, connecting_rx) = async_channel::bounded(backlog);

        let app = axum::Router::new()
            .route("/", axum::routing::post(offer))
            .layer(
                tower_http::cors::CorsLayer::new()
                    .allow_methods([axum::http::Method::POST])
                    .allow_origin(tower_http::cors::Any),
            )
            .with_state(std::sync::Arc::new(AppState {
                bind_addr,
                connecting_tx: connecting_tx.clone(),
            }));

        tokio::task::spawn(async move {
            let _ = axum::serve(listener, app).await;
            connecting_tx.close();
        });

        Ok(Self { connecting_rx })
    }

    pub async fn accept(&self) -> Option<Connecting> {
        self.connecting_rx.recv().await.ok()
    }
}
