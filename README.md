# `transmission-compose`

A very small tool to organize torrents into directory trees using the transmission RPC. Think docker-compose but arrr.

Place your config in the `config.yaml` file. This file by default contains an example to start downloading the Archlinux and NixOS ISO torrents.

Place any credentials in a `.env` file in the current directory. For example:

```
USER="transmission"
PASSWORD="yoursuperstrongpasswordbruh"
```

This file can be left empty if no RPC password is set.

Now execute `cargo run`.
