use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use super::types::*;

const API_BASE: &str = "https://bandcamp.com/api";

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
        let mut url = format!(
            "{}/discover/2/get?g={}&s={}&p={}&f=all&w=0",
            API_BASE, params.genre, params.sort, params.page
        );
        if !params.tag.is_empty() {
            url.push_str(&format!("&t={}", params.tag));
        }

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
                let band_id = item.get("band_id").and_then(|v| v.as_u64());
                let item_id = item.get("id").and_then(|v| v.as_u64());
                let item_type = item
                    .get("type")
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
                    art_url: art_id.map(art_url_thumb),
                    url: album_url,
                    genre,
                    band_id,
                    item_id,
                    item_type,
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
                    art_url: item.item_art_id.map(art_url_thumb),
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
        let (band_id, tralbum_type, tralbum_id) = self.resolve_tralbum(album_url).await?;
        self.get_album_details_by_id(band_id, &tralbum_type, tralbum_id, album_url)
            .await
    }

    pub async fn get_album_details_by_id(
        &self,
        band_id: u64,
        tralbum_type: &str,
        tralbum_id: u64,
        album_url: &str,
    ) -> Result<AlbumDetails> {
        let resp: serde_json::Value = self
            .inner
            .client
            .post(format!("{}/mobile/24/tralbum_details", API_BASE))
            .json(&serde_json::json!({
                "band_id": band_id,
                "tralbum_type": tralbum_type,
                "tralbum_id": tralbum_id
            }))
            .send()
            .await?
            .json()
            .await?;

        let album_title = resp
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let artist = resp
            .get("tralbum_artist")
            .or_else(|| resp.get("band").and_then(|b| b.get("name")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let album_art_id = resp.get("art_id").and_then(|v| v.as_u64());

        let tracks = resp
            .get("tracks")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let track_title = t
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let stream_url = t
                    .get("streaming_url")
                    .and_then(|s| s.get("mp3-128"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let duration = t.get("duration").and_then(|v| v.as_f64());
                let track_art_id = t.get("art_id").and_then(|v| v.as_u64());
                let art = track_art_id.or(album_art_id).map(art_url_large);

                TrackInfo {
                    title: track_title,
                    artist: artist.clone(),
                    album: album_title.clone(),
                    art_url: art,
                    stream_url,
                    duration,
                }
            })
            .collect();

        Ok(AlbumDetails {
            url: album_url.to_string(),
            tracks,
        })
    }

    async fn resolve_tralbum(&self, url: &str) -> Result<(u64, String, u64)> {
        let html = self
            .inner
            .client
            .get(url)
            .headers(self.headers())
            .send()
            .await?
            .text()
            .await?;

        let marker = "data-tralbum=\"";
        let start = html
            .find(marker)
            .ok_or_else(|| anyhow!("No tralbum data found on page"))?
            + marker.len();
        let end = html[start..]
            .find('"')
            .ok_or_else(|| anyhow!("Malformed tralbum data"))?
            + start;
        let escaped = &html[start..end];
        let json_str = escaped
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">");

        let data: serde_json::Value = serde_json::from_str(&json_str)?;

        let current = data
            .get("current")
            .ok_or_else(|| anyhow!("No current field in tralbum"))?;
        let band_id = current
            .get("band_id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("No band_id in tralbum"))?;
        let tralbum_id = current
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("No id in tralbum"))?;
        let item_type = current
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("album");
        let tralbum_type = match item_type {
            "track" => "t",
            _ => "a",
        }
        .to_string();

        Ok((band_id, tralbum_type, tralbum_id))
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Album>> {
        let json: serde_json::Value = self
            .inner
            .client
            .get(format!("{}/fuzzysearch/1/app_autocomplete", API_BASE))
            .query(&[("q", query), ("param_with_locations", "true")])
            .send()
            .await?
            .json()
            .await?;

        let results = json
            .get("results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(results
            .into_iter()
            .filter_map(|item| {
                let result_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match result_type {
                    "a" | "t" => {
                        let title = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let artist = item
                            .get("band_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let art_id = item.get("art_id").and_then(|v| v.as_u64());
                        let url = item
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let genre = item
                            .get("genre_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let band_id = item.get("band_id").and_then(|v| v.as_u64());
                        let item_id = item.get("id").and_then(|v| v.as_u64());

                        if url.is_empty() {
                            return None;
                        }

                        Some(Album {
                            title,
                            artist,
                            art_url: art_id.map(art_url_thumb),
                            url,
                            genre,
                            band_id,
                            item_id,
                            item_type: Some(result_type.to_string()),
                        })
                    }
                    "b" => {
                        let name = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let img_id = item.get("img_id").and_then(|v| v.as_u64());
                        let url = item
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let location = item
                            .get("location")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let genre = item
                            .get("genre_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        if url.is_empty() {
                            return None;
                        }

                        let art_url =
                            img_id.map(|id| format!("https://f4.bcbits.com/img/{:010}_23.jpg", id));

                        Some(Album {
                            title: name,
                            artist: location.unwrap_or_default(),
                            art_url,
                            url,
                            genre,
                            band_id: None,
                            item_id: None,
                            item_type: Some("b".to_string()),
                        })
                    }
                    _ => None,
                }
            })
            .collect())
    }

}
