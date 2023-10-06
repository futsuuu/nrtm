use std::{fs, path::PathBuf};

use anyhow::Context as _;
use once_cell::sync::Lazy;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client,
};
use serde::Deserialize;

use crate::CACHE_DIR;

static API_VERSION: &str = "2022-11-28";
static USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
static CACHE_FILE: Lazy<PathBuf> = Lazy::new(|| CACHE_DIR.join("request_releases.json"));

pub async fn update_request_cache() -> anyhow::Result<String> {
    let request_url = "https://api.github.com/repos/neovim/neovim/releases";

    let json = new_client()?
        .get(request_url)
        .header(header::CONTENT_TYPE, "application/json")
        .send()
        .await?
        .text()
        .await?;

    fs::write(&*CACHE_FILE, &json)?;

    Ok(json)
}

pub async fn get_releases() -> anyhow::Result<Vec<Release>> {
    let json = if CACHE_FILE.exists() {
        fs::read_to_string(&*CACHE_FILE)?
    } else {
        update_request_cache().await?
    };

    let releases = serde_json::from_str(&json)?;

    Ok(releases)
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Release {
    pub tag_name: String,
    pub body: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn get_nvim_version(&self) -> anyhow::Result<String> {
        let version = self
            .body
            .split_once("NVIM v")
            .context("Failed to get Neovim version froma release note.")?
            .1
            .split_whitespace()
            .next()
            .unwrap();

        Ok(version.to_string())
    }
}

#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub content_type: String,
    pub browser_download_url: String,
}

fn new_client() -> anyhow::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static(API_VERSION),
    );

    let client = Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()?;

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_nvim_version() {
        let mut release = Release::default();
        release.body = "foo\n```\nNVIM v1.0.0-12-234 (bar)\n".into();
        let version = release.get_nvim_version();
        assert!(version.is_ok());
        assert_eq!(version.unwrap(), "1.0.0-12-234".to_string());
    }
}
