#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("dachannel: {0}")]
    Dachannel(#[from] crate::Error),

    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("malformed body")]
    MalformedBody,
}

pub async fn connect(
    url: &str,
    authorization: Option<String>,
    conn: &crate::Connection,
) -> Result<(), Error> {
    conn.set_local_description(crate::SdpType::Offer).await?;
    conn.ice_candidates_gathered().await;
    let offer_sdp = conn.local_description()?.unwrap().sdp;

    let client = reqwest::Client::new();
    let mut req = client.post(url).body(offer_sdp);
    if let Some(authorization) = authorization {
        req = req.header(reqwest::header::AUTHORIZATION, authorization);
    }
    let res = req.send().await?.error_for_status()?;
    let answer_sdp =
        String::from_utf8(res.bytes().await?.to_vec()).map_err(|_| Error::MalformedBody)?;

    conn.set_remote_description(&crate::Description {
        type_: crate::SdpType::Answer,
        sdp: answer_sdp,
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "server")]
    #[tokio::test]
    pub async fn test_connect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let (serve_fut, connecting_rx) = crate::server::serve(listener, 128).await;

        tokio::spawn(async move {
            serve_fut.await.unwrap();
        });

        let client_jh = tokio::spawn(async move {
            let config: crate::Configuration = Default::default();
            let conn = crate::Connection::new(config).unwrap();
            let dc = conn
                .create_data_channel(
                    "test",
                    datachannel_facade::DataChannelOptions {
                        negotiated: true,
                        id: Some(1),
                        ..Default::default()
                    },
                )
                .unwrap();

            connect(
                &format!("http://127.0.0.1:{}", local_addr.port()),
                None,
                &conn,
            )
            .await
            .unwrap();

            dc.send(b"hello world").await.unwrap();
        });

        let connecting = connecting_rx.recv().await.unwrap();
        let dc = connecting
            .connection()
            .create_data_channel(
                "test",
                datachannel_facade::DataChannelOptions {
                    negotiated: true,
                    id: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();

        let _conn = connecting.await.unwrap();
        assert_eq!(dc.recv().await.unwrap(), b"hello world");

        client_jh.await.unwrap();
    }
}
