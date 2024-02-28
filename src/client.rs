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
