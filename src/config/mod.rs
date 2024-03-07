use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(
        short,
        long,
        value_name = "RCON upstream WebSocket endpoint connection string"
    )]
    rcon_upstream_ws_connection_string: String,
}

pub struct Config {
    pub rcon_upstream_ws_connection_string: String,
}
impl Config {
    pub fn get() -> Self {
        let cli = Cli::parse();

        return Self {
            rcon_upstream_ws_connection_string: cli.rcon_upstream_ws_connection_string,
        };
    }
}
