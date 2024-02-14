use crate::torrent::Torrent;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Deserialize, Debug)]
pub struct Session {
    pub arguments: Arguments,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Arguments {
    pub download_dir: PathBuf,
}

#[derive(Serialize)]
pub struct Request {
    pub method: &'static str,
    pub arguments: Torrent,
}
