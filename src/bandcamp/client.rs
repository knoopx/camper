use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::sync::Arc;

use super::types::*;

const API_BASE: &str = "https://bandcamp.com/api";

fn art_url(art_id: u64) -> String {
    format!("https://f4.bcbits.com/img/a{}_10.jpg", art_id)
}

#[derive(Debug, Clone, Deserialize)]
struct CollectionSummaryResponse {
    collection_summary: Option<CollectionSummaryData>,
}

#[derive(Debug, Clone, Deserialize)]
struct CollectionSummaryData {
    fan_id: u64,
    username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CollectionResponse {
    items: Vec<CollectionItemData>,
    more_available: bool,
    last_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CollectionItemData {
    item_title: Option<String>,
    band_name: Option<String>,
    item_art_id: Option<u64>,
    item_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TralbumData {
    current: Option<TralbumCurrent>,
    trackinfo: Option<Vec<TralbumTrack>>,
    art_id: Option<u64>,
    artist: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TralbumCurrent {
    title: Option<String>,
    artist: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TralbumTrack {
    title: Option<String>,
    duration: Option<f64>,
    file: Option<TralbumFile>,
}

#[derive(Debug, Deserialize)]
struct TralbumFile {
    #[serde(rename = "mp3-128")]
    mp3_128: Option<String>,
}

#[derive(Debug)]
struct ClientInner {
    client: Client,
    cookies: String,
    fan: FanInfo,
}

#[derive(Clone, Debug)]
pub struct BandcampClient {
    inner: Arc<ClientInner>,
}

unsafe impl Send for BandcampClient {}
unsafe impl Sync for BandcampClient {}

impl BandcampClient {
    pub async fn new(cookies: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0")
            .build()?;

        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, HeaderValue::from_str(&cookies)?);

        let resp: CollectionSummaryResponse = client
            .get(format!("{}/fan/2/collection_summary", API_BASE))
            .headers(headers)
            .send()
            .await?
            .json()
            .await?;

        let summary = resp
            .collection_summary
            .ok_or_else(|| anyhow!("Not authenticated"))?;

        let fan = FanInfo {
            fan_id: summary.fan_id,
            username: summary.username.unwrap_or_default(),
        };

        Ok(Self {
            inner: Arc::new(ClientInner {
                client,
                cookies,
                fan,
            }),
        })
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(cookie) = HeaderValue::from_str(&self.inner.cookies) {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    pub fn fan(&self) -> &FanInfo {
        &self.inner.fan
    }

    pub async fn discover(&self, params: &DiscoverParams) -> Result<Vec<Album>> {
        let url = format!(
            "{}/discover/3/get_web?g={}&s={}&p={}&gn={}&f={}&w=0&lo=0",
            API_BASE, params.genre, params.sort, params.page, params.subgenre, params.format
        );

        let json: serde_json::Value = self
            .inner
            .client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let items = json
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(items
            .into_iter()
            .filter_map(|item| {
                let title = item
                    .get("primary_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let artist = item
                    .get("secondary_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let art_id = item.get("art_id").and_then(|v| v.as_u64());
                let genre = item
                    .get("genre_text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let album_url = item.get("url_hints").and_then(|hints| {
                    let subdomain = hints.get("subdomain")?.as_str()?;
                    let slug = hints.get("slug")?.as_str()?;
                    let itype = hints
                        .get("item_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("a");
                    let type_path = match itype {
                        "a" => "album",
                        "t" => "track",
                        _ => "album",
                    };
                    Some(format!(
                        "https://{}.bandcamp.com/{}/{}",
                        subdomain, type_path, slug
                    ))
                })?;

                Some(Album {
                    title,
                    artist,
                    art_url: art_id.map(art_url),
                    url: album_url,
                    genre,
                })
            })
            .collect())
    }

    pub async fn get_collection(&self) -> Result<Vec<CollectionItem>> {
        self.fetch_items(&format!("{}/fancollection/1/collection_items", API_BASE))
            .await
    }

    pub async fn get_wishlist(&self) -> Result<Vec<CollectionItem>> {
        self.fetch_items(&format!("{}/fancollection/1/wishlist_items", API_BASE))
            .await
    }

    async fn fetch_items(&self, url: &str) -> Result<Vec<CollectionItem>> {
        let fan_id = self.inner.fan.fan_id;
        let mut all_items = Vec::new();
        let mut token = format!(
            "{}::a::",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );

        loop {
            let resp: CollectionResponse = self
                .inner
                .client
                .post(url)
                .headers(self.headers())
                .json(&serde_json::json!({
                    "fan_id": fan_id,
                    "older_than_token": token,
                    "count": 50
                }))
                .send()
                .await?
                .json()
                .await?;

            for item in resp.items {
                all_items.push(CollectionItem {
                    title: item.item_title.unwrap_or_default(),
                    artist: item.band_name.unwrap_or_default(),
                    art_url: item.item_art_id.map(art_url),
                    url: item.item_url.unwrap_or_default(),
                });
            }

            if !resp.more_available {
                break;
            }

            token = resp.last_token.ok_or_else(|| anyhow!("Missing token"))?;
        }

        Ok(all_items)
    }

    pub async fn get_album_details(&self, album_url: &str) -> Result<AlbumDetails> {
        let html = self
            .inner
            .client
            .get(album_url)
            .headers(self.headers())
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_document(&html);
        let selector = Selector::parse("script[data-tralbum]").unwrap();

        let tralbum_json = document
            .select(&selector)
            .next()
            .and_then(|el| el.value().attr("data-tralbum"))
            .ok_or_else(|| anyhow!("No tralbum data found"))?;

        let data: TralbumData = serde_json::from_str(tralbum_json)?;
        let current = data
            .current
            .ok_or_else(|| anyhow!("No current album data"))?;

        let artist = current.artist.or(data.artist).unwrap_or_default();
        let album_title = current.title.unwrap_or_default();
        let art_url_val = data.art_id.map(art_url);

        let tracks = data
            .trackinfo
            .unwrap_or_default()
            .into_iter()
            .map(|t| TrackInfo {
                title: t.title.unwrap_or_default(),
                artist: artist.clone(),
                album: album_title.clone(),
                art_url: art_url_val.clone(),
                stream_url: t.file.and_then(|f| f.mp3_128),
                duration: t.duration,
            })
            .collect();

        Ok(AlbumDetails {
            url: album_url.to_string(),
            tracks,
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Album>> {
        let json: serde_json::Value = self
            .inner
            .client
            .post("https://bandcamp.com/api/bcsearch_public_api/1/autocomplete_elastic")
            .json(&serde_json::json!({
                "search_text": query,
                "search_filter": "a",
                "full_page": true,
                "fan_id": self.inner.fan.fan_id,
            }))
            .send()
            .await?
            .json()
            .await?;

        let results = json
            .get("auto")
            .and_then(|v| v.get("results"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(results
            .into_iter()
            .filter_map(|item| {
                let title = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let artist = item.get("band_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let art_id = item.get("art_id").and_then(|v| v.as_u64());
                let url = item.get("item_url_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let genre = item.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string());

                Some(Album {
                    title,
                    artist,
                    art_url: art_id.map(art_url),
                    url,
                    genre,
                })
            })
            .collect())
    }


}
