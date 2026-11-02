use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct PcapngWriter {
    file: File,
    packet_count: u32,
}

impl PcapngWriter {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let mut writer = PcapngWriter {
            file,
            packet_count: 0,
        };
        writer.write_shb()?;
        writer.write_idb()?;
        Ok(writer)
    }

    fn write_shb(&mut self) -> std::io::Result<()> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x0a0d0d0a_u32.to_le_bytes());

        let total_len: u32 = 32;
        block.extend_from_slice(&total_len.to_le_bytes());

        block.extend_from_slice(&0x1a2b3c4d_u32.to_le_bytes());
        block.extend_from_slice(&1_u16.to_le_bytes());
        block.extend_from_slice(&0_u16.to_le_bytes());
        block.extend_from_slice(&0xffffffffffffffff_u64.to_le_bytes());

        block.extend_from_slice(&0_u32.to_le_bytes());

        block.extend_from_slice(&total_len.to_le_bytes());

        self.file.write_all(&block)
    }

    fn write_idb(&mut self) -> std::io::Result<()> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x00000001_u32.to_le_bytes());

        let total_len: u32 = 32;
        block.extend_from_slice(&total_len.to_le_bytes());

        block.extend_from_slice(&1_u16.to_le_bytes());
        block.extend_from_slice(&0_u16.to_le_bytes());
        block.extend_from_slice(&65536_u32.to_le_bytes());

        // if_tsresol = 9: packets are stamped in nanoseconds. Without it readers
        // assume microseconds and every timestamp comes out 1000x too large.
        block.extend_from_slice(&9_u16.to_le_bytes());
        block.extend_from_slice(&1_u16.to_le_bytes());
        block.extend_from_slice(&[9, 0, 0, 0]);

        block.extend_from_slice(&0_u32.to_le_bytes());

        block.extend_from_slice(&total_len.to_le_bytes());

        self.file.write_all(&block)
    }

    pub fn write_packet(&mut self, timestamp_ns: u64, data: &[u8]) -> std::io::Result<()> {
        let mut block = Vec::new();

        block.extend_from_slice(&0x00000006_u32.to_le_bytes());

        let interface_id: u32 = 0;
        let ts_high: u32 = (timestamp_ns >> 32) as u32;
        let ts_low: u32 = (timestamp_ns & 0xffffffff) as u32;
        let captured_len: u32 = data.len() as u32;
        let orig_len: u32 = data.len() as u32;

        let padded_len = (data.len() + 3) & !3usize;
        let options_end: u32 = 0;

        let total_len: u32 = 4 + 4 + 4 + 4 + 4 + 4 + 4 + padded_len as u32 + 4 + 4;

        block.extend_from_slice(&total_len.to_le_bytes());
        block.extend_from_slice(&interface_id.to_le_bytes());
        block.extend_from_slice(&ts_high.to_le_bytes());
        block.extend_from_slice(&ts_low.to_le_bytes());
        block.extend_from_slice(&captured_len.to_le_bytes());
        block.extend_from_slice(&orig_len.to_le_bytes());

        block.extend_from_slice(data);
        if padded_len > data.len() {
            block.extend(vec![0u8; padded_len - data.len()]);
        }

        block.extend_from_slice(&options_end.to_le_bytes());

        block.extend_from_slice(&total_len.to_le_bytes());

        self.file.write_all(&block)?;
        self.packet_count += 1;
        Ok(())
    }
}

/// Where a run's packets get logged, shared between its capture threads.
///
/// The file is only created once the first packet arrives, so that a run which
/// never saw any doesn't wipe the capture of the one before it.
#[derive(Clone)]
pub struct CaptureLog(Arc<Mutex<CaptureLogState>>);

struct CaptureLogState {
    file_name: &'static str,
    writer: Option<PcapngWriter>,
    failed: bool,
}

impl CaptureLog {
    pub fn new(file_name: &'static str) -> Self {
        Self(Arc::new(Mutex::new(CaptureLogState {
            file_name,
            writer: None,
            failed: false,
        })))
    }

