use std::net::TcpStream;
use serde::Serialize;
use tungstenite::{stream::MaybeTlsStream, WebSocket};

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconCommandIssued {
    Identifier: i32,
    Message: String,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct RconResponse {
    Message: String,
    Identifier: i32,
    Type: String,
    Stacktrace: String,
}

pub fn send_rcon_command(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    rcon_symbol: &str,
    timeout: &std::time::Duration,
) -> String {
    let mut rng = rand::thread_rng();
    let command_id = rand::Rng::gen_range(&mut rng, 0..9999);
    let rcon_command = RconCommandIssued {
        Identifier: command_id,
        Message: rcon_symbol.to_string(),
    };
    let cmd_serialized = serde_json::to_string(&rcon_command).unwrap();
    let ws_message_out = tungstenite::protocol::Message::text(cmd_serialized);

    let timestamp_send = std::time::SystemTime::now();
    socket.write(ws_message_out).unwrap();
    socket.flush().unwrap();

    loop {
        // only wait for a relevant response message till timeout
        let elapsed = timestamp_send.elapsed().unwrap();
        if elapsed >= *timeout {
            todo!(); // TODO: return some kinda error
        }

        // TODO: if no message is ever received, we'll be stuck here. fix! (make the given timeout cover this case too)
        let ws_message_in = socket.read().unwrap();
        let text = ws_message_in.to_text().unwrap();
        let rcon_response: RconResponse;
        match serde_json::from_str(text) {
            Ok(n) => {
                rcon_response = n;
            },
            Err(_) => {
                continue;
            },
        }
        if rcon_response.Identifier == rcon_command.Identifier {
            return rcon_response.Message;
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PlayerInfo {
    Address: String,
    ConnectedSeconds: u32,
    CurrentLevel: f64,
    DisplayName: String,
    Health: f64,
    OwnerSteamID: String,
    Ping: u32,
    SteamID: String,
    UnspentXp: f64,
    VoiationLevel: f64,
}
pub type PlayerList = Vec<PlayerInfo>;

/// RCON command `global.playerlist`
pub fn global_playerlist(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: &std::time::Duration,
) -> PlayerList {
    let rcon_symbol = "global.playerlist";
    let response_raw = send_rcon_command(websocket, rcon_symbol, timeout);
    let response_parsed = serde_json::from_str(&response_raw);
    return response_parsed.unwrap();
}

#[derive(Debug, Serialize)]
pub struct EnvTime(pub f64);

#[derive(Debug, PartialEq, Serialize)]
pub struct RconPosition {
    /// horizontal offset from the map's center
    pub x: f64,
    /// vertical offset from the map's center
    pub y: f64,
    /// altitude
    pub z: f64,
}

#[derive(Debug, Serialize)]
pub struct Player {
    pub address: String,
    pub connected_seconds: u32,
    pub display_name: String,
    pub health: f64,
    pub id: String,
    pub position: RconPosition,
}
#[derive(Debug, PartialEq, Serialize)]
pub struct ToolCupboard {
    pub id: String,
    pub position: RconPosition,
    pub auth_count: u32,
}
#[derive(Debug, Serialize)]
pub struct State {
    /// List of players on the server.
    pub players: Vec<Player>,
    /// List of toolcupboards on the server.
    pub tcs: Vec<ToolCupboard>,
    /// Game time as reported by RCON -- a decimal representation of 24-hour clock.
    pub game_time: EnvTime,
    /// When the RCON state was synced -- Unix timestamp in milliseconds.
    pub sync_time_ms: u128,
}

pub fn merge_playerlists(playerlistpos: PlayerPosList, playerlist: PlayerList) -> Vec<Player> {
    let mut players = vec![];

    let mut iterable_playerlistpos = playerlistpos.iter();
    for player in playerlist {
        let player_position: RconPosition;
        let player_positioned = iterable_playerlistpos.find(|x| x.steamd_id == player.SteamID);
        match player_positioned {
            Some(player_positioned) => {
                player_position = RconPosition {
                    x: player_positioned.position.0,
                    z: player_positioned.position.1,
                    y: player_positioned.position.2,
                }
            }
            None => {
                continue; // TODO: log a warning -- no position information found for some player
            }
        }
        let p = Player {
            address: player.Address,
            connected_seconds: player.ConnectedSeconds,
            display_name: player.DisplayName,
            health: player.Health,
            id: player.SteamID,
            position: player_position,
        };
        players.push(p);
    }

    return players;
}

/// RCON command `env.time`
pub fn env_time(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: &std::time::Duration,
) -> Option<EnvTime> {
    let rcon_symbol = "env.time";
    let response_raw = send_rcon_command(websocket, rcon_symbol, timeout);

    // Match the float in e.g. `env.time: "10.63853"`
    let re = regex::Regex::new(r#"env\.time:\s*"(\d+\.\d+)""#).unwrap();
    match re.captures(&response_raw) {
        Some(captures) => {
            let match_group = &captures[1];
            let float = match_group.parse::<f64>().unwrap();
            return Some(EnvTime(float));
        },
        None => {
            eprintln!("Failed to parse env.time response:\n{}", response_raw);
            return None;
        },
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PlayerPos {
    steamd_id: String,
    position: (f64, f64, f64),
    rotation: (f64, f64, f64),
}
pub type PlayerPosList = Vec<PlayerPos>;

/// RCON command `global.playerlistpos`
pub fn global_playerlistpos(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: &std::time::Duration,
) -> PlayerPosList {
    let rcon_symbol = "global.playerlistpos";
    let response_raw = send_rcon_command(websocket, rcon_symbol, timeout);

    let mut player_list: PlayerPosList = Vec::new();
    let mut line_number = 0;
    for line in response_raw.lines() {
        line_number = line_number + 1;

        if line_number == 1 {
            continue;
        }

        player_list.push(parse_playerlistpos(line));
    }

    return player_list;
}

/// RCON command `global.listtoolcupboards`
pub fn global_listtoolcupboards(
    websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: &std::time::Duration,
) -> Vec<ToolCupboard> {
    let rcon_symbol = "global.listtoolcupboards";
    let response_raw = send_rcon_command(websocket, rcon_symbol, timeout);

    let mut tc_list: Vec<ToolCupboard> = Vec::new();
    let mut line_number = 0;
    for line in response_raw.lines() {
        line_number = line_number + 1;

        if line_number == 1 {
            continue;
        }
        let tc = parse_listtoolcupboards(line);
        tc_list.push(tc);
    }

    return tc_list;
}

fn parse_playerlistpos(arg: &str) -> PlayerPos {
    let re = regex::Regex::new(r#"(\d{17})(.*)\((.*)\)\s*\((.*)\)"#).unwrap();
    let captures = re.captures(arg).unwrap();
    let steam_id_raw = captures[1].to_string();
    let player_position_raw = captures[3].to_string();
    let player_rotation_raw = captures[4].to_string();
    return PlayerPos {
        position: parse_float_triple(&player_position_raw),
        rotation: parse_float_triple(&player_rotation_raw),
        steamd_id: steam_id_raw.to_string(),
    };
}

fn parse_listtoolcupboards(arg: &str) -> ToolCupboard {
    let re = regex::Regex::new(r#"(\d+)\s+\((.*)\)\s+(\d+)"#).unwrap();
    let captures = re.captures(arg).unwrap();
    let entity_id_raw = captures[1].to_string();
    let position_raw = captures[2].to_string();
    let auth_count_raw = captures[3].to_string();
    let pos = parse_float_triple(&position_raw);
    return ToolCupboard {
        id: entity_id_raw.to_string(),
        position: RconPosition {
            x: pos.0,
            z: pos.1,
            y: pos.2,
        },
        auth_count: auth_count_raw.parse::<u32>().unwrap(),
    };
}

fn parse_float_triple(arg: &String) -> (f64, f64, f64) {
    let parts: Vec<String> = arg.split(",").map(String::from).collect();

    let mut parsed = (0.0, 0.0, 0.0);

    for idx in 0..=2 {
        let part = &parts[idx];
        let trimmed = part.trim();
        let float = trimmed.parse::<f64>().unwrap();
        match idx {
            0 => {
                parsed.0 = float;
            }
            1 => {
                parsed.1 = float;
            }
            2 => {
                parsed.2 = float;
            }
            _ => todo!(),
        }
    }

    return parsed;
}

// TODO: move tests in a separate file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_float_triple() {
        assert_eq!(
            parse_float_triple(&"821.94, 0.00, 676.77".to_string()),
            (821.94, 0.00, 676.77)
        );
        assert_eq!(
            parse_float_triple(&"821.94, 0.00, -676.77".to_string()),
            (821.94, 0.00, -676.77)
        );
    }

    #[test]
    fn test_parse_playerlistpos() {
        assert_eq!(
            parse_playerlistpos(
                "76561198135242017 Jeti        (-1027.08, 0.31, 668.11) (-0.72, 0.00, 0.69)",
            ),
            PlayerPos {
                position: (-1027.08, 0.31, 668.11),
                rotation: (-0.72, 0.00, 0.69),
                steamd_id: "76561198135242017".to_string(),
            }
        );

        assert_eq!(
            parse_playerlistpos(
                "76561198347416108 TheRedGam3r (-1804.55, 33.14, -696.72) (0.23, -0.59, 0.77)",
            ),
            PlayerPos {
                position: (-1804.55, 33.14, -696.72),
                rotation: (0.23, -0.59, 0.77),
                steamd_id: "76561198347416108".to_string(),
            }
        );

        assert_eq!(
            parse_playerlistpos(
                "76561198135242017 Raudus      (1627.04, 2.02, 1795.76)   (-0.15, 0.04, 0.99)",
            ),
            PlayerPos {
                position: (1627.04, 2.02, 1795.76),
                rotation: (-0.15, 0.04, 0.99),
                steamd_id: "76561198135242017".to_string(),
            }
        );

        assert_eq!(
            parse_playerlistpos(
                "76561199278150966 softside bandit Erkki (-138.61, 8.61, -634.68) (0.94, -0.17, 0.30)",
            ),
            PlayerPos {
                position: (-138.61, 8.61, -634.68),
                rotation: (0.94, -0.17, 0.30),
                steamd_id: "76561199278150966".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_listtoolcupboards() {
        assert_eq!(
            parse_listtoolcupboards("754298   (278.51, 37.18, 83.82)     0",),
            ToolCupboard {
                auth_count: 0,
                id: "754298".to_string(),
                position: RconPosition {
                    x: 278.51,
                    z: 37.18,
                    y: 83.82
                },
            }
        );

        assert_eq!(
            parse_listtoolcupboards("55220    (-211.04, 7.62, -605.48) 1",),
            ToolCupboard {
                auth_count: 1,
                id: "55220".to_string(),
                position: RconPosition {
                    x: -211.04,
                    z: 7.62,
                    y: -605.48
                },
            }
        );
    }
}