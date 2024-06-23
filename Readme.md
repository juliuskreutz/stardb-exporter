# stardb-exporter

## Instructions

This method will not work on any kind of VPN

- Download and install pcap
  - Windows: [Npcap Installer](https://npcap.com/#download) (Ensure `Install Npcap in WinPcap API-compatible mode` is ticked)
  - Linux: Figure it out, lol. The package should be called libpcap
  - Macos: Use brew https://formulae.brew.sh/formula/libpcap

- Note for WiFi users:
  - Windows: During Npcap installation, ensure `Support raw 802.11 traffic (and monitor mode) for wireless adapters` is ticked.
  - Linux and Macos: Make sure you enable monitor mode for your wireless adapter.

- Download the latest release [here](https://github.com/juliuskreutz/stardb-exporter/releases/latest) (and move it to its own folder preferably).
- Launch the game to the point where the train is right before going into hyper speed.
- Execute the exporter (You might need to do this as admin/root) and wait for it to say `All ready~!`.
- Go into hyperspeed and it should copy the export to your clipboard.
- Paste it [here](https://stardb.gg/import).
