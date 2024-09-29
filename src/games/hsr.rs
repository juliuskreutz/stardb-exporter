use std::{collections::HashMap, sync::mpsc};

use base64::prelude::*;
use reliquary::network::{
    gen::{command_id, proto::GetQuestDataScRsp::GetQuestDataScRsp},
    GamePacket, GameSniffer,
};

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
            if command.command_id == command_id::GetQuestDataScRsp {
                if !achievements.is_empty() {
                    continue;
                }

                println!("Got achievements packet");

                if let Ok(quest_data) = command.parse_proto::<GetQuestDataScRsp>() {
                    for quest in quest_data.quest_list {
                        if achievement_ids.contains(&quest.id)
                            && (quest.status.value() == 2 || quest.status.value() == 3)
                        {
                            achievements.push(quest.id);
                        }
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

fn load_keys() -> anyhow::Result<HashMap<u32, Vec<u8>>> {
    let keys: HashMap<u32, String> = serde_json::from_slice(include_bytes!("../../hsr_keys.json"))?;

    let mut keys_bytes = HashMap::new();

    for (k, v) in keys {
        keys_bytes.insert(k, BASE64_STANDARD.decode(v)?);
    }

    Ok(keys_bytes)
}
