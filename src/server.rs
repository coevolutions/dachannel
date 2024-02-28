use datachannel_facade::platform::native::ConfigurationExt as _;
use http_body_util::BodyExt as _;
use hyper::body::Body as _;

pub struct Server<T> {
    conn_rx: async_channel::Receiver<(crate::Connection, T)>,
}

async fn offer<T>(
    conn_tx: async_channel::Sender<(crate::Connection, T)>,
    initialize_conn: impl Fn(
            &hyper::HeaderMap,
            &crate::Connection,
        ) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
        + Send
        + Sync
        + Clone
        + 'static,
    bind_addr: std::net::SocketAddr,
    req: hyper::Request<hyper::body::Incoming>,
) -> Result<
    hyper::Response<http_body_util::Full<hyper::body::Bytes>>,
    Box<dyn std::error::Error + Send + Sync>,
>
where
    T: Send + Sync + 'static,
{
    if req.method() != hyper::Method::POST {
        let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
            hyper::StatusCode::METHOD_NOT_ALLOWED
                .canonical_reason()
                .unwrap_or(""),
        )));
        *resp.status_mut() = hyper::StatusCode::METHOD_NOT_ALLOWED;
        return Ok(resp);
    }

    let (parts, body) = req.into_parts();

    let mut config: crate::Configuration = Default::default();
    config.set_bind(bind_addr.ip(), bind_addr.port(), bind_addr.port());

    let conn = crate::Connection::new(config)?;

    let r = initialize_conn(&parts.headers, &conn)?;

    if body.size_hint().upper().unwrap_or(u64::MAX) > 8 * 1024 {
        let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
            hyper::StatusCode::PAYLOAD_TOO_LARGE
                .canonical_reason()
                .unwrap_or(""),
        )));
        *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
        return Ok(resp);
    }

    let offer_sdp = String::from_utf8(body.collect().await?.to_bytes().to_vec())?;

    conn.set_remote_description(&crate::Description {
        type_: crate::SdpType::Offer,
        sdp: offer_sdp,
    })
    .await?;
    conn.set_local_description(crate::SdpType::Answer).await?;
    conn.ice_candidates_gathered().await;

    // TODO: Remove this unwrap?
    let answer_sdp = conn.local_description()?.unwrap().sdp;

    let _ = conn_tx.send((conn, r)).await;

    Ok(hyper::Response::new(http_body_util::Full::new(
        hyper::body::Bytes::from(answer_sdp),
    )))
}

impl<T> Server<T>
where
    T: Send + Sync + 'static,
{
    pub async fn listen(
        addr: std::net::SocketAddr,
        initialize_conn: impl Fn(
                &hyper::HeaderMap,
                &crate::Connection,
            ) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
            + Send
            + Sync
            + Clone
            + 'static,
        backlog: usize,
    ) -> Result<Self, std::io::Error> {
        let listener = tokio::net::TcpListener::bind(addr.clone()).await?;

        let (conn_tx, conn_rx) = async_channel::bounded(backlog);
        tokio::task::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok((stream, addr)) => (stream, addr),
                    Err(err) => {
                        log::error!("Error accepting connection: {:?}", err);
                        break;
                    }
                };
                let io = hyper_util::rt::TokioIo::new(stream);

                let bind_addr = addr.clone();
                let conn_tx = conn_tx.clone();
                let initialize_conn = initialize_conn.clone();

                tokio::task::spawn(async move {
                    if let Err(err) = hyper::server::conn::http1::Builder::new()
                        .serve_connection(
                            io,
                            hyper::service::service_fn(move |req| {
                                offer::<T>(conn_tx.clone(), initialize_conn.clone(), bind_addr, req)
                            }),
                        )
                        .await
                    {
                        log::error!("Error serving connection: {:?}", err);
                    }
                });
            }
        });

        Ok(Self { conn_rx })
    }

    pub async fn accept(&self) -> Option<(crate::Connection, T)> {
        self.conn_rx.recv().await.ok()
    }
}
