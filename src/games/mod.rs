mod gi;
mod hsr;
mod zzz;

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

#[cfg(not(any(feature = "pktmon", feature = "pcap")))]
compile_error!("at least one of the features \"pktmon\" or \"pcap\" must be enabled");
#[cfg(all(feature = "pktmon", feature = "pcap"))]
compile_error!("at most one of the features \"pktmon\" or \"pcap\" must be enabled");

use crate::app::{Message, State};
use regex::Regex;
use tracing::info;

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

            let (device_tx, device_rx) = mpsc::channel();
            #[cfg(feature = "pcap")]
            {
                let devices = match self.devices() {
                    Ok(devices) => devices,
                    Err(e) => {
                        message_tx
                            .send(Message::GoTo(State::Error(e.to_string())))
                            .unwrap();
                        return;
                    }
                };

                for (i, device) in devices.into_iter().enumerate() {
                    let device_tx = device_tx.clone();
                    let message_tx = message_tx.clone();
                    std::thread::spawn(move || {
                        self.capture_device_pcap(i, device, &device_tx, &message_tx)
                    });
                }
            }
            #[cfg(feature = "pktmon")]
            {
                let device_tx = device_tx.clone();
                let message_tx = message_tx.clone();
                std::thread::spawn(move || self.capture_device_pktmon(&device_tx, &message_tx));
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
            Game::Gi => "genshin/",
            Game::Zzz => "zzz/",
        };

        format!("https://stardb.gg/{prefix}achievement-tracker")
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

        let path = match self {
            Game::Hsr => "/api/achievements",
            Game::Gi => "/api/gi/achievements",
            _ => unimplemented!(),
        };

        let url = format!("https://stardb.gg{path}");
        let backup_url =
            format!("https://raw.githubusercontent.com/hashblen/stardb-mirror/main{path}");

        let response = ureq::get(url).call();
        let achievements: Vec<Achievement> = match response {
            Ok(resp) if resp.status().is_success() => resp.into_body().read_json()?,
            _ => {
                info!("Fetching backup {backup_url}");
                let backup_response = ureq::get(backup_url).call()?;
                info!("Got status code {}", backup_response.status());
                assert!(
                    backup_response.status().is_success()
                        || backup_response.status().is_redirection(),
                    "status code {}",
                    backup_response.status()
                );
                backup_response.into_body().read_json()?
            }
        };
        let achievement_ids: Vec<_> = achievements.into_iter().map(|a| a.id).collect();

        Ok(achievement_ids)
    }

    #[cfg(feature = "pcap")]
    fn devices(self) -> anyhow::Result<Vec<pcap::Device>> {
        Ok(pcap::Device::list()?
            .into_iter()
            .filter(|d| d.flags.connection_status == pcap::ConnectionStatus::Connected)
            .filter(|d| !d.addresses.is_empty())
            .filter(|d| !d.flags.is_loopback())
            .collect())
    }

    #[cfg(feature = "pcap")]
    fn capture_device_pcap(
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

        tracing::info!("Running exporter with pcap...");
        tracing::debug!("Finding devices...");

        loop {
            let mut capture = pcap::Capture::from_device(device.clone())?
                .immediate_mode(true)
                .promisc(true)
                .buffer_size(1024 * 1024 * 16) // 16MB
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
            tracing::info!("Device {i} Ready~!");

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
            tracing::info!("Device {i} Error. Starting up again...");
        }
    }

    #[cfg(feature = "pktmon")]
    fn capture_device_pktmon(
        self,
        device_tx: &mpsc::Sender<Vec<u8>>,
        message_tx: &mpsc::Sender<Message>,
    ) -> anyhow::Result<()> {
        let port_range = match self {
            Game::Hsr => (23301, 23302),
            Game::Gi => (22101, 22102),
            _ => unimplemented!(),
        };
        // let packet_filter = format!("udp portrange {}-{}", port_range.0, port_range.1);

        tracing::info!("Running exporter with pktmon...");

        loop {
            let mut capture = pktmon::Capture::new()?;

            vec![port_range.0, port_range.1]
                .into_iter()
                .map(|port| pktmon::filter::PktMonFilter {
                    name: "UDP Filter".to_string(),
                    transport_protocol: Some(pktmon::filter::TransportProtocol::UDP),
                    port: port.into(),
                    ..pktmon::filter::PktMonFilter::default()
                })
                .for_each(|filter| {
                    capture.add_filter(filter).unwrap();
                });

            message_tx
                .send(Message::Toast({
                    let mut toast = egui_notify::Toast::success("Capture Ready~!".to_string());
                    toast.duration(None);
                    toast
                }))
                .unwrap();

            message_tx
                .send(Message::GoTo(State::Waiting("Running".to_string())))
                .unwrap();
            tracing::info!("Capture Ready~!");

            let mut has_captured = false;
            capture.start().unwrap();

            loop {
                match capture.next_packet_timeout(std::time::Duration::from_secs(1)) {
                    Ok(packet) => {
                        device_tx.send(packet.payload.to_vec().clone())?;
                        has_captured = true;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        has_captured = true;
                        continue;
                    }
                    Err(_) if !has_captured => break,
                    Err(e) => return Err(anyhow::anyhow!("{e}")),
                }
            }

            message_tx
                .send(Message::Toast({
                    let mut toast = egui_notify::Toast::error(
                        "Capture Error. Starting up again...".to_string(),
                    );
                    toast.duration(None);
                    toast
                }))
                .unwrap();
            tracing::info!("Capture Error. Starting up again...");
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
        if line.starts_with("https://")
            && (line.contains("getGachaLog") || line.contains("getLdGachaLog"))
            && let Some(url) = line.split('\0').next()
            && ureq::get(url)
                .call()
                .ok()
                .and_then(|mut r| r.body_mut().read_json::<serde_json::Value>().ok())
                .map(|j| j["retcode"] == 0)
                .unwrap_or_default()
        {
            return Ok(url.to_string());
        }
    }

    Err(anyhow::anyhow!("Couldn't find pull url"))
}
