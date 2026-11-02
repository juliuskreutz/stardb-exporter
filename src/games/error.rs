use super::Game;

/// Why a run ended without achievements.
#[derive(Debug, PartialEq)]
pub enum ExportError {
    /// The user pressed stop. Not an error to them.
    Cancelled,
    KeyMissing {
        version: Option<u32>,
        saw_handshake: bool,
    },
    SessionSeedMissing,
    BruteforceFailed,
    SessionKeyLost,
    /// The packets ran out before any achievements came by.
    CaptureEnded,
    #[cfg_attr(feature = "pktmon", allow(dead_code))]
    NoDevices,
    CaptureFailed(String),
    AchievementIds(String),
    /// A capture file that could not be read.
    CaptureUnreadable(String),
    Internal(String),
    Other(String),
}

impl From<anyhow::Error> for ExportError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e.to_string())
    }
}

/// What to show the user about an [`ExportError`].
pub struct ErrorReport {
    pub title: String,
    pub detail: Option<String>,
    pub steps: Vec<String>,
    /// Whether to ask for a ticket if the steps don't help.
    pub support: bool,
    /// Set when this came from loading a capture rather than a live run.
    pub capture_report: Option<String>,
}

const RESTART_STEPS: [&str; 3] = [
    "Close the game completely.",
    "Click Try again below and wait until the exporter says it's ready.",
    "Start the game and log in.",
];

fn restart_steps() -> Vec<String> {
    RESTART_STEPS.iter().map(|step| step.to_string()).collect()
}

impl ExportError {
    pub fn report(&self, game: Game) -> ErrorReport {
        let mut report = ErrorReport {
            title: String::new(),
            detail: None,
            steps: Vec::new(),
            support: true,
            capture_report: None,
        };

        match self {
            Self::Cancelled => report.title = "The run was stopped".to_string(),
            Self::KeyMissing { saw_handshake, .. } => {
                if *saw_handshake {
                    report.title = "Couldn't find the key for this game version".to_string();
                } else {
                    report.title = "The exporter started after the game".to_string();
                    report.detail =
                        Some("The exporter has to be running before the game logs in.".to_string());
                }
                report.steps = restart_steps();
                if game == Game::Hsr {
                    report.steps.push(
                        "If it keeps happening, the game has probably been updated. Wait for an exporter update."
                            .to_string(),
                    );
                }
            }
            Self::SessionSeedMissing => {
                report.title = "Couldn't find the session key's seed".to_string();
                report.steps = restart_steps();
                report.steps.push(
                    "If it keeps happening, the game has probably been updated. Wait for an exporter update."
                        .to_string(),
                );
            }
            Self::BruteforceFailed => {
                report.title = "Couldn't work out the session key".to_string();
                report.steps = restart_steps();
            }
            Self::SessionKeyLost => {
                report.title = "The session key stopped working".to_string();
                report.detail = Some(
                    "This can happen when the game reconnects while it's being read.".to_string(),
                );
                report.steps = restart_steps();
            }
            Self::CaptureEnded => {
                report.title = "Stopped receiving packets before any achievements".to_string();
                report.steps = restart_steps();
            }
            Self::NoDevices => {
                report.title = "No network devices found".to_string();
                report
                    .steps
                    .push("Make sure you are connected to the internet.".to_string());
                #[cfg(feature = "pcap")]
                report
                    .steps
                    .push("Make sure Npcap is installed.".to_string());
                report.support = false;
            }
            Self::CaptureFailed(e) => {
                report.title = "Couldn't capture packets".to_string();
                report.detail = (!e.is_empty()).then(|| e.clone());
                #[cfg(feature = "pcap")]
                report.steps.push(
                    "Make sure Npcap is installed, in WinPcap API-compatible mode.".to_string(),
                );
                #[cfg(feature = "pktmon")]
                report
                    .steps
                    .push("Make sure the exporter is running as administrator.".to_string());
                report.steps.push("Try again.".to_string());
                report.support = false;
            }
            Self::AchievementIds(e) => {
                report.title = "Couldn't download the achievement list".to_string();
                report.detail = Some(e.clone());
                report.steps = vec!["Check your internet connection and try again.".to_string()];
                report.support = false;
            }
            Self::CaptureUnreadable(e) => {
                report.title = "Couldn't read this capture".to_string();
                report.detail = Some(e.clone());
            }
            Self::Internal(e) => {
                report.title = "Something went wrong inside the exporter".to_string();
                report.detail = Some(e.clone());
                report.steps = vec!["Try again.".to_string()];
            }
            Self::Other(e) => {
                report.title = "Something went wrong".to_string();
                report.detail = Some(e.clone());
                report.steps = vec!["Try again.".to_string()];
            }
        }

        report
    }
}
