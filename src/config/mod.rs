use clap::Parser;

pub struct Config {}

#[derive(Parser)]
struct Cli {
    #[arg(
        short,
        long,
        value_name = "RCON upstream WebSocket endpoint connection string"
    )]
    rcon_upstream_ws_connection_string: String,
}

impl Config {
    pub fn get() -> Self {
        let cli = Cli::parse();

        println!("RCON addr: {}", cli.rcon_upstream_ws_connection_string);

        return Self {};
    }
}
