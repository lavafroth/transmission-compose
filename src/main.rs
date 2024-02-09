use anyhow::{bail, Result};
use dotenvy::dotenv;
use serde::Deserialize;
use std::env;
use std::path::Path;
use std::{collections::HashMap, fs};
use transmission_rpc::types::TorrentAddArgs;
use transmission_rpc::{types::BasicAuth, TransClient};

#[derive(Debug, Deserialize)]
pub struct Torrent {
    url: String,
    find: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    torrents: Option<Vec<Torrent>>,
    children: Option<Directory>,
}

#[derive(Debug, Deserialize)]
pub struct Directory(HashMap<String, Entry>);

impl Directory {
    fn write_traversal_to_vec(&self, download_dir: &Path, list: &mut Vec<(String, String)>) {
        for (directory, entry) in self.0.iter() {
            let mut download_dir = download_dir.to_path_buf();
            download_dir.push(directory);
            if let Some(torrents) = &entry.torrents {
                for torrent in torrents {
                    list.push((
                        torrent.url.clone(),
                        download_dir.to_string_lossy().to_string(),
                    ));
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
    dotenv().ok();
    simple_logger::init_with_level(log::Level::Info)?;
    let url = env::var("URL").unwrap_or("http://localhost:9091/transmission/rpc".to_string());
    let mut client = if let (Ok(user), Ok(password)) = (env::var("USER"), env::var("PASSWORD")) {
        let basic_auth = BasicAuth { user, password };
        TransClient::with_auth(url.parse()?, basic_auth)
    } else {
        TransClient::new(url.parse()?)
    };

    let Ok(response) = client.session_get().await else {
        bail!("failed to retrieve information about the current transmission session");
    };

    let download_dir = Path::new(&response.arguments.download_dir);
    let config: Directory = serde_yaml::from_str(&fs::read_to_string("config.yml")?)?;
    for (filename, download_dir) in config.traverse(download_dir) {
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
