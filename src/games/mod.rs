mod gi;
mod hsr;
mod zzz;

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use regex::Regex;

use crate::app::{Message, State};

#[derive(Clone, Copy, PartialEq)]
pub enum Game {
    Hsr,
    Gi,
    Zzz,
}

impl Game {
    pub fn achievements(self, message_tx: &mpsc::Sender<Message>) {
        let message_tx = message_tx.clone();

        thread::spawn(move || {
            let achievement_ids = match self.achievement_ids() {
                Ok(achievement_ids) => achievement_ids,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            let devices = match self.devices() {
                Ok(devices) => devices,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            let (device_tx, device_rx) = mpsc::channel();
            for (i, device) in devices.into_iter().enumerate() {
                let device_tx = device_tx.clone();
                let message_tx = message_tx.clone();
                std::thread::spawn(move || self.capture_device(i, device, &device_tx, &message_tx));
            }

            let achievements = match self {
                Game::Hsr => hsr::sniff(&achievement_ids, &device_rx),
                Game::Gi => gi::sniff(&achievement_ids, &device_rx),
                _ => unimplemented!(),
            };
            let achievements = match achievements {
                Ok(achievements) => achievements,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            message_tx
                .send(Message::GoTo(State::Achievements(achievements)))
                .unwrap();
        });
    }

    pub fn game_path(self) -> anyhow::Result<PathBuf> {
        match self {
            Game::Hsr => hsr::game_path(),
            Game::Gi => gi::game_path(),
            Game::Zzz => zzz::game_path(),
        }
    }

    pub fn achievement_url(self) -> String {
        let prefix = match self {
            Game::Hsr => "",
            Game::Gi => "genshin",
            Game::Zzz => "zzz",
        };

        format!("https://stardb.gg/{prefix}/achievement-tracker")
    }

    pub fn pull_url(self) -> String {
        let path = match self {
            Game::Hsr => "warp-tracker",
            Game::Gi => "genshin/wish-tracker",
            Game::Zzz => "zzz/signal-tracker",
        };

        format!("https://stardb.gg/{path}")
    }

    fn achievement_ids(self) -> anyhow::Result<Vec<u32>> {
        #[derive(serde::Deserialize)]
        struct Achievement {
            id: u32,
        }

        let url = match self {
            Game::Hsr => "https://stardb.gg/api/achievements",
            Game::Gi => "https://stardb.gg/api/gi/achievements",
            _ => unimplemented!(),
        };

        let achievements: Vec<Achievement> = ureq::get(url).call()?.body_mut().read_json()?;
        let achievement_ids: Vec<_> = achievements.into_iter().map(|a| a.id).collect();

        Ok(achievement_ids)
    }

    fn devices(self) -> anyhow::Result<Vec<pcap::Device>> {
        Ok(pcap::Device::list()?
            .into_iter()
            .filter(|d| d.flags.connection_status == pcap::ConnectionStatus::Connected)
            .filter(|d| !d.addresses.is_empty())
            .filter(|d| !d.flags.is_loopback())
            .collect())
    }

    fn capture_device(
        self,
        i: usize,
        device: pcap::Device,
        device_tx: &mpsc::Sender<Vec<u8>>,
        message_tx: &mpsc::Sender<Message>,
    ) -> anyhow::Result<()> {
        let packet_filer = match self {
            Game::Hsr => "udp portrange 23301-23302",
            Game::Gi => "udp portrange 22101-22102",
            _ => unimplemented!(),
        };

        loop {
            let mut capture = pcap::Capture::from_device(device.clone())?
                .immediate_mode(true)
                .promisc(true)
                .timeout(0)
                .open()?;

            capture.filter(packet_filer, true)?;

            message_tx
                .send(Message::Toast({
                    let mut toast = egui_notify::Toast::success(format!("Device {i} Ready~!"));
                    toast.duration(None);
                    toast
                }))
                .unwrap();

            message_tx
                .send(Message::GoTo(State::Waiting("Running".to_string())))
                .unwrap();

            let mut has_captured = false;

            loop {
                match capture.next_packet() {
                    Ok(packet) => {
                        device_tx.send(packet.data.to_vec())?;
                        has_captured = true;
                    }
                    Err(_) if !has_captured => break,
                    Err(pcap::Error::TimeoutExpired) => continue,
                    Err(e) => return Err(anyhow::anyhow!("{e}")),
                }
            }

            message_tx
                .send(Message::Toast({
                    let mut toast = egui_notify::Toast::error(format!(
                        "Device {i} Error. Starting up again..."
                    ));
                    toast.duration(None);
                    toast
                }))
                .unwrap();
        }
    }
}

pub fn pulls_from_game_path(path: &Path) -> anyhow::Result<String> {
    let mut path = path.to_path_buf();

    path.push("webCaches");

    let re = Regex::new(r"^\d+\.\d+\.\d+\.\d+$")?;
    let mut paths: Vec<_> = path
        .read_dir()?
        .flat_map(|r| r.ok().map(|d| d.path()))
        .filter(|p| re.is_match(p.file_name().and_then(|o| o.to_str()).unwrap_or_default()))
        .collect();
    paths.sort();

    let mut cache_path = paths[paths.len() - 1].clone();
    cache_path.push("Cache");
    cache_path.push("Cache_Data");
    cache_path.push("data_2");

    let bytes = std::fs::read(cache_path)?;
    let data = String::from_utf8_lossy(&bytes);
    let lines: Vec<_> = data.split("1/0/").collect();

    for line in lines.iter().rev() {
        if line.starts_with("https://") && line.contains("getGachaLog") {
            if let Some(url) = line.split('\0').next() {
                if ureq::get(url)
                    .call()
                    .ok()
                    .and_then(|mut r| r.body_mut().read_json::<serde_json::Value>().ok())
                    .map(|j| j["retcode"] == 0)
                    .unwrap_or_default()
                {
                    return Ok(url.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("Couldn't find pull url"))
}
