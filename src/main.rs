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
use std::{collections::HashMap, io::Write, panic::catch_unwind, sync::mpsc};

#[derive(serde::Deserialize)]
struct Id {
    id: u32,
}

#[derive(serde::Serialize)]
struct Export {
    achievements: Vec<u32>,
    books: Vec<u32>,
}

fn main() -> Result<()> {
    if let Err(e) = catch_unwind(|| export()) {
        println!("{e:?}")
    }

    println!("Press return to exit...");

    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut String::new())?;

    Ok(())
}

fn export() -> Result<()> {
    let achievements: Vec<Id> = ureq::get("https://stardb.gg/api/achievements")
        .call()?
        .into_json()?;
    let achievement_ids: Vec<_> = achievements.into_iter().map(|a| a.id).collect();

    let books: Vec<Id> = ureq::get("https://stardb.gg/api/books")
        .call()?
        .into_json()?;
    let book_ids: Vec<_> = books.into_iter().map(|a| a.id).collect();

    let keys = load_online_keys()?;

    let mut join_handles = Vec::new();
    let (tx, rx) = mpsc::channel();
    for device in Device::list()
        .unwrap()
        .into_iter()
        .filter(|d| d.flags.connection_status == ConnectionStatus::Connected)
        .filter(|d| !d.addresses.is_empty())
        .filter(|d| !d.flags.is_loopback())
    {
        let tx = tx.clone();
        let handle = std::thread::spawn(move || capture_device(device, tx));
        join_handles.push(handle);
    }
    drop(tx);

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

            if command.command_id == command_id::GetBagScRsp {
                if !books.is_empty() {
                    continue;
                }

                println!("Got books packet");

                if let Ok(bag) = command.parse_proto::<GetBagScRsp>() {
                    for material in bag.material_list {
                        if book_ids.contains(&material.tid) {
                            books.push(material.tid);
                        }
                    }
                }
            }
        }

        if !achievements.is_empty() && !books.is_empty() {
            break;
        }
    }

    if achievements.is_empty() && books.is_empty() {
        return Err(anyhow::anyhow!("No achievements or books found"));
    }

    println!("Copying to clipboard");

    let export = Export {
        achievements,
        books,
    };
    let json = serde_json::to_string(&export)?;

    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(json)?;

    println!(
        "Copied {} achievements and {} books to clipboard",
        export.achievements.len(),
        export.books.len()
    );

    Ok(())
}

fn load_online_keys() -> Result<HashMap<u32, Vec<u8>>> {
    let keys: HashMap<u32, String> = ureq::get(
        "https://raw.githubusercontent.com/juliuskreutz/stardb-exporter/master/keys.json",
    )
    .call()?
    .into_json()?;

    let mut keys_bytes = HashMap::new();

    for (k, v) in keys {
        keys_bytes.insert(k, BASE64_STANDARD.decode(v)?);
    }

    Ok(keys_bytes)
}

fn capture_device(device: Device, tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
    loop {
        let mut capture = pcap::Capture::from_device(device.clone())?
            .immediate_mode(true)
            .promisc(true)
            .timeout(0)
            .open()?;

        capture.filter("udp portrange 23301-23302", true).unwrap();

        println!("All ready~!");

        let mut has_captured = false;

        loop {
            match capture.next_packet() {
                Ok(packet) => {
                    tx.send(packet.data.to_vec())?;
                    has_captured = true;
                }
                Err(_) if !has_captured => break,
                Err(pcap::Error::TimeoutExpired) => continue,
                Err(e) => return Err(anyhow::anyhow!("{e}")),
            }
        }

        println!("Error. Starting up again...");
    }
}
