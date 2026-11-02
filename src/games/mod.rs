mod error;
mod gi;
mod hsr;
mod zzz;

use std::{
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(not(any(feature = "pktmon", feature = "pcap")))]
compile_error!("at least one of the features \"pktmon\" or \"pcap\" must be enabled");
#[cfg(all(feature = "pktmon", feature = "pcap"))]
compile_error!("at most one of the features \"pktmon\" or \"pcap\" must be enabled");

pub use self::error::{ErrorReport, ExportError};
use crate::app::{Message, State};
use crate::pcapng::{CaptureFile, CaptureLog, CaptureSummary, read_capture};
use regex::Regex;

/// How long the capture threads and the sniffer wait on a packet before checking
/// whether they should stop.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// How long a live run stays quiet about something looking wrong. The exporter is
/// started before the game, so waiting for a while is normal.
const WARNING_DELAY: Duration = Duration::from_secs(60);

/// How many times in a row a capture may fail right after opening before giving
/// up on it.
const MAX_CAPTURE_RESTARTS: u32 = 5;

#[derive(Clone, Copy, PartialEq)]
pub enum Game {
    Hsr,
    Gi,
    Zzz,
}

/// The result of a successful run.
pub struct Achievements {
    pub ids: Vec<u32>,
    pub breakdown: Option<Breakdown>,
    /// Set when these came from loading a capture rather than a live run.
    pub capture_report: Option<String>,
}

/// How many achievements each of ZZZ's two lists had finished, if it came by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Breakdown {
    pub regular: Option<usize>,
    pub arcade: Option<usize>,
}

impl Breakdown {
    /// One of the counts, keeping a list that never came by apart from an empty one.
    pub fn describe(count: Option<usize>) -> String {
        count.map_or_else(|| "not received".to_string(), |count| count.to_string())
    }
}

pub struct Packet {
    pub data: Vec<u8>,
    pub timestamp_ms: u64,
}

/// The way to stop a run from the outside.
#[derive(Clone, Default)]
pub struct RunHandle(Arc<AtomicBool>);

impl RunHandle {
    pub fn stop(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum KeyStage {
    #[default]
    None,
    Dispatch,
    Session,
}

/// How far a run got.
#[derive(Default)]
pub struct Progress {
    pub packets: usize,
    pub handshakes: usize,
    pub disconnects: usize,
    pub key_stage: KeyStage,
}

pub enum Received {
    Packet(Packet),
    /// The deadline passed.
    Deadline,
    /// The run was stopped.
    Stopped,
    /// No more packets are coming.
    Ended,
}

/// The packets of a run, and what can be told from them about how it is going.
pub struct PacketStream<'a> {
    packets: mpsc::Receiver<Packet>,
    run: &'a RunHandle,
    /// Where to warn the user about a live run. A capture file has nobody to warn.
    warnings: Option<&'a mpsc::Sender<Message>>,
    started: Instant,
    session_since: Option<Instant>,
    warning: Option<&'static str>,
    pub progress: Progress,
}

impl<'a> PacketStream<'a> {
    fn new(
        packets: mpsc::Receiver<Packet>,
        run: &'a RunHandle,
        warnings: Option<&'a mpsc::Sender<Message>>,
    ) -> Self {
        Self {
            packets,
            run,
            warnings,
            started: Instant::now(),
            session_since: None,
            warning: None,
            progress: Progress::default(),
        }
    }

