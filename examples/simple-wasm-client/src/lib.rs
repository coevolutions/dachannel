use wasm_bindgen::prelude::*;

async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    let config: dachannel::Configuration = Default::default();
    let conn = dachannel::Connection::new(config)?;
    let dc = conn.create_data_channel(
        "test",
        dachannel::DataChannelOptions {
            negotiated: true,
            id: Some(1),
            ..Default::default()
        },
    )?;
    dachannel_client::connect("http://127.0.0.1:12345", None, &conn).await?;
    log::info!("got: {:?}", dc.recv().await);
    dc.send(b"hello world!!").await?;
    Ok(())
}

#[wasm_bindgen(start)]
pub async fn main_js() {
    main().await.unwrap()
}
