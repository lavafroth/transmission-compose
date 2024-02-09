# `transmission-compose`

A very small tool to organize torrents into directory trees using the
transmission RPC. Think docker-compose but arrr.

## Installation

Install Rust (preferably using rustup) and run the following:

```sh
cargo install --git https://github.com/lavafroth/transmission-compose
```

## Usage

Place your config in the `config.yaml`file. Credentials can be specified in the
config file using the `username` and `password` fields. They can also be left
unspecified in case the RPC does not require authentication. The `root` field
specifies the directory structure of the torrents to be downloaded. This root is
implicitly set to the download directory for the current transmission session.
Make sure to have the daemon running in the proper directory before running
`transmission-compose`.

The config entries for each directory can have the following two fields:
- `torrents`: a list of URLs, paths or magnet links that will be downloaded to the directory
- `children`: sub-directories, each of whom can further have these two fields

The example config file uses no credentials (commented out) and starts downloading the Archlinux and NixOS ISO torrents.

After making sure that the transmission-daemon is running, execute the following
to run transmission-compose.

```sh
cargo run
```
