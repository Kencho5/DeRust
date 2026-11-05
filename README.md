<p align="center">
  <img src="assets/icon.png" width="128" alt="derust icon">
</p>

<h1 align="center">derust</h1>

<p align="center">A fast, free and open source alternative to CleanMyMac, written in Rust.</p>

![derust](docs/screenshot.png)

## Features

- **System Junk**: caches, logs, Xcode leftovers and broken launch agents
- **Disk Explorer**: browse your whole disk by size and remove what you don't need
- **Applications**: uninstall apps together with their leftover files

Everything goes to the Trash, so nothing is lost by accident.

## Performance

Scans use `getattrlistbulk` and run in parallel on the performance cores. A full disk with about 2.3 million files takes around 10 seconds on an M4 Pro.

The release binary is about **7.5 MB**.

## Building

```sh
cargo build --release
./target/release/derust
```

To build a universal `derust.app` with a `.dmg`, run `./scripts/bundle.sh`.

Grant Full Disk Access in System Settings for complete results.

## Why

This is something I built for myself to replace CleanMyMac, which is paid. I'm sharing it in case it's useful to anyone else.

## AI Usage

- **UI**: copied from my startup. A designer and I made it.
- **Scanning and categorization**: copied one to one from another project of mine that I wrote years ago. I reviewed it manually and changed the removal logic so files go to the Trash instead of being permanently deleted.
- **Iced UI implementation**: written with AI. This is the only part I haven't fully reviewed. It's a thin layer and can't cause catastrophic bugs.

## License

[MIT](LICENSE). Space Grotesk is licensed under the [SIL Open Font License](assets/fonts/OFL.txt).
