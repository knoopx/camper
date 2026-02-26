#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub art_url: Option<String>,
    pub url: String,
    pub genre: Option<String>,
    pub band_id: Option<u64>,
    pub item_id: Option<u64>,
    pub item_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CollectionItem {
    pub title: String,
    pub artist: String,
    pub art_url: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct FanInfo {
    pub fan_id: u64,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub stream_url: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AlbumDetails {
    pub url: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone)]
pub struct DiscoverParams {
    pub genre: String,
    pub tag: String,
    pub sort: String,
    pub page: u32,
}

impl Default for DiscoverParams {
    fn default() -> Self {
        Self {
            genre: "all".to_string(),
            tag: String::new(),
            sort: "new".to_string(),
            page: 0,
        }
    }
}

pub const GENRES: &[(&str, &str)] = &[
    ("all", "All"),
    ("electronic", "Electronic"),
    ("rock", "Rock"),
    ("metal", "Metal"),
    ("alternative", "Alternative"),
    ("hip-hop-rap", "Hip-Hop/Rap"),
    ("experimental", "Experimental"),
    ("punk", "Punk"),
    ("folk", "Folk"),
    ("pop", "Pop"),
    ("ambient", "Ambient"),
    ("soundtrack", "Soundtrack"),
    ("world", "World"),
    ("jazz", "Jazz"),
    ("acoustic", "Acoustic"),
    ("funk", "Funk"),
    ("r-b-soul", "R&B/Soul"),
    ("devotional", "Devotional"),
    ("classical", "Classical"),
    ("reggae", "Reggae"),
    ("podcasts", "Podcasts"),
    ("country", "Country"),
    ("spoken-word", "Spoken Word"),
    ("comedy", "Comedy"),
    ("blues", "Blues"),
    ("kids", "Kids"),
    ("audiobooks", "Audiobooks"),
    ("latin", "Latin"),
];

pub const SORT_OPTIONS: &[(&str, &str)] = &[
    ("new", "New Arrivals"),
    ("rec", "Recommended"),
    ("top", "Best Sellers"),
];

/// Build an image URL from an art_id using the given format ID.
/// Format 10 = 350px (grid thumbnails), Format 5 = 700px (player art).
pub fn art_url(art_id: u64, format_id: u32) -> String {
    format!("https://f4.bcbits.com/img/a{:010}_{}.jpg", art_id, format_id)
}

/// 350px thumbnail for grid cards.
pub fn art_url_thumb(art_id: u64) -> String {
    art_url(art_id, 10)
}

/// 700px image for player / detail views.
pub fn art_url_large(art_id: u64) -> String {
    art_url(art_id, 5)
}
