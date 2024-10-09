use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

pub fn game_path() -> anyhow::Result<PathBuf> {
    let mut log_path = PathBuf::from(&std::env::var("APPDATA")?);
    log_path.pop();
    log_path.push("LocalLow");
    log_path.push("miHoYo");

    let mut log_path_cn = log_path.clone();

    log_path.push("ZenlessZoneZero");
    log_path_cn.push("绝区零");

    log_path.push("Player.log");
    log_path_cn.push("Player.log");

    let log_path = match (log_path.exists(), log_path_cn.exists()) {
        (true, _) => log_path,
        (_, true) => log_path_cn,
        _ => return Err(anyhow::anyhow!("Can't find log file")),
    };

    for line in BufReader::new(File::open(log_path)?).lines() {
        let Ok(line) = line else {
            break;
        };

        if let Some(line) = line.strip_prefix("[Subsystems] Discovering subsystems at path ") {
            let game_path = line.strip_suffix("/UnitySubsystems").unwrap();

            return Ok(PathBuf::from(game_path));
        }
    }

    Err(anyhow::anyhow!("Couldn't find game path"))
}
