pub async fn connect(
    url: &str,
    headers: reqwest::header::HeaderMap,
    conn: &crate::Connection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    conn.set_local_description(crate::SdpType::Offer).await?;
    conn.ice_candidates_gathered().await;
    let offer_sdp = conn.local_description()?.unwrap().sdp;

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .headers(headers)
        .body(offer_sdp)
        .send()
        .await?
        .error_for_status()?;
    let answer_sdp = String::from_utf8(res.bytes().await?.to_vec())?;

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

    #[tokio::test]
    pub async fn test_connect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let server = crate::server::Server::listen(listener, 128).await.unwrap();

        tokio::spawn(async move {
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
                Default::default(),
                &conn,
            )
            .await
            .unwrap();

            dc.send(b"hello world").await.unwrap();
        });

        let connecting = server.accept().await.unwrap();
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

        let _conn = connecting.finish().await.unwrap();
        assert_eq!(dc.recv().await.unwrap(), b"hello world");
    }
}
