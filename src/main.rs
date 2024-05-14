use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use pcap::{ConnectionStatus, Device};
use reliquary::network::{
    gen::{
        command_id,
        proto::{GetBagScRsp::GetBagScRsp, GetQuestDataScRsp::GetQuestDataScRsp},
    },
    GamePacket, GameSniffer,
};
use std::{collections::HashMap, fs::File, io::BufWriter, sync::mpsc};

#[derive(serde::Deserialize)]
struct Id {
    id: u32,
}

#[derive(serde::Serialize)]
struct Jason {
    achievements: Vec<u32>,
    books: Vec<u32>,
}

fn main() -> Result<()> {
    let achievements: Vec<Id> = ureq::get("https://stardb.gg/api/achievements")
        .call()?
        .into_json()?;
    let achievement_ids: Vec<_> = achievements.into_iter().map(|a| a.id).collect();

    let books: Vec<Id> = ureq::get("https://stardb.gg/api/books")
        .call()?
        .into_json()?;
    let book_ids: Vec<_> = books.into_iter().map(|a| a.id).collect();

    let keys = load_online_keys()?;

    let device = pcap::Device::list()?
        .into_iter()
        .filter(|d| d.flags.connection_status == ConnectionStatus::Connected)
        .filter(|d| !d.addresses.is_empty())
        .find(|d| !d.flags.is_loopback())
        .unwrap();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || capture_device(device, tx));

    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut achievements = Vec::new();
    let mut books = Vec::new();

    while let Ok(data) = rx.recv() {
        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
        };

        for command in commands {
            if command.command_id == command_id::GetQuestDataScRsp {
                if !achievements.is_empty() {
                    continue;
                }

                let quest_data: GetQuestDataScRsp = command.parse_proto()?;

                for quest in quest_data.quest_list {
                    if achievement_ids.contains(&quest.id) && quest.status.value() == 3 {
                        achievements.push(quest.id);
                    }
                }
            }

            if command.command_id == command_id::GetBagScRsp {
                let bag: GetBagScRsp = command.parse_proto()?;

                for material in bag.material_list {
                    if book_ids.contains(&material.tid) {
                        books.push(material.tid);
                    }
                }
            }
        }

        if !achievements.is_empty() && !books.is_empty() {
            break;
        }
    }

    let file = File::create("export.json")?;

    serde_json::to_writer(
        BufWriter::new(file),
        &Jason {
            achievements,
            books,
        },
    )?;

    println!(
        "Exported successfully to {}",
        std::env::current_dir()?
            .join("export.json")
            .to_string_lossy()
    );

    std::io::stdin().read_line(&mut String::new())?;

    Ok(())
}

fn load_online_keys() -> Result<HashMap<u32, Vec<u8>>> {
    let keys: HashMap<u32, String> = ureq::get("https://stardb.gg/static/keys.json")
        .call()?
        .into_json()?;

    let mut keys_bytes = HashMap::new();

    for (k, v) in keys {
        keys_bytes.insert(k, BASE64_STANDARD.decode(v)?);
    }

    Ok(keys_bytes)
}

fn capture_device(device: Device, tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
    let mut capture = pcap::Capture::from_device(device)?
        .immediate_mode(true)
        .promisc(true)
        .timeout(0)
        .open()?;

    capture.filter("udp portrange 23301-23302", true).unwrap();

    println!("All ready~!");

    while let Ok(packet) = capture.next_packet() {
        tx.send(packet.data.to_vec())?;
    }

    Ok(())
}
