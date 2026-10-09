use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
};

use auto_discipher::{GamePacket, GameSniffer};
use base64::prelude::*;

pub fn sniff(
    achievement_ids: &[(u32, bool)],
    device_rx: &mpsc::Receiver<Vec<u8>>,
) -> anyhow::Result<Vec<u32>> {
    let keys = load_keys()?;

    let mut sniffer = GameSniffer::new()
        .set_initial_keys(keys)
        .set_achievement_ids(achievement_ids.iter().copied());

    let mut achievements = Vec::new();

    while let Ok(data) = device_rx.recv() {
        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
        };

        for command in commands {
            if let Some(read_achievements) = sniffer.matches_achievement_packet(&command) {
                tracing::info!("Found achievement packet");

                // The login burst carries both the regular and the arcade list, so
                // the second one found is not a repeat of the first.
                if !achievements.is_empty() {
                    continue;
                }

                for achievement in read_achievements {
                    // The regular list marks finished ones with `finished`, the
                    // arcade one with `completed`, which means the same thing.
                    if achievement.completed {
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
        return Err(anyhow::anyhow!("No achievements found"));
    }

    Ok(achievements)
}

/// The shipped dispatch keys, oldest first.
///
/// Read in file order and reversed, so the newest key is the one appended last.
/// Which one is right is decided by testing against the traffic rather than by
/// the version string, since the version is only a label. That way a capture
/// from an older build still decrypts after an update.
fn load_keys() -> anyhow::Result<Vec<Vec<u8>>> {
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
