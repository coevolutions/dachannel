use std::io::{BufRead as _, Write as _};

use clap::Parser as _;

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    connect_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

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
    dachannel_client::connect(&args.connect_url, None, &conn).await?;

    let stdin = std::io::stdin();
    loop {
        print!("input> ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        dc.send(line.as_bytes()).await?;
        println!(
            "got: {:?}",
            String::from_utf8_lossy(&dc.recv().await.unwrap())
        );
    }
}