    /// The next packet. A deadline only applies to live runs, since it's meant in
    /// the time the packets took to arrive and a capture file has them all at once.
    pub fn next(&mut self, deadline: Option<Instant>) -> Received {
        let deadline = deadline.filter(|_| self.warnings.is_some());

        loop {
            if self.run.is_stopped() {
                return Received::Stopped;
            }
            self.update_warning();

            let mut timeout = POLL_INTERVAL;
            if let Some(deadline) = deadline {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return Received::Deadline;
                }
                timeout = timeout.min(left);
            }

            match self.packets.recv_timeout(timeout) {
                Ok(packet) => {
                    self.progress.packets += 1;
                    return Received::Packet(packet);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                // Stopping ends the captures too, and that can get here before the
                // check above comes around again.
                Err(mpsc::RecvTimeoutError::Disconnected) if self.run.is_stopped() => {
                    return Received::Stopped;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Received::Ended,
            }
        }
    }

    pub fn note_key(&mut self, stage: KeyStage) {
        self.progress.key_stage = stage;
        if stage == KeyStage::Session {
            self.session_since.get_or_insert_with(Instant::now);
        }
    }

    fn update_warning(&mut self) {
        let Some(message_tx) = self.warnings else {
            return;
        };

        let warning = if self.started.elapsed() < WARNING_DELAY {
            None
        } else if self.progress.packets == 0 {
            Some(
                "No game traffic so far. Has the game been started? Disable any VPN. A game running on another device or through cloud gaming can't be read.",
            )
        } else if self.progress.handshakes == 0 && self.progress.key_stage == KeyStage::None {
            // A capture that starts mid-connection never gets to decrypt anything,
            // so this is the only sign of it. Restarting the game fixes it without
            // restarting the run.
            Some(
                "The game is running but its login wasn't seen. Close the game completely and start it again, the exporter can keep running.",
            )
        } else if self
            .session_since
            .is_some_and(|since| since.elapsed() >= WARNING_DELAY)
        {
            Some(
                "Decrypting the game's traffic works, waiting for the achievements. Log in fully and get into the game.",
            )
        } else {
            None
        };

        if warning != self.warning {
            self.warning = warning;
            message_tx
                .send(Message::RunWarning(warning.map(str::to_string)))
                .unwrap();
        }
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "Unknown panic".to_string())
}

impl Game {
    /// Start reading achievements from the network. The run ends with a
    /// [`Message::RunEnded`], after everything it started has stopped.
    pub fn achievements(self, message_tx: &mpsc::Sender<Message>, run: RunHandle) {
        let message_tx = message_tx.clone();

        thread::spawn(move || {
            let outcome = self.run_live(&message_tx, &run);
            let state = self.final_state(outcome, None);
            message_tx.send(Message::RunEnded(state)).unwrap();
        });
    }

    /// Start reading achievements from a capture file, the same way as from the
    /// network, also ending with a [`Message::RunEnded`].
    pub fn replay(self, path: PathBuf, message_tx: &mpsc::Sender<Message>, run: RunHandle) {
        let message_tx = message_tx.clone();

        thread::spawn(move || {
            let (outcome, report) = self.run_capture_file(&path, &run);
            let state = self.final_state(outcome, Some(report));
            message_tx.send(Message::RunEnded(state)).unwrap();
        });
    }

    fn final_state(
        self,
        outcome: Result<Achievements, ExportError>,
        capture_report: Option<String>,
    ) -> State {
        match outcome {
            Ok(achievements) => State::Achievements(Achievements {
                capture_report,
                ..achievements
            }),
            Err(ExportError::Cancelled) => State::Game,
            Err(e) => {
                tracing::error!(game = self.name(), error = ?e, "Run failed");
                let is_capture = capture_report.is_some();
                let mut report = e.report(self);
                report.capture_report = capture_report;
                // Whoever loads a capture is who the ticket would go to, and there
                // is no game of theirs to restart.
                if is_capture {
                    report.support = false;
                    report.steps.clear();
                    if e == ExportError::CaptureEnded {
                        report.title = "The capture ended before any achievements".to_string();
                    }
                }
                State::RunError(report)
            }
        }
    }

    fn run_live(
        self,
        message_tx: &mpsc::Sender<Message>,
        run: &RunHandle,
    ) -> Result<Achievements, ExportError> {
        let achievement_ids = self
            .achievement_ids()
            .map_err(|e| ExportError::AchievementIds(e.to_string()))?;

        if run.is_stopped() {
            return Err(ExportError::Cancelled);
        }

        let (device_tx, device_rx) = mpsc::channel();
        let captures = self.start_captures(device_tx, message_tx, run)?;

        let mut stream = PacketStream::new(device_rx, run, Some(message_tx));
        let outcome = self.sniff_guarded(&achievement_ids, &mut stream);
        drop(stream);

        run.stop();

        let mut errors = Vec::new();
        for capture in captures {
            match capture.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(e.to_string()),
                Err(payload) => errors.push(panic_message(&*payload)),
            }
        }

        match outcome {
            Err(ExportError::CaptureEnded) => Err(ExportError::CaptureFailed(errors.join("; "))),
            outcome => outcome,
        }
    }

