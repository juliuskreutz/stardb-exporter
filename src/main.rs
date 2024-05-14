use anyhow::{anyhow, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::Parser;
use pcap::{ConnectionStatus, Device};
use reliquary::network::{
    gen::{
        command_id,
        proto::{GetBagScRsp::GetBagScRsp, GetQuestDataScRsp::GetQuestDataScRsp},
    },
    GamePacket, GameSniffer,
};
use std::{collections::HashMap, io::Write, sync::mpsc};

/// The stardb exporter cli to export your achievements and books easily
#[derive(Parser, Clone, Copy)]
#[command(version)]
struct Args {
    /// Print verbose information
    #[arg(short, long)]
    verbose: bool,

    /// UwU what's this? ~murr~
    #[arg(short, long)]
    uwu: bool,
}

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
    let args = Args::parse();

    if args.verbose {
        println!(
            "{}",
            uwu("Getting achievement ids from stardb-api", args.uwu)
        )
    }
    let achievements: Vec<Id> = ureq::get("https://stardb.gg/api/achievements")
        .call()?
        .into_json()?;
    let achievement_ids: Vec<_> = achievements.into_iter().map(|a| a.id).collect();
    if args.verbose {
        println!(
            "{}",
            uwu(
                &format!("Got {} achievement ids", achievement_ids.len()),
                args.uwu
            )
        )
    }

    if args.verbose {
        println!("{}", uwu("Getting book ids from stardb-api", args.uwu))
    }
    let books: Vec<Id> = ureq::get("https://stardb.gg/api/books")
        .call()?
        .into_json()?;
    let book_ids: Vec<_> = books.into_iter().map(|a| a.id).collect();
    if args.verbose {
        println!(
            "{}",
            uwu(&format!("Got {} book ids", achievement_ids.len()), args.uwu)
        )
    }

    if args.verbose {
        println!("{}", uwu("Loading rsa packet decryption keys", args.uwu))
    }
    let keys = load_online_keys(args)?;

    if args.verbose {
        println!("{}", uwu("Finding pcap device", args.uwu))
    }
    let device = pcap::Device::list()?
        .into_iter()
        .filter(|d| d.flags.connection_status == ConnectionStatus::Connected)
        .filter(|d| !d.addresses.is_empty())
        .find(|d| !d.flags.is_loopback())
        .unwrap();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || capture_device(device, tx, args));

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

                if args.verbose {
                    println!("{}", uwu("Caught achievement packet", args.uwu))
                }

                for quest in quest_data.quest_list {
                    if achievement_ids.contains(&quest.id)
                        && (quest.status.value() == 2 || quest.status.value() == 3)
                    {
                        if args.verbose {
                            println!(
                                "{}",
                                uwu(
                                    &format!("Found completed achievement {}", quest.id),
                                    args.uwu
                                )
                            )
                        }

                        achievements.push(quest.id);
                    }
                }
            }

            if command.command_id == command_id::GetBagScRsp {
                if !books.is_empty() {
                    continue;
                }

                if args.verbose {
                    println!("{}", uwu("Caught book packet", args.uwu))
                }

                let bag: GetBagScRsp = command.parse_proto()?;

                for material in bag.material_list {
                    if book_ids.contains(&material.tid) {
                        if args.verbose {
                            println!(
                                "{}",
                                uwu(&format!("Found collected book {}", material.tid), args.uwu)
                            )
                        }

                        books.push(material.tid);
                    }
                }
            }
        }

        if !achievements.is_empty() && !books.is_empty() {
            break;
        }
    }

    let export = Export {
        achievements,
        books,
    };
    let json = serde_json::to_string(&export)?;

    clipboard_win::set_clipboard_string(&json).map_err(|_| anyhow!("Error setting clipboard"))?;

    println!(
        "{}",
        uwu(
            &format!(
                "Copied {} achievement and {} books to clipboard",
                export.achievements.len(),
                export.books.len()
            ),
            args.uwu
        )
    );
    println!("{}", uwu("Press return to exit...", args.uwu));

    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut String::new())?;

    Ok(())
}

fn load_online_keys(args: Args) -> Result<HashMap<u32, Vec<u8>>> {
    let keys: HashMap<u32, String> = ureq::get("https://stardb.gg/static/keys.json")
        .call()?
        .into_json()?;

    let mut keys_bytes = HashMap::new();

    for (k, v) in keys {
        if args.verbose {
            println!("{}", uwu(&format!("Version {k} Key {v}"), args.uwu));
        }
        keys_bytes.insert(k, BASE64_STANDARD.decode(v)?);
    }

    Ok(keys_bytes)
}

fn capture_device(device: Device, tx: mpsc::Sender<Vec<u8>>, args: Args) -> Result<()> {
    loop {
        let mut capture = pcap::Capture::from_device(device.clone())?
            .immediate_mode(true)
            .promisc(true)
            .timeout(0)
            .open()?;

        capture.filter("udp portrange 23301-23302", true).unwrap();

        println!("{}", uwu("All ready~!", args.uwu));

        while let Ok(packet) = capture.next_packet() {
            tx.send(packet.data.to_vec())?;
        }
    }
}

fn uwu(s: &str, b: bool) -> String {
    if b {
        uwuifier::uwuify_str_sse(s)
    } else {
        s.to_string()
    }
}
