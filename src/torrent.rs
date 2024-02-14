use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Torrent {
    File {
        filename: String,
        download_dir: String,
    },
    Metainfo {
        metainfo: String,
        download_dir: String,
    },
}
