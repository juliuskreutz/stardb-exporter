mod gi;
mod hsr;

use std::{sync::mpsc, thread};

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Game {
    #[default]
    Hsr,
    Gi,
}

impl Game {
    pub fn run(self) -> mpsc::Receiver<Option<String>> {
        let (log_tx, log_rx) = mpsc::channel();

        thread::spawn(move || -> anyhow::Result<()> {
            log_tx.send(Some("Getting achievements from api".to_string()))?;

            let achievement_ids = match self.achievement_ids() {
                Ok(achievement_ids) => achievement_ids,
                Err(e) => {
                    log_tx.send(Some(format!("Error: {e}")))?;
                    log_tx.send(None)?;
                    return Ok(());
                }
            };

            log_tx.send(Some(format!(
                "Got {} achievements from api",
                achievement_ids.len()
            )))?;

            let devices = match self.devices() {
                Ok(devices) => devices,
                Err(e) => {
                    log_tx.send(Some(format!("Error: {e}")))?;
                    log_tx.send(None)?;
                    return Ok(());
                }
            };

            log_tx.send(Some(format!("Found {} network devices", devices.len())))?;

            let mut join_handles = Vec::new();
            let (device_tx, device_rx) = mpsc::channel();
            for (i, device) in devices.into_iter().enumerate() {
                let device_tx = device_tx.clone();
                let log_tx = log_tx.clone();
                let handle =
                    std::thread::spawn(move || self.capture_device(i, device, &device_tx, &log_tx));
                join_handles.push(handle);
            }

            let achievements = match self {
                Game::Hsr => hsr::sniff(&achievement_ids, &device_rx),
                Game::Gi => gi::sniff(&achievement_ids, &device_rx),
            };
            let achievements = match achievements {
                Ok(achievements) => achievements,
                Err(e) => {
                    log_tx.send(Some(format!("Error: {e}")))?;
                    log_tx.send(None)?;
                    return Ok(());
                }
            };

            let json = match self {
                Game::Hsr => {
                    serde_json::to_string(&serde_json::json!({"hsr_achievements": achievements}))?
                }
                Game::Gi => {
                    serde_json::to_string(&serde_json::json!({"gi_achievements": achievements}))?
                }
            };

            let mut clipboard = match arboard::Clipboard::new() {
                Ok(clipboard) => clipboard,
                Err(e) => {
                    log_tx.send(Some(format!("Error: {e}")))?;
                    log_tx.send(None)?;
                    return Ok(());
                }
            };

            if let Err(e) = clipboard.set_text(json) {
                log_tx.send(Some(format!("Error: {e}")))?;
            } else {
                log_tx.send(Some(format!(
                    "Copied {} achievements to clipboard",
                    achievements.len()
                )))?;
            };

            log_tx.send(None)?;

            Ok(())
        });

        log_rx
    }

    fn achievement_ids(self) -> anyhow::Result<Vec<u32>> {
        #[derive(serde::Deserialize)]
        struct Achievement {
            id: u32,
        }

        let url = match self {
            Game::Hsr => "https://stardb.gg/api/achievements",
            Game::Gi => "https://stardb.gg/api/gi/achievements",
        };

        let achievements: Vec<Achievement> = ureq::get(url).call()?.into_json()?;
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
        log_tx: &mpsc::Sender<Option<String>>,
    ) -> anyhow::Result<()> {
        let packet_filer = match self {
            Game::Hsr => "udp portrange 23301-23302",
            Game::Gi => "udp portrange 22101-22102",
        };

        loop {
            let mut capture = pcap::Capture::from_device(device.clone())?
                .immediate_mode(true)
                .promisc(true)
                .timeout(0)
                .open()?;

            capture.filter(packet_filer, true)?;

            log_tx.send(Some(format!("Device {i} ready~!")))?;

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

            log_tx.send(Some(format!("Device {i} Error. Starting up again...")))?;
        }
    }
}
