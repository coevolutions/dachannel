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
    println!("listening on: {}", listener.local_addr()?);

    let server = dachannel::server::Server::listen(listener, 128).await?;

    while let Some(connecting) = server.accept().await {
        println!("got connection");

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