    #[cfg(feature = "pcap")]
    fn start_captures(
        self,
        device_tx: mpsc::Sender<Packet>,
        message_tx: &mpsc::Sender<Message>,
        run: &RunHandle,
    ) -> Result<Vec<thread::JoinHandle<anyhow::Result<()>>>, ExportError> {
        let devices = self
            .devices()
            .map_err(|e| ExportError::CaptureFailed(e.to_string()))?;
        if devices.is_empty() {
            return Err(ExportError::NoDevices);
        }

        let log = CaptureLog::new(self.capture_file());

        Ok(devices
            .into_iter()
            .enumerate()
            .map(|(i, device)| {
                let device_tx = device_tx.clone();
                let message_tx = message_tx.clone();
                let run = run.clone();
                let log = log.clone();
                thread::spawn(move || {
                    let result =
                        self.capture_device_pcap(i, device, &device_tx, &message_tx, &run, &log);
                    if let Err(e) = &result {
                        tracing::error!("Device {i} failed: {e}");
                        if !run.is_stopped() {
                            message_tx
                                .send(Message::Toast(egui_notify::Toast::error(format!(
                                    "Device {i} failed: {e}"
                                ))))
                                .unwrap();
                        }
                    }
                    result
                })
            })
            .collect())
    }

    #[cfg(feature = "pktmon")]
    fn start_captures(
        self,
        device_tx: mpsc::Sender<Packet>,
        message_tx: &mpsc::Sender<Message>,
        run: &RunHandle,
    ) -> Result<Vec<thread::JoinHandle<anyhow::Result<()>>>, ExportError> {
        let message_tx = message_tx.clone();
        let run = run.clone();
        let log = CaptureLog::new(self.capture_file());

        Ok(vec![thread::spawn(move || {
            let result = self.capture_device_pktmon(&device_tx, &message_tx, &run, &log);
            if let Err(e) = &result {
                tracing::error!("Capture failed: {e}");
            }
            result
        })])
    }

    fn run_capture_file(
        self,
        path: &Path,
        run: &RunHandle,
    ) -> (Result<Achievements, ExportError>, String) {
        let name = path.display().to_string();

        let capture = std::fs::read(path)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| read_capture(&bytes));
        let capture = match capture {
            Ok(capture) => capture,
            Err(e) => {
                let report = format!(
                    "Capture: {name}\nGame: {}\nCould not read it: {e}",
                    self.name()
                );
                return (Err(ExportError::CaptureUnreadable(e.to_string())), report);
            }
        };

        let summary = CaptureSummary::new(&capture, self.ports());
        let truncated = capture.truncated;
        let unsupported: Vec<_> = capture
            .unsupported_links
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        let total = capture.packets.len();

        let (outcome, progress) = match self.achievement_ids() {
            Ok(achievement_ids) => self.replay_packets(&achievement_ids, capture, run),
            Err(e) => (
                Err(ExportError::AchievementIds(e.to_string())),
                Progress::default(),
            ),
        };

        let report = self.capture_report(
            &name,
            total,
            truncated,
            &unsupported,
            &summary,
            &progress,
            &outcome,
        );
        tracing::info!("Capture report:\n{report}");
        (outcome, report)
    }

