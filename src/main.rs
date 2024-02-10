use anyhow::{bail, Result};
use futures::{stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};
use url::Url;

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
    concurrency: Option<usize>,
    root: Directory,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Torrent {
    filename: String,
    download_dir: String,
}

impl Directory {
    fn write_traversal_to_vec(&self, download_dir: &Path, list: &mut Vec<Torrent>) {
        for (directory, entry) in self.0.iter() {
            let mut download_dir = download_dir.to_path_buf();
            download_dir.push(directory);
            if let Some(torrents) = &entry.torrents {
                for torrent in torrents {
                    list.push(Torrent {
                        filename: torrent.clone(),
                        download_dir: download_dir.to_string_lossy().to_string(),
                    });
                }
            }
            if let Some(children) = &entry.children {
                children.write_traversal_to_vec(&download_dir, list);
            }
        }
    }

    fn traverse(&self, download_dir: &Path) -> Vec<Torrent> {
        let mut list = vec![];
        self.write_traversal_to_vec(download_dir, &mut list);
        list
    }
}

async fn get_csrf_token(
    url: Url,
    username: Option<String>,
    password: Option<String>,
) -> Result<Option<HeaderValue>> {
    let requestbuilder = reqwest::Client::new().get(url);
    let resp = if let Some(username) = username {
        requestbuilder.basic_auth(username, password)
    } else {
        requestbuilder
    }
    .send()
    .await?;
    if let (StatusCode::CONFLICT, Some(id)) = (
        resp.status(),
        resp.headers().get("X-Transmission-Session-Id"),
    ) {
        Ok(Some(id.to_owned()))
    } else {
        Ok(None)
    }
}

#[derive(Serialize, Debug)]
pub struct TorrentAddRequest {
    method: &'static str,
    arguments: Torrent,
}

#[derive(Deserialize, Debug)]
pub struct Session {
    arguments: SessionArguments,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SessionArguments {
    download_dir: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct TorrentAddResponse {
    result: String,
}

pub async fn add_torrent(
    client: &Client,
    url: Url,
    username: Option<String>,
    password: Option<String>,
    torrent: Torrent,
) -> Result<()> {
    let requestbuilder = client.post(url).json(&TorrentAddRequest {
        method: "torrent-add",
        arguments: torrent.clone(),
    });

    let torrent_add_response: TorrentAddResponse = match username {
        Some(username) => requestbuilder.basic_auth(username, password),
        None => requestbuilder,
    }
    .send()
    .await?
    .json()
    .await?;
    if torrent_add_response.result != "success" {
        bail!(r#"RPC responded without result "success""#);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;
    let config: Config = serde_yaml::from_str(&fs::read_to_string("config.yml")?)?;
    let username = config.username;
    let password = config.password;
    let url: Url = config
        .url
        .unwrap_or("http://localhost:9091/transmission/rpc".to_string())
        .parse()?;
    let cbuilder = reqwest::Client::builder();
    let client = if let Some(token) =
        get_csrf_token(url.clone(), username.clone(), password.clone()).await?
    {
        let mut headers = HeaderMap::new();
        log::debug!(
            "daemon has set X-Transmission-Session-Id header to {}",
            token.to_str()?.to_string()
        );
        headers.insert("X-Transmission-Session-Id", token);
        cbuilder.default_headers(headers)
    } else {
        cbuilder
    }
    .build()?;

    let requestbuilder = client.post(url.clone());
    let resp: Session = match username.clone() {
        Some(username) => requestbuilder.basic_auth(username, password.clone()),
        None => requestbuilder,
    }
    .body(r#"{"method":"session-get"}"#)
    .send()
    .await?
    .json()
    .await?;

    let client = &client;
    let url = &url;
    let username = &username;
    let password = &password;
    stream::iter(config.root.traverse(&resp.arguments.download_dir))
        .map(|torrent| async move {
            match add_torrent(
                client,
                url.clone(),
                username.clone(),
                password.clone(),
                torrent.clone(),
            )
            .await
            {
                Ok(_) => log::info!(
                    "added torrent {} to {}",
                    torrent.filename,
                    torrent.download_dir
                ),
                Err(error) => log::error!(
                    "failed to add torrent {} to {}: {}",
                    torrent.filename,
                    torrent.download_dir,
                    error
                ),
            };
        })
        .buffer_unordered(config.concurrency.unwrap_or(4))
        .collect::<Vec<()>>()
        .await;

    Ok(())
}
