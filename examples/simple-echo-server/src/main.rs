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

    let (serve_fut, connecting_rx) = dachannel_server::serve(listener, 128).await;

    tokio::spawn(async move {
        serve_fut.await.unwrap();
    });

    while let Some(connecting) = connecting_rx.recv().await.ok() {
        let remote_addr = connecting.remote_addr().clone();
        println!("[{}] connected", remote_addr);

        let dc = connecting.connection().create_data_channel(
            "test",
            dachannel::DataChannelOptions {
                negotiated: true,
                id: Some(1),
                ..Default::default()
            },
        )?;

        let pc = connecting.await?;

        tokio::spawn(async move {
            let _pc = pc;
            loop {
                let buf = match dc.recv().await {
                    Ok(buf) => buf,
                    Err(e) => {
                        println!("[{}] disconnected: {}", remote_addr, e);
                        break;
                    }
                };

                println!("[{}] {:?}", remote_addr, String::from_utf8_lossy(&buf));
                dc.send(&buf).await.unwrap();
            }
        });
    }
    Ok(())
}