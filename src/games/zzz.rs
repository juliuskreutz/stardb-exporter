use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    time::{Duration, Instant},
};

use auto_discipher::{ConnectionPacket, GamePacket, GameSniffer, KeyState, SniffError};
use base64::prelude::*;

use super::{Achievements, Breakdown, ExportError, KeyStage, PacketStream, Received};

/// How long to keep listening for the second achievement list once the first
/// one is in.
const SECOND_LIST_GRACE: Duration = Duration::from_secs(10);

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
            SniffError::BruteforceFailed => Self::BruteforceFailed,
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
    keys: Vec<Vec<u8>>,
    achievement_ids: &[(u32, bool)],
    stream: &mut PacketStream,
) -> Result<Achievements, ExportError> {
    let mut sniffer = GameSniffer::new()
        .set_initial_keys(keys)
        .set_achievement_ids(achievement_ids.iter().copied());

    // The two lists are told apart by their ids: the arcade one holds only
    // arcade ids, and the sniffer keeps returning repeats of either.
    let arcade_ids: HashSet<u32> = achievement_ids
        .iter()
        .filter(|(_, arcade)| *arcade)
        .map(|(id, _)| *id)
        .collect();

    let mut regular: Option<Vec<u32>> = None;
    let mut arcade: Option<Vec<u32>> = None;
    // Set once the first list is in. The login burst carries both lists within
    // milliseconds of each other, but the arcade one is not guaranteed to be in
    // it, so the second is only waited for this long rather than forever.
    let mut deadline: Option<Instant> = None;

    loop {
        let packet = match stream.next(deadline) {
            Received::Packet(packet) => packet,
            Received::Stopped => return Err(ExportError::Cancelled),
            Received::Ended => break,
            Received::Deadline => {
                tracing::info!("Second achievement list not seen, exporting what was found");
                break;
            }
        };

        let packet = match sniffer.receive_packet_at(packet.data, packet.timestamp_ms) {
            Ok(Some(packet)) => packet,
            Ok(None) => continue,
            // One list is still worth more than none.
            Err(e) if regular.is_some() || arcade.is_some() => {
                tracing::warn!("{e}, exporting what was found");
                break;
            }
            Err(e) => return Err(e.into()),
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
            let Some(read_achievements) = sniffer.matches_achievement_packet(&command) else {
                continue;
            };

            let is_arcade = read_achievements
                .first()
                .is_some_and(|achievement| arcade_ids.contains(&achievement.id));
            let slot = if is_arcade { &mut arcade } else { &mut regular };
            if slot.is_some() {
                continue;
            }
            tracing::info!(arcade = is_arcade, "Found achievement packet");

            // The regular list marks finished ones with `finished`, the arcade
            // one with `completed`, which means the same thing.
            *slot = Some(
                read_achievements
                    .into_iter()
                    .filter(|achievement| achievement.completed)
                    .map(|achievement| achievement.id)
                    .collect(),
            );
            deadline.get_or_insert_with(|| Instant::now() + SECOND_LIST_GRACE);
        }

        if regular.is_some() && arcade.is_some() {
            break;
        }
    }

    if regular.is_none() && arcade.is_none() {
        return Err(ExportError::CaptureEnded);
    }

    let breakdown = Breakdown {
        regular: regular.as_ref().map(Vec::len),
        arcade: arcade.as_ref().map(Vec::len),
    };
    let ids: Vec<u32> = regular.into_iter().chain(arcade).flatten().collect();
    if ids.is_empty() {
        return Err(ExportError::Other(
            "The achievement lists came through, but none of them are finished".to_string(),
        ));
    }

    Ok(Achievements {
        ids,
        breakdown: Some(breakdown),
        capture_report: None,
    })
}

/// The shipped dispatch keys, oldest first.
///
/// Read in file order and reversed, so the newest key is the one appended last.
/// Which one is right is decided by testing against the traffic rather than by
/// the version string, since the version is only a label. That way a capture
/// from an older build still decrypts after an update.
pub fn load_keys() -> anyhow::Result<Vec<Vec<u8>>> {
    let keys: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(include_bytes!("../../keys/zzz.json"))?;

    let mut keys_bytes = keys
        .values()
        .map(|key| BASE64_STANDARD.decode(key.as_str().unwrap()))
        .collect::<Result<Vec<_>, _>>()?;
    keys_bytes.reverse();

    Ok(keys_bytes)
}

pub fn game_path() -> anyhow::Result<PathBuf> {
    let mut log_path = PathBuf::from(&std::env::var("APPDATA")?);
    log_path.pop();
    log_path.push("LocalLow");
    log_path.push("miHoYo");

    let mut log_path_cn = log_path.clone();

    log_path.push("ZenlessZoneZero");
    log_path_cn.push("绝区零");

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

        if let Some(line) = line.strip_prefix("[Subsystems] Discovering subsystems at path ") {
            let mut path = PathBuf::from(line);

            path.pop();

            return Ok(path);
        }
    }

    Err(anyhow::anyhow!("Couldn't find game path"))
}
