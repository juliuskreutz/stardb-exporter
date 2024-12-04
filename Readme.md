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

- Download the latest release:
  - [Windows](https://github.com/juliuskreutz/stardb-exporter/releases/latest/download/stardb-exporter.exe)
  - [Linux](https://github.com/juliuskreutz/stardb-exporter/releases/latest/download/stardb-exporter-linux)
  - [MacOs](https://github.com/juliuskreutz/stardb-exporter/releases/latest/download/stardb-exporter-macos)
- Launch the game to the point where.
  - HSR: The train is right before going into hyper speed
  - Genshin: Right before entering the door
- Execute the exporter (You might need to do this as admin/root) and wait for it to say `Device <i> ready~!`.
- Go into hyperspeed/Enter the door and it should copy the export to your clipboard.
- Paste it [here](https://stardb.gg/import).

## Special thanks

Thank you [@IceDynamix](https://github.com/IceDynamix) for providing the building blocks for this with their [reliquary](https://github.com/IceDynamix/reliquary) project!

Thank you [@hashblen](https://github.com/hashblen) for creating a protocol parsers that don't need any further updates ([auto-reliquary](https://github.com/hashblen/auto-reliquary) and [auto-artifactarium](https://github.com/hashblen/auto-artifactarium))!

Thank you [@emmachase](https://github.com/emmachase) for providing support wherever she can!
