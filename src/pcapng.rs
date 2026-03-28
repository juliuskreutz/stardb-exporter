use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

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

        let total_len: u32 = 24;
        block.extend_from_slice(&total_len.to_le_bytes());

        block.extend_from_slice(&1_u16.to_le_bytes());
        block.extend_from_slice(&0_u16.to_le_bytes());
        block.extend_from_slice(&65536_u32.to_le_bytes());

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

pub fn get_pcapng_path() -> Option<PathBuf> {
    let storage_dir = eframe::storage_dir("Stardb Exporter")?;
    let mut path = storage_dir;
    path.push("log");
    std::fs::create_dir_all(&path).ok()?;
    path.push("latest.pcapng");
    Some(path)
}
