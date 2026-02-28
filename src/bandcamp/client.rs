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

#[derive(Debug, Clone, Deserialize)]
struct DiscoverResponse {
    #[serde(default)]
    items: Vec<DiscoverItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct DiscoverItem {
    primary_text: Option<String>,
    secondary_text: Option<String>,
    art_id: Option<u64>,
    genre_text: Option<String>,
    band_id: Option<u64>,
    id: Option<u64>,
    #[serde(rename = "type")]
    item_type: Option<String>,
    url_hints: Option<UrlHints>,
}

#[derive(Debug, Clone, Deserialize)]
struct UrlHints {
    subdomain: Option<String>,
    slug: Option<String>,
    item_type: Option<String>,
}

impl DiscoverItem {
    fn to_album(self) -> Option<Album> {
        let hints = self.url_hints?;
        let subdomain = hints.subdomain?;
        let slug = hints.slug?;
        let type_path = match hints.item_type.as_deref().unwrap_or("a") {
            "t" => "track",
            _ => "album",
        };
        let url = format!("https://{}.bandcamp.com/{}/{}", subdomain, type_path, slug);

        Some(Album {
            title: self.primary_text.unwrap_or_default(),
            artist: self.secondary_text.unwrap_or_default(),
            art_url: self.art_id.map(art_url_thumb),
            url,
            genre: self.genre_text,
            band_id: self.band_id,
            item_id: self.id,
            item_type: self.item_type,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchAutoResponse {
    auto: SearchResponse,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResult {
    name: Option<String>,
    band_name: Option<String>,
    art_id: Option<u64>,
    item_url_path: Option<String>,
    band_id: Option<u64>,
    id: Option<u64>,
    #[serde(default)]
    tag_names: Vec<String>,
}

impl SearchResult {
    fn to_album(self) -> Option<Album> {
        let url = self.item_url_path.filter(|u| !u.is_empty())?;
        let genre = if self.tag_names.is_empty() {
            None
        } else {
            Some(self.tag_names.join(", "))
        };

        Some(Album {
            title: self.name.unwrap_or_default(),
            artist: self.band_name.unwrap_or_default(),
            art_url: self.art_id.map(art_url_thumb),
            url,
            genre,
            band_id: self.band_id,
            item_id: self.id,
            item_type: Some("a".to_string()),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TralbumResponse {
    title: Option<String>,
    tralbum_artist: Option<String>,
    art_id: Option<u64>,
    band: Option<TralbumBand>,
    #[serde(default)]
    tracks: Vec<TralbumTrack>,
}

#[derive(Debug, Clone, Deserialize)]
struct TralbumBand {
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TralbumTrack {
    title: Option<String>,
    streaming_url: Option<StreamingUrl>,
    duration: Option<f64>,
    art_id: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct StreamingUrl {
    #[serde(rename = "mp3-128")]
    mp3_128: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TralbumPageData {
    current: Option<TralbumCurrent>,
}

#[derive(Debug, Clone, Deserialize)]
struct TralbumCurrent {
    band_id: Option<u64>,
    id: Option<u64>,
    #[serde(rename = "type")]
    item_type: Option<String>,
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

        let resp: DiscoverResponse = self
            .inner
            .client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.items.into_iter().filter_map(DiscoverItem::to_album).collect())
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
        let resp: TralbumResponse = self
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

        let album_title = resp.title.unwrap_or_default();
        let artist = resp
            .tralbum_artist
            .or_else(|| resp.band.and_then(|b| b.name))
            .unwrap_or_default();

        let tracks = resp
            .tracks
            .into_iter()
            .map(|t| TrackInfo {
                title: t.title.unwrap_or_default(),
                artist: artist.clone(),
                album: album_title.clone(),
                art_url: t.art_id.or(resp.art_id).map(art_url_large),
                stream_url: t.streaming_url.and_then(|s| s.mp3_128),
                duration: t.duration,
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

        let data: TralbumPageData = serde_json::from_str(&json_str)?;
        let current = data
            .current
            .ok_or_else(|| anyhow!("No current field in tralbum"))?;
        let band_id = current
            .band_id
            .ok_or_else(|| anyhow!("No band_id in tralbum"))?;
        let tralbum_id = current
            .id
            .ok_or_else(|| anyhow!("No id in tralbum"))?;
        let tralbum_type = match current.item_type.as_deref() {
            Some("track") => "t",
            _ => "a",
        }
        .to_string();

        Ok((band_id, tralbum_type, tralbum_id))
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Album>> {
        let resp: SearchAutoResponse = self
            .inner
            .client
            .post(format!("{}/bcsearch_public_api/1/autocomplete_elastic", API_BASE))
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

        Ok(resp.auto.results.into_iter().filter_map(SearchResult::to_album).collect())
    }
}
