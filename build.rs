use cfg_aliases::cfg_aliases;

fn main() {

    cfg_aliases! {
        // pktmon for windows only and pcap feture not enbles
        //pcap for linux,bsd,mac or windows with feature flag on
        pktmon: { all(target_os = "windows", not(feature = "pcap")) },
        pcap: { any(not(target_os = "windows"), feature = "pcap") },
    }

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        res.compile().unwrap();
    }
}
