use std::{fs, path::PathBuf};

use anyhow::Context as _;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client,
};
use semver::Version;
use serde::Deserialize;

use crate::CACHE_DIR;

static API_VERSION: &str = "2022-11-28";
static USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
static CACHE_FILE: Lazy<PathBuf> = Lazy::new(|| CACHE_DIR.join("releases.json"));

pub async fn cache_response() -> anyhow::Result<String> {
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
        cache_response().await?
    };

    let releases = serde_json::from_str(&json)?;

    Ok(releases)
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Release {
    pub name: String,
    pub tag_name: String,
    pub body: String,
    pub assets: Vec<Asset>,
    pub html_url: String,
}

impl Release {
    pub fn get_nvim_version(&self) -> anyhow::Result<Version> {
        match get_nvim_version(&self.body) {
            Ok(v) => Ok(v),
            Err(_) => get_nvim_version(&self.name),
        }
    }

    pub fn filter_assets(&self) -> Option<&Asset> {
        let invalid_os = [
            #[cfg(not(target_os = "linux"))]
            "linux",
            #[cfg(not(target_os = "macos"))]
            "macos",
            #[cfg(not(target_os = "windows"))]
            "win",
        ];
        let invalid_pointer_width = [
            #[cfg(not(target_pointer_width = "32"))]
            "32",
            #[cfg(not(target_pointer_width = "64"))]
            "64",
        ];

        'asset: for asset in &self.assets {
            if asset.get_type().is_none() {
                continue;
            }
            for os_name in &invalid_os {
                if asset.name.contains(os_name) {
                    continue 'asset;
                }
            }
            for pointer_width in &invalid_pointer_width {
                if asset.name.contains(pointer_width) {
                    continue 'asset;
                }
            }

            return Some(asset);
        }

        None
    }
}

fn get_nvim_version(text: &str) -> anyhow::Result<Version> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(^|\W)(?i)nvim(?-i) v?(?<version>\d+\.\d+\.\d+(-\S+)*(\+\S+)*)(\W|$)",
        )
        .unwrap()
    });

    let caps = RE
        .captures(text)
        .with_context(|| format!("Failed to get Neovim version from text: {text}"))?;
    let raw_version = caps.name("version").map_or("", |m| m.as_str());

    let version = if let Some((version_prerelease, build_metadata)) =
        raw_version.rsplit_once('-')
    {
        Version::parse(&format!("{version_prerelease}+{build_metadata}"))?
    } else {
        Version::parse(raw_version)?
    };

    Ok(version)
}

#[test]
fn get_nvim_version_t() {
    let version = get_nvim_version("NVIM v1.0.0");
    assert_eq!(version.unwrap(), Version::parse("1.0.0").unwrap());
    let version = get_nvim_version("foo\n```\nNvim v1.0.0-dev-1234 (bar)\n");
    assert_eq!(version.unwrap(), Version::parse("1.0.0-dev+1234").unwrap());
}

#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub content_type: String,
    pub browser_download_url: String,
}

impl Asset {
    pub fn get_type(&self) -> Option<AssetType> {
        match self.content_type.as_str() {
            "application/x-zip-compressed" | "application/zip" => Some(AssetType::Zip),
            "application/x-gtar" | "application/x-gzip" | "application/gzip" => {
                Some(AssetType::TarGz)
            }
            _ => None,
        }
    }
}

pub enum AssetType {
    Zip,
    TarGz,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use AssetType::*;
        match *self {
            Zip => write!(f, "zip")?,
            TarGz => write!(f, "tar.gz")?,
        }
        Ok(())
    }
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
