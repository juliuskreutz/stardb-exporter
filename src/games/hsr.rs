use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
};

use base64::prelude::*;
use reliquary::network::{
    GamePacket, GameSniffer,
    command::{command_id, proto},
};

pub fn sniff(
    achievement_ids: &[u32],
    device_rx: &mpsc::Receiver<Vec<u8>>,
) -> anyhow::Result<Vec<u32>> {
    let keys = load_keys()?;

    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut achievements = Vec::new();

    while let Ok(data) = device_rx.recv() {
        let Ok(packets) = sniffer.receive_packet(data) else {
            continue;
        };

        for packet in packets {
            let GamePacket::Commands(Ok(command)) = packet else {
                continue;
            };

            let Ok(quests) = (match command.command_id {
                command_id::GetQuestDataScRsp => command
                    .parse_proto::<proto::GetQuestDataScRsp::GetQuestDataScRsp>()
                    .map(|p| p.quest_list),
                command_id::BatchGetQuestDataScRsp => command
                    .parse_proto::<proto::BatchGetQuestDataScRsp::BatchGetQuestDataScRsp>()
                    .map(|p| p.quest_list),
                command_id::PlayerSyncScNotify => command
                    .parse_proto::<proto::PlayerSyncScNotify::PlayerSyncScNotify>()
                    .map(|p| p.quest_list),
                _ => continue,
            }) else {
                continue;
            };

            tracing::info!("Found achievement packet");

            for quest in quests {
                if achievement_ids.contains(&quest.id)
                    && (quest.status.value() == 2 || quest.status.value() == 3)
                {
                    achievements.push(quest.id);
                }
            }

            if !achievements.is_empty() {
                break;
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

fn load_keys() -> anyhow::Result<HashMap<u32, Vec<u8>>> {
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
