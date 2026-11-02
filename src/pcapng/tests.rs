use super::*;

fn block(block_type: u32, body: &[u8]) -> Vec<u8> {
    let padded = body.len().next_multiple_of(4);
    let total_len = (12 + padded) as u32;
    let mut block = Vec::new();
    block.extend_from_slice(&block_type.to_le_bytes());
    block.extend_from_slice(&total_len.to_le_bytes());
    block.extend_from_slice(body);
    block.resize(8 + padded, 0);
    block.extend_from_slice(&total_len.to_le_bytes());
    block
}

fn section_header() -> Vec<u8> {
    let mut body = 0x1a2b3c4d_u32.to_le_bytes().to_vec();
    body.extend_from_slice(&[1, 0, 0, 0]);
    body.extend_from_slice(&u64::MAX.to_le_bytes());
    block(BLOCK_SECTION_HEADER, &body)
}

fn interface(link: u16, options: &[u8]) -> Vec<u8> {
    let mut body = link.to_le_bytes().to_vec();
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(&65536_u32.to_le_bytes());
    body.extend_from_slice(options);
    block(BLOCK_INTERFACE, &body)
}

fn packet(timestamp: u64, data: &[u8]) -> Vec<u8> {
    let mut body = 0_u32.to_le_bytes().to_vec();
    body.extend_from_slice(&((timestamp >> 32) as u32).to_le_bytes());
    body.extend_from_slice(&(timestamp as u32).to_le_bytes());
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(data);
    body.resize(body.len().next_multiple_of(4), 0);
    block(BLOCK_ENHANCED_PACKET, &body)
}

const NOW_MS: u64 = 1_792_265_711_791;

#[test]
fn reads_back_what_the_writer_wrote() {
    let path =
        std::env::temp_dir().join(format!("stardb-roundtrip-{}.pcapng", std::process::id()));
    let mut writer = PcapngWriter::new(path.clone()).unwrap();
    writer
        .write_packet(NOW_MS * 1_000_000, &[1, 2, 3, 4, 5])
        .unwrap();
    writer
        .write_packet((NOW_MS + 1) * 1_000_000, &[6; 8])
        .unwrap();
    drop(writer);

    let capture = read_capture(&std::fs::read(&path).unwrap()).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert!(!capture.truncated);
    assert_eq!(capture.packets.len(), 2);
    assert_eq!(capture.packets[0].data, [1, 2, 3, 4, 5]);
    assert_eq!(capture.packets[0].timestamp_ms, NOW_MS);
    assert_eq!(capture.packets[1].data, [6; 8]);
    assert_eq!(capture.packets[1].timestamp_ms, NOW_MS + 1);
}

#[test]
fn missing_resolution_means_microseconds_unless_written_by_an_old_exporter() {
    let mut file = section_header();
    file.extend(interface(LINK_ETHERNET, &[]));
    file.extend(packet(NOW_MS * 1_000, &[1]));
    file.extend(packet(NOW_MS * 1_000_000, &[2]));

    let capture = read_capture(&file).unwrap();
    assert_eq!(capture.packets[0].timestamp_ms, NOW_MS);
    assert_eq!(capture.packets[1].timestamp_ms, NOW_MS);
}

#[test]
fn raw_ip_gets_an_ethernet_header() {
    let mut file = section_header();
    file.extend(interface(LINK_RAW, &[]));
    file.extend(packet(0, &[0x45, 0, 0, 0]));
    file.extend(packet(0, &[0x60, 0, 0, 0]));

    let capture = read_capture(&file).unwrap();
    assert_eq!(capture.packets[0].data[12..], [0x08, 0x00, 0x45, 0, 0, 0]);
    assert_eq!(capture.packets[1].data[12..], [0x86, 0xdd, 0x60, 0, 0, 0]);
}

#[test]
fn a_cut_off_file_keeps_what_came_before() {
    let mut file = section_header();
    file.extend(interface(LINK_ETHERNET, &[]));
    file.extend(packet(0, &[1, 2, 3, 4]));
    file.extend(&packet(0, &[5, 6, 7, 8])[..10]);

    let capture = read_capture(&file).unwrap();
    assert!(capture.truncated);
    assert_eq!(capture.packets.len(), 1);
}