    pub fn write(&self, timestamp_ns: u64, data: &[u8]) {
        let Ok(mut state) = self.0.lock() else {
            return;
        };

        if state.writer.is_none() && !state.failed {
            match get_pcapng_path(state.file_name).map(PcapngWriter::new) {
                Some(Ok(writer)) => state.writer = Some(writer),
                Some(Err(e)) => {
                    tracing::warn!("Could not create the capture file: {e}");
                    state.failed = true;
                }
                None => state.failed = true,
            }
        }

        if let Some(writer) = &mut state.writer {
            let _ = writer.write_packet(timestamp_ns, data);
        }
    }
}

pub fn log_dir() -> Option<PathBuf> {
    let mut path = eframe::storage_dir("Stardb Exporter")?;
    path.push("log");
    std::fs::create_dir_all(&path).ok()?;
    Some(path)
}

pub fn get_pcapng_path(file_name: &str) -> Option<PathBuf> {
    let mut path = log_dir()?;
    path.push(file_name);
    Some(path)
}

const BLOCK_SECTION_HEADER: u32 = 0x0a0d0d0a;
const BLOCK_INTERFACE: u32 = 0x00000001;
const BLOCK_SIMPLE_PACKET: u32 = 0x00000003;
const BLOCK_ENHANCED_PACKET: u32 = 0x00000006;

const OPTION_TSRESOL: u16 = 9;

const LINK_ETHERNET: u16 = 1;
const LINK_RAW: u16 = 101;
const LINK_IPV4: u16 = 228;
const LINK_IPV6: u16 = 229;

