use datachannel_facade::platform::native::ConfigurationExt as _;
use http_body_util::BodyExt as _;
use hyper::body::Body as _;

pub struct Server {}

async fn hello(
    bind_addr: std::net::SocketAddr,
    req: hyper::Request<hyper::body::Incoming>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, hyper::Error> {
    if req.method() != hyper::Method::POST {
        let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
            hyper::StatusCode::METHOD_NOT_ALLOWED
                .canonical_reason()
                .unwrap_or(""),
        )));
        *resp.status_mut() = hyper::StatusCode::METHOD_NOT_ALLOWED;
        return Ok(resp);
    }

    let (_parts, body) = req.into_parts();

    if body.size_hint().upper().unwrap_or(u64::MAX) > 8 * 1024 {
        let mut resp = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
            hyper::StatusCode::PAYLOAD_TOO_LARGE
                .canonical_reason()
                .unwrap_or(""),
        )));
        *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
        return Ok(resp);
    }

    let offer_sdp = String::from_utf8(body.collect().await?.to_bytes().to_vec()).unwrap();

    let mut config: crate::Configuration = Default::default();
    config.set_bind(bind_addr.ip(), bind_addr.port(), bind_addr.port());

    let conn = crate::Connection::new(config).unwrap();
    conn.set_remote_description(&crate::Description {
        type_: crate::SdpType::Offer,
        sdp: offer_sdp,
    })
    .await
    .unwrap();
    conn.set_local_description(crate::SdpType::Answer)
        .await
        .unwrap();
    conn.ice_candidates_gathered().await;

    Ok(hyper::Response::new(http_body_util::Full::new(
        hyper::body::Bytes::from(conn.local_description().unwrap().unwrap().sdp),
    )))
}

impl Server {
    pub async fn new(addr: std::net::SocketAddr) -> Result<Self, std::io::Error> {
        let listener = tokio::net::TcpListener::bind(addr.clone()).await?;
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
                tokio::task::spawn(async move {
                    if let Err(err) = hyper::server::conn::http1::Builder::new()
                        .serve_connection(
                            io,
                            hyper::service::service_fn(|req| hello(bind_addr, req)),
                        )
                        .await
                    {
                        log::error!("Error serving connection: {:?}", err);
                    }
                });
            }
        });
        Ok(Self {})
    }
}
