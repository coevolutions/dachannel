use clap::Parser as _;

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    listen_addr: std::net::SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let listener = tokio::net::TcpListener::bind(args.listen_addr).await?;
    let local_addr = listener.local_addr()?;
    println!("listening on: {}", local_addr);

    let (serve_fut, connecting_rx) = dachannel::server::listen(listener, 128).await?;

    tokio::spawn(async move {
        serve_fut.await.unwrap();
    });

    while let Some(connecting) = connecting_rx.recv().await.ok() {
        println!("got connection from {}", connecting.remote_addr());

        let dc = connecting.connection().create_data_channel(
            "test",
            dachannel::DataChannelOptions {
                negotiated: true,
                id: Some(1),
                ..Default::default()
            },
        )?;
        let _pc = connecting.finish().await?;

        dc.send(b"hello world").await?;
        println!("got: {:?}", dc.recv().await);
    }
    Ok(())
}
