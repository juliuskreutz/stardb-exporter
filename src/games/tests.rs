use super::*;
use crate::pcapng::CapturedPacket;

fn stream_of<'a>(packets: Vec<Packet>, run: &'a RunHandle) -> PacketStream<'a> {
    let (tx, rx) = mpsc::channel();
    for packet in packets {
        tx.send(packet).unwrap();
    }
    PacketStream::new(rx, run, None)
}

#[test]
fn stopping_while_waiting_on_a_packet_is_not_running_out_of_them() {
    let (tx, rx) = mpsc::channel::<Packet>();
    let run = RunHandle::default();
    let mut stream = PacketStream::new(rx, &run, None);

    // The way a stop plays out: the flag is set, then the captures exit, all
    // while the stream is past checking the flag and waiting on the channel.
    let stopper = run.clone();
    let captures = thread::spawn(move || {
        thread::sleep(POLL_INTERVAL / 4);
        stopper.stop();
        drop(tx);
    });

    assert!(matches!(stream.next(None), Received::Stopped));
    captures.join().unwrap();
}

#[test]
fn hsr_asks_to_wait_for_an_update_but_the_others_do_not() {
    let error = ExportError::KeyMissing {
        version: Some(1),
        saw_handshake: true,
    };
    let mentions_update = |game| {
        error
            .report(game)
            .steps
            .iter()
            .any(|step: &String| step.contains("update"))
    };
    assert!(mentions_update(Game::Hsr));
    assert!(!mentions_update(Game::Gi));
    assert!(!mentions_update(Game::Zzz));
}

// The rest need real captures, which hold login data and don't belong in the
// repository. STARDB_CAPTURES is a directory with:
//   {name}.pcapng               the captures
//   {hsr,gi,zzz}-achievements.json   what stardb.gg serves for them
//   expected.txt                lines of "name game regular arcade" for ZZZ
//                               and "name game total" for the others
// Run with: cargo test --features pktmon -- --ignored --nocapture

fn captures_dir() -> PathBuf {
    PathBuf::from(std::env::var("STARDB_CAPTURES").expect("STARDB_CAPTURES is not set"))
}

fn game_named(name: &str) -> Game {
    match name {
        "hsr" => Game::Hsr,
        "gi" => Game::Gi,
        "zzz" => Game::Zzz,
        _ => panic!("unknown game {name}"),
    }
}

fn game_ids(game: &str) -> Vec<(u32, bool)> {
    #[derive(serde::Deserialize)]
    struct Achievement {
        id: u32,
        #[serde(default)]
        arcade: bool,
    }
    let path = captures_dir().join(format!("{game}-achievements.json"));
    let achievements: Vec<Achievement> =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    achievements.into_iter().map(|a| (a.id, a.arcade)).collect()
}

fn capture(name: &str) -> CaptureFile {
    let path = captures_dir().join(format!("{name}.pcapng"));
    read_capture(&std::fs::read(path).unwrap()).unwrap()
}

fn as_packets(packets: Vec<CapturedPacket>) -> Vec<Packet> {
    packets
        .into_iter()
        .map(|p| Packet {
            data: p.data,
            timestamp_ms: p.timestamp_ms,
        })
        .collect()
}

fn expected() -> Vec<Vec<String>> {
    std::fs::read_to_string(captures_dir().join("expected.txt"))
        .unwrap()
        .lines()
        .map(|line| line.split_whitespace().map(str::to_string).collect())
        .collect()
}

/// Stopping has to work while no packet ever arrives, which is exactly when a
/// capture blocks. Only needs a network device, and permission to use it.
#[test]
#[ignore = "opens a real capture"]
fn a_live_capture_stops_promptly_without_any_packets() {
    let (device_tx, device_rx) = mpsc::channel();
    let (message_tx, message_rx) = mpsc::channel();
    let run = RunHandle::default();

    let captures = Game::Hsr
        .start_captures(device_tx, &message_tx, &run)
        .unwrap();
    println!("{} capture(s) started", captures.len());
    thread::sleep(Duration::from_secs(2));

    let stopping = Instant::now();
    run.stop();
    for capture in captures {
        println!("capture ended with {:?}", capture.join().unwrap());
    }
    println!("stopped in {:?}", stopping.elapsed());
    assert!(stopping.elapsed() < Duration::from_secs(3));

    drop((device_rx, message_rx));
}

#[test]
#[ignore = "needs STARDB_CAPTURES"]
fn captures_give_the_expected_achievements() {
    let run = RunHandle::default();
    for line in expected() {
        let (name, game_name) = (&line[0], &line[1]);
        let game = game_named(game_name);
        let file = capture(name);
        let summary = CaptureSummary::new(&file, game.ports());
        let total = file.packets.len();
        let (truncated, unsupported) = (file.truncated, Vec::new());
        let (outcome, progress) = game.replay_packets(&game_ids(game_name), file, &run);
        println!(
            "{}\n",
            game.capture_report(
                name,
                total,
                truncated,
                &unsupported,
                &summary,
                &progress,
                &outcome
            )
        );

        let achievements = outcome.unwrap();
        if game == Game::Zzz {
            assert_eq!(
                achievements.breakdown,
                Some(Breakdown {
                    regular: Some(line[2].parse().unwrap()),
                    arcade: Some(line[3].parse().unwrap()),
                }),
                "{name}"
            );
        } else {
            assert_eq!(
                achievements.ids.len(),
                line[2].parse::<usize>().unwrap(),
                "{name}"
            );
        }
    }
}

#[test]
#[ignore = "needs STARDB_CAPTURES"]
fn a_capture_without_its_login_says_so() {
    let run = RunHandle::default();
    for line in expected() {
        let (name, game_name) = (&line[0], &line[1]);
        let game = game_named(game_name);
        let mut file = capture(name);
        file.packets.drain(..file.packets.len() / 2);
        let (outcome, progress) = game.replay_packets(&game_ids(game_name), file, &run);
        println!(
            "{name}: {:?}, handshakes {}",
            outcome.as_ref().err(),
            progress.handshakes
        );
        assert_eq!(progress.handshakes, 0, "{name}");
        assert!(
            matches!(
                outcome,
                Err(ExportError::CaptureEnded
                    | ExportError::KeyMissing {
                        saw_handshake: false,
                        ..
                    })
            ),
            "{name}: {:?}",
            outcome.as_ref().err()
        );
    }
}

#[test]
#[ignore = "needs STARDB_CAPTURES"]
fn a_capture_without_a_matching_key_says_so() {
    let run = RunHandle::default();
    for line in expected() {
        let (name, game_name) = (&line[0], &line[1]);
        let game = game_named(game_name);
        let ids = game_ids(game_name);
        let packets = as_packets(capture(name).packets);
        let mut stream = stream_of(packets, &run);
        let outcome = match game {
            Game::Zzz => zzz::sniff_with_keys(vec![vec![0; 16]], &ids, &mut stream),
            Game::Gi => gi::sniff_with_keys(Default::default(), &ids, &mut stream),
            Game::Hsr => hsr::sniff_with_keys(Default::default(), &ids, &mut stream),
        };
        println!(
            "{name}: {:?} after {} packets",
            outcome.as_ref().err(),
            stream.progress.packets
        );
        assert!(
            matches!(
                outcome,
                Err(ExportError::KeyMissing {
                    saw_handshake: true,
                    ..
                })
            ),
            "{name}: {:?}",
            outcome.as_ref().err()
        );
    }
}
