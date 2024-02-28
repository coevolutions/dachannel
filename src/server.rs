use datachannel_facade::platform::native::ConfigurationExt as _;
use http_body_util::BodyExt as _;
use hyper::body::Body as _;

pub struct Server {
    connecting_rx: async_channel::Receiver<Connecting>,
}

struct ConnectingInner {
    connection: crate::Connection,
    body: hyper::body::Incoming,
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
    connecting_tx: async_channel::Sender<Connecting>,
    bind_addr: std::net::SocketAddr,
    req: hyper::Request<hyper::body::Incoming>,
) -> Result<
    hyper::Response<http_body_util::Full<hyper::body::Bytes>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
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

    if body.size_hint().upper().unwrap_or(u64::MAX) > 8 * 1024 {
        let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
            hyper::StatusCode::PAYLOAD_TOO_LARGE
                .canonical_reason()
                .unwrap_or(""),
        )));
        *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
        return Ok(resp);
    }

    let (answer_sdp_tx, answer_sdp_rx) = tokio::sync::oneshot::channel();
    connecting_tx
        .send(Connecting {
            inner: Some(ConnectingInner {
                connection: conn,
                body,
                answer_sdp_tx,
            }),
            headers: parts.headers,
        })
        .await?;

    let answer_sdp = if let Some(answer_sdp) = answer_sdp_rx.await? {
        answer_sdp
    } else {
        let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
            hyper::StatusCode::FORBIDDEN
                .canonical_reason()
                .unwrap_or(""),
        )));
        *resp.status_mut() = hyper::StatusCode::FORBIDDEN;
        return Ok(resp);
    };

    let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
        answer_sdp,
    )));
    resp.headers_mut().append(
        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        hyper::header::HeaderValue::from_str("*").unwrap(),
    );
    resp.headers_mut().append(
        hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
        hyper::header::HeaderValue::from_str("POST").unwrap(),
    );
    Ok(resp)
}

impl Server {
    pub async fn listen(
        listener: tokio::net::TcpListener,
        backlog: usize,
    ) -> Result<Self, std::io::Error> {
        let bind_addr = listener.local_addr()?;

        let (connecting_tx, connecting_rx) = async_channel::bounded(backlog);
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

                let bind_addr = bind_addr.clone();
                let connecting_tx = connecting_tx.clone();

                tokio::task::spawn(async move {
                    if let Err(err) = hyper::server::conn::http1::Builder::new()
                        .serve_connection(
                            io,
                            hyper::service::service_fn(move |req| {
                                offer(connecting_tx.clone(), bind_addr, req)
                            }),
                        )
                        .await
                    {
                        log::error!("Error serving connection: {:?}", err);
                    }
                });
            }
        });

        Ok(Self { connecting_rx })
    }

    pub async fn accept(&self) -> Option<Connecting> {
        self.connecting_rx.recv().await.ok()
    }
}
