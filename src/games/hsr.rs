use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use auto_reliquary::{
    ConnectionPacket, GamePacket, GameSniffer, KeyState, SniffError, matches_achievement_packet,
};
use base64::prelude::*;

use super::{Achievements, ExportError, KeyStage, PacketStream, Received};

impl From<SniffError> for ExportError {
    fn from(e: SniffError) -> Self {
        match e {
            SniffError::InitialKeyMissing {
                version,
                saw_handshake,
            } => Self::KeyMissing {
                version,
                saw_handshake,
            },
            SniffError::SessionSeedMissing => Self::SessionSeedMissing,
            SniffError::SessionKeyLost => Self::SessionKeyLost,
        }
    }
}

impl From<KeyState> for KeyStage {
    fn from(state: KeyState) -> Self {
        match state {
            KeyState::None => Self::None,
            KeyState::Dispatch => Self::Dispatch,
            KeyState::Session => Self::Session,
        }
    }
}

pub fn sniff(
    achievement_ids: &[(u32, bool)],
    stream: &mut PacketStream,
) -> Result<Achievements, ExportError> {
    sniff_with_keys(load_keys()?, achievement_ids, stream)
}

pub fn sniff_with_keys(
    keys: HashMap<u32, Vec<u8>>,
    achievement_ids: &[(u32, bool)],
    stream: &mut PacketStream,
) -> Result<Achievements, ExportError> {
    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut achievements = Vec::new();

    loop {
        let data = match stream.next(None) {
            Received::Packet(packet) => packet.data,
            Received::Stopped => return Err(ExportError::Cancelled),
            Received::Deadline | Received::Ended => break,
        };

        let Some(packet) = sniffer.receive_packet(data)? else {
            continue;
        };
        stream.note_key(sniffer.key_state().into());

        let commands = match packet {
            GamePacket::Commands(commands) => commands,
            GamePacket::Connection(ConnectionPacket::HandshakeRequested) => {
                stream.progress.handshakes += 1;
                continue;
            }
            GamePacket::Connection(ConnectionPacket::Disconnected) => {
                stream.progress.disconnects += 1;
                continue;
            }
            GamePacket::Connection(_) => continue,
        };

        for command in commands {
            if let Some(read_achievements) = matches_achievement_packet(&command) {
                tracing::info!("Found achievement packet");

                if !achievements.is_empty() {
                    continue;
                }

                for achievement in read_achievements {
                    if achievement_ids.iter().any(|(id, _)| *id == achievement.id)
                        && (achievement.status == 2 || achievement.status == 3)
                    {
                        achievements.push(achievement.id);
                    }
                }
            }
        }

        if !achievements.is_empty() {
            break;
        }
    }

    if achievements.is_empty() {
        return Err(ExportError::CaptureEnded);
    }

    Ok(Achievements {
        ids: achievements,
        breakdown: None,
        capture_report: None,
    })
}

pub fn load_keys() -> anyhow::Result<HashMap<u32, Vec<u8>>> {
    let keys: HashMap<u32, String> = serde_json::from_slice(include_bytes!("../../keys/hsr.json"))?;

    let mut keys_bytes = HashMap::new();

    for (k, v) in keys {
        keys_bytes.insert(k, BASE64_STANDARD.decode(v)?);
    }

    Ok(keys_bytes)
}

pub fn game_path() -> anyhow::Result<PathBuf> {
    let mut log_path = PathBuf::from(&std::env::var("APPDATA")?);
    log_path.pop();
    log_path.push("LocalLow");

    let mut log_path_cn = log_path.clone();

    log_path.push("Cognosphere");
    log_path_cn.push("miHoYo");

    log_path.push("Star Rail");
    log_path_cn.push("崩坏：星穹铁道");

    log_path.push("Player.log");
    log_path_cn.push("Player.log");

    let log_path = match (log_path.exists(), log_path_cn.exists()) {
        (true, _) => log_path,
        (_, true) => log_path_cn,
        _ => return Err(anyhow::anyhow!("Can't find log file")),
    };

    for line in BufReader::new(File::open(log_path)?).lines() {
        let Ok(line) = line else {
            break;
        };

        if let Some(line) = line.strip_prefix("Loading player data from ") {
            let mut path = PathBuf::from(line);

            path.pop();

            return Ok(path);
        }
    }

    Err(anyhow::anyhow!("Couldn't find game path"))
}