/// A packet from a capture file, always as an Ethernet frame since that's the
/// only thing the sniffers read.
pub struct CapturedPacket {
    pub timestamp_ms: u64,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct CaptureFile {
    pub packets: Vec<CapturedPacket>,
    /// The file ended in the middle of a block.
    pub truncated: bool,
    /// Packets dropped because of what their interface captures.
    pub unsupported_links: BTreeMap<u16, usize>,
}

struct Interface {
    link: u16,
    snap_len: u32,
    /// `if_tsresol` as timestamp units per second, if the file has one.
    units_per_second: Option<u128>,
}

fn timestamp_ms(raw: u64, units_per_second: Option<u128>) -> u64 {
    // Files without if_tsresol mean microseconds, except that older exporter
    // versions wrote nanoseconds and left it out. Microseconds only reach 1e17 in
    // the year 5138, so there's no telling them apart wrongly.
    let units = units_per_second.unwrap_or(if raw > 100_000_000_000_000_000 {
        1_000_000_000
    } else {
        1_000_000
    });
    (raw as u128 * 1000 / units.max(1)) as u64
}

fn ethernet_frame(link: u16, data: &[u8]) -> Option<Vec<u8>> {
    match link {
        LINK_ETHERNET => Some(data.to_vec()),
        LINK_RAW | LINK_IPV4 | LINK_IPV6 => {
            let ether_type: [u8; 2] = match data.first()? >> 4 {
                4 => [0x08, 0x00],
                6 => [0x86, 0xdd],
                _ => return None,
            };
            let mut frame = vec![0; 12];
            frame.extend_from_slice(&ether_type);
            frame.extend_from_slice(data);
            Some(frame)
        }
        _ => None,
    }
}

pub fn read_capture(bytes: &[u8]) -> anyhow::Result<CaptureFile> {
    match bytes.first_chunk::<4>() {
        Some(&[0x0a, 0x0d, 0x0d, 0x0a]) => {}
        Some(&[0xa1, 0xb2, 0xc3, 0xd4] | &[0xd4, 0xc3, 0xb2, 0xa1])
        | Some(&[0xa1, 0xb2, 0x3c, 0x4d] | &[0x4d, 0x3c, 0xb2, 0xa1]) => {
            anyhow::bail!("This is a classic .pcap file. Only .pcapng files can be loaded")
        }
        _ => anyhow::bail!("This is not a .pcapng file"),
    }

    let mut capture = CaptureFile::default();
    let mut interfaces: Vec<Interface> = Vec::new();
    let mut big_endian = false;
    // Simple packet blocks have no timestamp of their own.
    let mut last_timestamp_ms = 0;
    let mut offset = 0;

    while offset < bytes.len() {
        let Some(head) = bytes.get(offset..offset + 12) else {
            capture.truncated = true;
            break;
        };

        let block_type = u32::from_le_bytes(head[..4].try_into()?);

        if block_type == BLOCK_SECTION_HEADER {
            big_endian = match head[8..12] {
                [0x4d, 0x3c, 0x2b, 0x1a] => false,
                [0x1a, 0x2b, 0x3c, 0x4d] => true,
                _ => anyhow::bail!("Unknown byte order at offset {offset}"),
            };
            interfaces.clear();
        }

        let read_u16 = |b: &[u8]| {
            let b = [b[0], b[1]];
            if big_endian {
                u16::from_be_bytes(b)
            } else {
                u16::from_le_bytes(b)
            }
        };
        let read_u32 = |b: &[u8]| {
            let b = [b[0], b[1], b[2], b[3]];
            if big_endian {
                u32::from_be_bytes(b)
            } else {
                u32::from_le_bytes(b)
            }
        };

        let block_type = read_u32(&head[..4]);
        let total_len = read_u32(&head[4..8]) as usize;
        if total_len < 12 || !total_len.is_multiple_of(4) {
            anyhow::bail!("Malformed block of length {total_len} at offset {offset}");
        }
        let Some(block) = bytes.get(offset..offset + total_len) else {
            capture.truncated = true;
            break;
        };
        // Without the leading type and length and the trailing length.
        let body = &block[8..total_len - 4];

        match block_type {
            BLOCK_INTERFACE if body.len() >= 8 => {
                let mut units_per_second = None;

                let mut options = &body[8..];
                while let Some(option) = options.get(..4) {
                    let code = read_u16(&option[..2]);
                    let len = read_u16(&option[2..4]) as usize;
                    if code == 0 {
                        break;
                    }
                    let Some(value) = options.get(4..4 + len) else {
                        break;
                    };
                    if code == OPTION_TSRESOL
                        && let Some(&resolution) = value.first()
                    {
                        let exponent = u32::from(resolution & 0x7f);
                        let base: u128 = if resolution & 0x80 == 0 { 10 } else { 2 };
                        units_per_second = base.checked_pow(exponent);
                    }
                    options = options
                        .get(4 + len.next_multiple_of(4)..)
                        .unwrap_or_default();
                }

                interfaces.push(Interface {
                    link: read_u16(&body[..2]),
                    snap_len: read_u32(&body[4..8]),
                    units_per_second,
                });
            }
            BLOCK_ENHANCED_PACKET if body.len() >= 20 => {
                let interface = interfaces.get(read_u32(&body[..4]) as usize);
                let raw =
                    (u64::from(read_u32(&body[4..8])) << 32) | u64::from(read_u32(&body[8..12]));
                let captured_len = read_u32(&body[12..16]) as usize;
                let Some(data) = body.get(20..20 + captured_len) else {
                    anyhow::bail!("Malformed packet at offset {offset}");
                };
                let Some(interface) = interface else {
                    anyhow::bail!("Packet for an unknown interface at offset {offset}");
                };

                last_timestamp_ms = timestamp_ms(raw, interface.units_per_second);
                match ethernet_frame(interface.link, data) {
                    Some(data) => capture.packets.push(CapturedPacket {
                        timestamp_ms: last_timestamp_ms,
                        data,
                    }),
                    None => {
                        *capture.unsupported_links.entry(interface.link).or_default() += 1;
                    }
                }
            }
            BLOCK_SIMPLE_PACKET if body.len() >= 4 => {
                let Some(interface) = interfaces.first() else {
                    anyhow::bail!("Packet for an unknown interface at offset {offset}");
                };
                let mut captured_len = read_u32(&body[..4]) as usize;
                if interface.snap_len != 0 {
                    captured_len = captured_len.min(interface.snap_len as usize);
                }
                let Some(data) = body.get(4..4 + captured_len) else {
                    anyhow::bail!("Malformed packet at offset {offset}");
                };

                match ethernet_frame(interface.link, data) {
                    Some(data) => capture.packets.push(CapturedPacket {
                        timestamp_ms: last_timestamp_ms,
                        data,
                    }),
                    None => {
                        *capture.unsupported_links.entry(interface.link).or_default() += 1;
                    }
                }
            }
            _ => {}
        }

        offset += total_len;
    }

    Ok(capture)
}

/// What a capture contains, without decrypting anything.
#[derive(Default)]
pub struct CaptureSummary {
    pub ethernet_received: usize,
    pub ethernet_sent: usize,
    pub wifi_received: usize,
    pub wifi_sent: usize,
    /// UDP packets for other games, by the lower of their two ports.
    pub other_ports: BTreeMap<u16, usize>,
    /// Anything that isn't UDP over IP, or in a frame that couldn't be read.
    pub unreadable: usize,
    pub first_timestamp_ms: u64,
    pub last_timestamp_ms: u64,
}

impl CaptureSummary {
    pub fn new(capture: &CaptureFile, ports: (u16, u16)) -> Self {
        let mut summary = Self {
            first_timestamp_ms: capture.packets.first().map_or(0, |p| p.timestamp_ms),
            last_timestamp_ms: capture.packets.last().map_or(0, |p| p.timestamp_ms),
            ..Self::default()
        };

        for packet in &capture.packets {
            let Some((wifi, ip)) = ip_packet(&packet.data) else {
                summary.unreadable += 1;
                continue;
            };
            let Some((source, destination)) = udp_ports(ip) else {
                summary.unreadable += 1;
                continue;
            };

            let is_ours = |port: u16| (ports.0..=ports.1).contains(&port);
            let count = match (is_ours(source), is_ours(destination), wifi) {
                (true, _, false) => &mut summary.ethernet_received,
                (true, _, true) => &mut summary.wifi_received,
                (false, true, false) => &mut summary.ethernet_sent,
                (false, true, true) => &mut summary.wifi_sent,
                (false, false, _) => summary
                    .other_ports
                    .entry(source.min(destination))
                    .or_default(),
            };
            *count += 1;
        }

        summary
    }

