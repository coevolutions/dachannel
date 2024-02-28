use wasm_bindgen::prelude::*;

async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    let config: dachannel::Configuration = Default::default();
    let cb = dachannel::Connection::builder(config)?;
    let dc = cb.create_data_channel(
        "test",
        dachannel::DataChannelOptions {
            negotiated: true,
            id: Some(1),
            ..Default::default()
        },
    )?;
    let _conn = dachannel_client::ConnectOptions::new()
        .connect(cb, "http://127.0.0.1:12345".to_string())
        .await?;
    dc.send(b"hello world!!").await?;
    log::info!(
        "got: {:?}",
        String::from_utf8_lossy(&dc.recv().await.unwrap())
    );
    Ok(())
}

#[wasm_bindgen(start)]
pub async fn main_js() {
    main().await.unwrap()
}
