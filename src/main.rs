use anyhow::{bail, Context, Result};
use base64::prelude::*;
use futures::{stream, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, RequestBuilder, StatusCode,
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};
use url::Url;

pub mod session;
pub mod torrent;

#[derive(Debug, Deserialize)]
pub struct Entry {
    #[serde(default)]
    torrents: Vec<String>,
    children: Option<Subdirectories>,
}
#[derive(Debug, Deserialize)]
pub struct Subdirectories(BTreeMap<String, Entry>);

#[derive(Debug, Deserialize, Clone)]
pub struct Authentication {
    username: Option<String>,
    password: Option<String>,
}

impl Authentication {
    fn apply(&self, rb: RequestBuilder) -> RequestBuilder {
        match self {
            Authentication {
                username: Some(username),
                password: Some(password),
            } => rb.basic_auth(username, Some(password)),
            _ => rb,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    url: Option<String>,
    #[serde(flatten)]
    auth: Authentication,
    concurrency: Option<usize>,
    root: Entry,
}

#[derive(Clone)]
pub struct Schema {
    filename: String,
    download_dir: String,
}

impl Subdirectories {
    fn write_traversal_to_vec(&self, download_dir: &Path, list: &mut Vec<Schema>) {
        for (directory, entry) in &self.0 {
            let mut download_dir = download_dir.to_path_buf();
            download_dir.push(directory);

            entry.write_traversal_to_vec(&download_dir, list);
        }
    }
}

impl Entry {
    fn write_traversal_to_vec(&self, download_dir: &Path, list: &mut Vec<Schema>) {
        list.extend(self.torrents.iter().map(|slug| Schema {
            filename: slug.clone(),
            download_dir: download_dir.to_string_lossy().to_string(),
        }));
        if let Some(children) = &self.children {
            children.write_traversal_to_vec(download_dir, list);
        }
    }

    fn traverse(&self, download_dir: &Path) -> Vec<Schema> {
        let mut list = vec![];
        self.write_traversal_to_vec(download_dir, &mut list);
        list
    }
}

async fn get_csrf_token(url: Url, auth: Authentication) -> Result<Option<HeaderValue>> {
    let resp = auth.apply(reqwest::Client::new().get(url)).send().await?;
    match (
        resp.status(),
        resp.headers().get("X-Transmission-Session-Id"),
    ) {
        (StatusCode::CONFLICT, Some(id)) => Ok(Some(id.to_owned())),
        _ => Ok(None),
    }
}

#[derive(Deserialize, Debug)]
pub struct TorrentAddResponse {
    result: String,
}

impl From<Schema> for torrent::Torrent {
    fn from(value: Schema) -> Self {
        match Url::parse(&value.filename) {
            // technically a url, not a filepath
            Ok(_) => Self::File {
                filename: value.filename,
                download_dir: value.download_dir,
            },
            Err(_) => match fs::read(&value.filename) {
                Ok(s) => Self::Metainfo {
                    metainfo: BASE64_STANDARD.encode(s),
                    download_dir: value.download_dir,
                },
                // the real case where we pass a filepath
                Err(_) => Self::File {
                    filename: value.filename,
                    download_dir: value.download_dir,
                },
            },
        }
    }
}

pub async fn add_torrent(
    client: &Client,
    url: Url,
    auth: Authentication,
    torrent: Schema,
) -> Result<()> {
    let response: TorrentAddResponse = auth
        .apply(client.post(url).json(&session::Request {
            method: "torrent-add",
            arguments: torrent.into(),
        }))
        .send()
        .await?
        .json()
        .await?;
    if response.result != "success" {
        bail!("RPC responded with result: {}", response.result);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;
    let config: Config = serde_yaml::from_str(
        &fs::read_to_string("config.yml").context("Failed to read config file `config.yml`")?,
    )?;
    let url: Url = config
        .url
        .unwrap_or("http://localhost:9091/transmission/rpc".to_string())
        .parse()?;
    let cbuilder = reqwest::Client::builder();
    let client = if let Some(token) = get_csrf_token(url.clone(), config.auth.clone()).await? {
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

    let session: session::Session = config
        .auth
        .apply(client.post(url.clone()).body(r#"{"method":"session-get"}"#))
        .send()
        .await?
        .json()
        .await?;

    let client = &client;
    let url = &url;
    let auth = &config.auth;
    stream::iter(config.root.traverse(&session.arguments.download_dir))
        .map(|torrent| async move {
            match add_torrent(client, url.clone(), auth.clone(), torrent.clone()).await {
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
        .buffer_unordered(config.concurrency.filter(|&c| c != 0).unwrap_or(4))
        .collect::<Vec<()>>()
        .await;

    Ok(())
}
