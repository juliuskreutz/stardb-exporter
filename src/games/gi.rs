use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
};

use auto_artifactarium::{matches_achievement_packet, GamePacket, GameSniffer};
use base64::prelude::*;

use regex::Regex;

pub fn sniff(
    achievement_ids: &[u32],
    device_rx: &mpsc::Receiver<Vec<u8>>,
) -> anyhow::Result<Vec<u32>> {
    let keys = load_keys()?;
    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut achievements = Vec::new();

    while let Ok(data) = device_rx.recv() {
        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
        };

        for command in commands {
            if let Some(read_achievements) = matches_achievement_packet(&command) {
                if !achievements.is_empty() {
                    continue;
                }

                for achievement in read_achievements {
                    if achievement_ids.contains(&achievement.id)
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
        return Err(anyhow::anyhow!("No achievements found"));
    }

    Ok(achievements)
}

fn load_keys() -> anyhow::Result<HashMap<u16, Vec<u8>>> {
    let keys: HashMap<u16, String> = serde_json::from_slice(include_bytes!("../../keys/gi.json"))?;

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
    log_path.push("miHoYo");

    let mut log_path_cn = log_path.clone();

    log_path.push("Genshin Impact");
    log_path_cn.push("原神");

    log_path.push("output_log.txt");
    log_path_cn.push("output_log.txt");

    let log_path = match (log_path.exists(), log_path_cn.exists()) {
        (true, _) => log_path,
        (_, true) => log_path_cn,
        _ => return Err(anyhow::anyhow!("Can't find log file")),
    };

    let re = Regex::new(r".:\\.+(GenshinImpact_Data|YuanShen_Data)")?;

    for line in BufReader::new(File::open(log_path)?).lines() {
        let Ok(line) = line else {
            break;
        };

        if let Some(m) = re.find(&line) {
            return Ok(PathBuf::from(m.as_str()));
        }
    }

    Err(anyhow::anyhow!("Couldn't find game path"))
}
