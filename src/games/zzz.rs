use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use auto_discipher::{GamePacket, GameSniffer};
use base64::prelude::*;

/// How long to keep listening for the second achievement list once the first
/// one is in.
const SECOND_LIST_GRACE: Duration = Duration::from_secs(10);

pub fn sniff(
    achievement_ids: &[(u32, bool)],
    device_rx: &mpsc::Receiver<Vec<u8>>,
) -> anyhow::Result<Vec<u32>> {
    let keys = load_keys()?;

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
        let data = match deadline {
            None => match device_rx.recv() {
                Ok(data) => data,
                Err(_) => break,
            },
            Some(deadline) => {
                let timeout = deadline.saturating_duration_since(Instant::now());
                match device_rx.recv_timeout(timeout) {
                    Ok(data) => data,
                    Err(_) => {
                        tracing::info!("Second achievement list not seen, exporting what was found");
                        break;
                    }
                }
            }
        };

        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
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

    let achievements: Vec<u32> = regular.into_iter().chain(arcade).flatten().collect();
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