    /// Run a capture's packets through the sniffer, the only difference with a live
    /// run being where they come from.
    fn replay_packets(
        self,
        achievement_ids: &[(u32, bool)],
        capture: CaptureFile,
        run: &RunHandle,
    ) -> (Result<Achievements, ExportError>, Progress) {
        let (packet_tx, packet_rx) = mpsc::channel();
        for packet in capture.packets {
            let _ = packet_tx.send(Packet {
                data: packet.data,
                timestamp_ms: packet.timestamp_ms,
            });
        }
        drop(packet_tx);

        let mut stream = PacketStream::new(packet_rx, run, None);
        let outcome = self.sniff_guarded(achievement_ids, &mut stream);
        (outcome, stream.progress)
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_report(
        self,
        name: &str,
        total: usize,
        truncated: bool,
        unsupported: &[(u16, usize)],
        summary: &CaptureSummary,
        progress: &Progress,
        outcome: &Result<Achievements, ExportError>,
    ) -> String {
        let (low, high) = self.ports();
        let seconds = summary
            .last_timestamp_ms
            .saturating_sub(summary.first_timestamp_ms) as f64
            / 1000.0;

        let mut lines = vec![
            format!("Capture: {name}"),
            format!("Game: {} (ports {low}-{high})", self.name()),
            format!("Packets: {total} over {seconds:.1}s"),
            format!(
                "Ethernet: {} received, {} sent",
                summary.ethernet_received, summary.ethernet_sent
            ),
            format!(
                "802.11 (ignored): {} received, {} sent",
                summary.wifi_received, summary.wifi_sent
            ),
            format!("Other ports: {:?}", summary.other_ports),
            format!("Not UDP: {}", summary.unreadable),
            format!(
                "Handshakes: {}, disconnects: {}",
                progress.handshakes, progress.disconnects
            ),
            format!(
                "Key reached: {}",
                match progress.key_stage {
                    KeyStage::None => "none",
                    KeyStage::Dispatch => "dispatch",
                    KeyStage::Session => "session",
                }
            ),
            format!("Packets read: {} of {total}", progress.packets),
        ];

        match outcome {
            Ok(achievements) => {
                lines.push(format!("Result: {} achievements", achievements.ids.len()));
                if let Some(breakdown) = achievements.breakdown {
                    lines.push(format!(
                        "Normal: {}, arcade: {}",
                        Breakdown::describe(breakdown.regular),
                        Breakdown::describe(breakdown.arcade)
                    ));
                }
            }
            Err(e) => lines.push(format!("Result: {e:?}")),
        }

        if truncated {
            lines.push("Warning: the file is cut off, its last packet is missing.".to_string());
        }
        if !unsupported.is_empty() {
            lines.push(format!(
                "Warning: packets skipped because of their link type (type, count): {unsupported:?}"
            ));
        }
        if !summary.has_game_traffic() {
            lines.push(format!(
                "Warning: nothing here is traffic of this game. Is this a capture of another one? Ports seen: {:?}",
                summary.other_ports.keys().collect::<Vec<_>>()
            ));
        } else {
            if summary.wifi_received > 0 && summary.ethernet_received == 0 {
                lines.push(
                    "Warning: received packets only exist as 802.11, which the sniffer can't read."
                        .to_string(),
                );
            } else if summary.ethernet_received == 0 && summary.wifi_received == 0 {
                lines.push("Warning: nothing was received from the server.".to_string());
            } else if summary.ethernet_sent == 0 && summary.wifi_sent == 0 {
                lines.push("Warning: nothing was sent to the server.".to_string());
            }
            if progress.handshakes == 0 {
                lines.push(
                    "Warning: no handshake, the capture started after the game connected."
                        .to_string(),
                );
            }
        }

        lines.join("\n")
    }

    /// A panic would otherwise leave the run waiting forever, with no way out of it.
    fn sniff_guarded(
        self,
        achievement_ids: &[(u32, bool)],
        stream: &mut PacketStream,
    ) -> Result<Achievements, ExportError> {
        panic::catch_unwind(AssertUnwindSafe(|| match self {
            Game::Hsr => hsr::sniff(achievement_ids, stream),
            Game::Gi => gi::sniff(achievement_ids, stream),
            Game::Zzz => zzz::sniff(achievement_ids, stream),
        }))
        .unwrap_or_else(|payload| Err(ExportError::Internal(panic_message(&*payload))))
    }

    pub fn name(self) -> &'static str {
        match self {
            Game::Hsr => "Honkai Star Rail",
            Game::Gi => "Genshin Impact",
            Game::Zzz => "Zenless Zone Zero",
        }
    }

