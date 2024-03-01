use std::io::{BufRead as _, Write as _};

use clap::Parser as _;

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    connect_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    let args = Args::parse();

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
        .connect(cb, &args.connect_url)
        .await?;

    let stdin = std::io::stdin();
    loop {
        print!("input> ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        dc.send(line.as_bytes()).await?;
        println!(
            "got: {:?}",
            String::from_utf8_lossy(&dc.recv().await.unwrap())
        );
    }

    Ok(())
}
