use clap::Parser as _;

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    listen_addr: std::net::SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    let args = Args::parse();

    let listener = tokio::net::TcpListener::bind(args.listen_addr).await?;
    let local_addr = listener.local_addr()?;
    println!("listening on: {}", local_addr);

    let (serve_fut, connecting_rx) = dachannel_server::ServeOptions::new()
        .ice_servers(vec![dachannel::IceServer {
            urls: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
                "stun:stun2.l.google.com:19302".to_string(),
                "stun:stun3.l.google.com:19302".to_string(),
                "stun:stun4.l.google.com:19302".to_string(),
            ],
            username: None,
            credential: None,
        }])
        .serve(listener);

    tokio::spawn(async move {
        serve_fut.await.unwrap();
    });

    while let Some(connecting) = connecting_rx.recv().await.ok() {
        let remote_addr = connecting.remote_addr().clone();
        println!("[{}] connected", remote_addr);

        let dc = connecting.create_data_channel(
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
