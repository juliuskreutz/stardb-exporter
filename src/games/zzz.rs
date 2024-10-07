use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use regex::Regex;

pub fn pulls() -> anyhow::Result<String> {
    let mut game_path = game_path()?;

    game_path.push("webCaches");

    let re = Regex::new(r"^\d+\.\d+\.\d+\.\d+$")?;
    let mut paths: Vec<_> = game_path
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
                    .and_then(|r| r.into_json::<serde_json::Value>().ok())
                    .map(|j| j["retcode"] == 0)
                    .unwrap_or_default()
                {
                    return Ok(url.to_string());
                } else {
                    return Err(anyhow::anyhow!("Warp url outdated"));
                }
            }
        }
    }

    Err(anyhow::anyhow!("Couldn't find warp url"))
}

fn game_path() -> anyhow::Result<PathBuf> {
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
