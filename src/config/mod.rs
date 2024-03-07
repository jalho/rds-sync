use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(
        short,
        long,
        value_name = "RCON connection string",
        help = r#"RCON WebSocket endpoint connection string. For example: "ws://127.0.0.1:28016/Your_Rcon_Password""#
    )]
    rcon_connection: String,
}

pub struct Config {
    pub rcon_connection: String,
}
impl Config {
    pub fn get() -> Self {
        let cli = Cli::parse();

        return Self {
            rcon_connection: cli.rcon_connection,
        };
    }
}
