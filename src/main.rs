use anyhow::{bail, Result};
use serde::Deserialize;
use std::path::Path;
use std::{collections::HashMap, fs};
use transmission_rpc::types::TorrentAddArgs;
use transmission_rpc::{types::BasicAuth, TransClient};

#[derive(Debug, Deserialize)]
pub struct Entry {
    torrents: Option<Vec<String>>,
    children: Option<Directory>,
}
#[derive(Debug, Deserialize)]
pub struct Directory(HashMap<String, Entry>);

#[derive(Debug, Deserialize)]
pub struct Config {
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    root: Directory,
}

impl Directory {
    fn write_traversal_to_vec(&self, download_dir: &Path, list: &mut Vec<(String, String)>) {
        for (directory, entry) in self.0.iter() {
            let mut download_dir = download_dir.to_path_buf();
            download_dir.push(directory);
            if let Some(torrents) = &entry.torrents {
                for torrent in torrents {
                    list.push((torrent.clone(), download_dir.to_string_lossy().to_string()));
                }
            }
            if let Some(children) = &entry.children {
                children.write_traversal_to_vec(&download_dir, list);
            }
        }
    }

    fn traverse(&self, download_dir: &Path) -> Vec<(String, String)> {
        let mut list = vec![];
        self.write_traversal_to_vec(download_dir, &mut list);
        list
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;
    let config: Config = serde_yaml::from_str(&fs::read_to_string("config.yml")?)?;
    let url = config
        .url
        .unwrap_or("http://localhost:9091/transmission/rpc".to_string())
        .parse()?;
    let mut client = match (config.username, config.password) {
        (Some(user), Some(password)) => TransClient::with_auth(url, BasicAuth { user, password }),
        _ => TransClient::new(url),
    };

    let Ok(response) = client.session_get().await else {
        bail!("failed to retrieve information about the current transmission session");
    };

    let download_dir = Path::new(&response.arguments.download_dir);
    for (filename, download_dir) in config.root.traverse(download_dir) {
        match client
            .torrent_add(TorrentAddArgs {
                filename: Some(filename.clone()),
                download_dir: Some(download_dir.clone()),
                ..Default::default()
            })
            .await
        {
            Ok(_) => log::info!("added torrent {} to {}", filename, download_dir),
            Err(_) => log::error!("failed to add torrent {} to {}", filename, download_dir),
        };
    }
    Ok(())
}