    /// Where the packets of this game's latest run get logged, in the log folder.
    pub fn capture_file(self) -> &'static str {
        match self {
            Game::Hsr => "latest-hsr.pcapng",
            Game::Gi => "latest-gi.pcapng",
            Game::Zzz => "latest-zzz.pcapng",
        }
    }

    fn ports(self) -> (u16, u16) {
        match self {
            Game::Hsr => (23301, 23302),
            Game::Gi => (22101, 22102),
            Game::Zzz => (20501, 20502),
        }
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

    /// The achievement ids, with the arcade ones flagged.
    ///
    /// ZZZ detection matches the ids on the wire against this set and needs
    /// the arcade subset to tell its two achievement packets apart. The other
    /// games have one packet and leave the flag unused.
    fn achievement_ids(self) -> anyhow::Result<Vec<(u32, bool)>> {
        #[derive(serde::Deserialize)]
        struct Achievement {
            id: u32,
            #[serde(default)]
            arcade: bool,
        }

        let path = match self {
            Game::Hsr => "/api/achievements",
            Game::Gi => "/api/gi/achievements",
            Game::Zzz => "/api/zzz/achievements",
        };

        let url = format!("https://stardb.gg{path}");
        let backup_url =
            format!("https://raw.githubusercontent.com/hashblen/stardb-mirror/main{path}");

        // Stopping a run has to wait for this, so it can't take forever.
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .into();

        let response = agent.get(url).call();
        let achievements: Vec<Achievement> = match response {
            Ok(resp) if resp.status().is_success() => resp.into_body().read_json()?,
            _ => {
                tracing::info!("Fetching backup {backup_url}");
                let backup_response = agent.get(backup_url).call()?;
                backup_response.into_body().read_json()?
            }
        };
        let achievement_ids = achievements.into_iter().map(|a| (a.id, a.arcade)).collect();

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
        device_tx: &mpsc::Sender<Packet>,
        message_tx: &mpsc::Sender<Message>,
        run: &RunHandle,
        log: &CaptureLog,
    ) -> anyhow::Result<()> {
        let packet_filer = match self {
            Game::Hsr => "udp portrange 23301-23302",
            Game::Gi => "udp portrange 22101-22102",
            Game::Zzz => "udp portrange 20501-20502",
        };

        tracing::info!("Running exporter with pcap...");
        tracing::debug!("Finding devices...");

        let mut restarts = 0;

        while !run.is_stopped() {
            let mut capture = pcap::Capture::from_device(device.clone())?
                .immediate_mode(true)
                .promisc(true)
                .buffer_size(1024 * 1024 * 16) // 16MB
                .timeout(POLL_INTERVAL.as_millis() as i32)
                .open()?;

            capture.filter(packet_filer, true)?;

            if run.is_stopped() {
                break;
            }

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
                if run.is_stopped() {
                    return Ok(());
                }

                match capture.next_packet() {
                    Ok(packet) => {
                        let ts = (packet.header.ts.tv_sec as u64 * 1_000_000_000)
                            + (packet.header.ts.tv_usec as u64 * 1000);
                        let sent = device_tx.send(Packet {
                            data: packet.data.to_vec(),
                            timestamp_ms: ts / 1_000_000,
                        });
                        if sent.is_err() {
                            return Ok(());
                        }
                        log.write(ts, packet.data);
                        has_captured = true;
                    }
                    Err(pcap::Error::TimeoutExpired) => continue,
                    Err(_) if !has_captured => break,
                    Err(e) => return Err(anyhow::anyhow!("{e}")),
                }
            }

            restarts += 1;
            if restarts >= MAX_CAPTURE_RESTARTS {
                anyhow::bail!("Device {i} keeps failing right after opening");
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

            thread::sleep(Duration::from_secs(1));
        }

        Ok(())
    }

    #[cfg(feature = "pktmon")]
    fn capture_device_pktmon(
        self,
        device_tx: &mpsc::Sender<Packet>,
        message_tx: &mpsc::Sender<Message>,
        run: &RunHandle,
        log: &CaptureLog,
    ) -> anyhow::Result<()> {
        let port_range = self.ports();
        // let packet_filter = format!("udp portrange {}-{}", port_range.0, port_range.1);

        tracing::info!("Running exporter with pktmon...");

        let mut restarts = 0;

        while !run.is_stopped() {
            let mut capture = pktmon::Capture::new()?;

            for port in [port_range.0, port_range.1] {
                capture.add_filter(pktmon::filter::PktMonFilter {
                    name: "UDP Filter".to_string(),
                    transport_protocol: Some(pktmon::filter::TransportProtocol::UDP),
                    port: port.into(),
                    ..pktmon::filter::PktMonFilter::default()
                })?;
            }

            if run.is_stopped() {
                break;
            }

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
            capture.start()?;

            loop {
                if run.is_stopped() {
                    capture.stop()?;
                    return Ok(());
                }

                match capture.next_packet_timeout(POLL_INTERVAL) {
                    Ok(packet) => {
                        let payload = packet.payload.to_vec().clone();
                        let timestamp_ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as u64;
                        log.write(timestamp_ns, &payload);
                        let sent = device_tx.send(Packet {
                            data: payload,
                            timestamp_ms: timestamp_ns / 1_000_000,
                        });
                        if sent.is_err() {
                            capture.stop()?;
                            return Ok(());
                        }
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

            restarts += 1;
            if restarts >= MAX_CAPTURE_RESTARTS {
                anyhow::bail!("The capture keeps failing right after opening");
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

            thread::sleep(Duration::from_secs(1));
        }

        Ok(())
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

#[cfg(test)]
mod tests;