    /// Whether anything at all in the capture is traffic of this game.
    pub fn has_game_traffic(&self) -> bool {
        self.ethernet_received + self.ethernet_sent + self.wifi_received + self.wifi_sent > 0
    }
}

/// The IP packet in a frame, and whether that frame was a raw 802.11 one. On Wi-Fi,
/// pktmon reports each received packet several times, as both.
fn ip_packet(frame: &[u8]) -> Option<(bool, &[u8])> {
    if let Some([0x08, 0x00] | [0x86, 0xdd]) = frame.get(12..14) {
        return Some((false, &frame[14..]));
    }

    let frame_control = *frame.first()?;
    // Data frames only. QoS ones have two more header bytes.
    if frame_control & 0x0c != 0x08 {
        return None;
    }
    let header_len = if frame_control & 0x80 == 0 { 24 } else { 26 };
    let llc = frame.get(header_len..header_len + 8)?;
    if llc[..6] != [0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00] || llc[6..] != [0x08, 0x00] {
        return None;
    }
    Some((true, &frame[header_len + 8..]))
}

fn udp_ports(ip: &[u8]) -> Option<(u16, u16)> {
    let udp = match ip.first()? >> 4 {
        4 if *ip.get(9)? == 17 => ip.get((ip[0] & 0x0f) as usize * 4..)?,
        6 if *ip.get(6)? == 17 => ip.get(40..)?,
        _ => return None,
    };
    let ports = udp.get(..4)?;
    Some((
        u16::from_be_bytes([ports[0], ports[1]]),
        u16::from_be_bytes([ports[2], ports[3]]),
    ))
}

#[cfg(test)]
mod tests;
