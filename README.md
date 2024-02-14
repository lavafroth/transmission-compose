# `transmission-compose`

A very small tool to organize torrents into directory trees using the
transmission RPC. Think docker-compose but arrr.

## Installation

### Binaries

Download the binary for your system from the [releases](https://github.com/lavafroth/transmission-compose/releases).

### From source

Install Rust (preferably using rustup) and run the following:

```sh
cargo install --git https://github.com/lavafroth/transmission-compose
```

You will now be able to execute `transmission-daemon` from the commandline.

## Usage

Place your config in the `config.yaml` file. RPC credentials can be optionally
specified in the config file using the `username` and `password` fields. For
adding torrents faster, the `concurrency` field can be set to a number larger
than the default of 4. The `root` field specifies the directory structure of the
torrents to be downloaded. This root is implicitly set to the download directory
for the current transmission session. Make sure to have the daemon running in
the proper directory before running `transmission-compose`.

The config entries for each directory can have the following two fields:
- `torrents`: a list of URLs, paths or magnet links that will be downloaded to the directory
- `children`: sub-directories, each of whom can further have these two fields

The example config file uses no credentials (commented out) and starts downloading the Archlinux and NixOS ISO torrents.

After making sure that the transmission-daemon is running, run `transmission-compose` inside the directory which contains the `config.yml` file.

### Torrent parsing nuance

If the torrent specified is a magnet link, it is passed straight to the RPC.
However, if it is a file path, transmission-compose will try to read the file if
it exists locally and send the contents (metainfo) to the RPC.

If reading the file fails due to lack of permission or the file not existing,
the RPC receives the file path and has to deal with it.
