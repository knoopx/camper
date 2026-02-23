#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub art_url: Option<String>,
    pub url: String,
    pub genre: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CollectionItem {
    pub title: String,
    pub artist: String,
    pub art_url: Option<String>,
    pub url: String,
    pub is_wishlist: bool,
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
    pub sort: String,
    pub format: String,
    pub page: u32,
}

impl Default for DiscoverParams {
    fn default() -> Self {
        Self {
            genre: "all".to_string(),
            sort: "new".to_string(),
            format: "all".to_string(),
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
    ("classical", "Classical"),
    ("reggae", "Reggae"),
    ("country", "Country"),
    ("blues", "Blues"),
    ("latin", "Latin"),
];

pub const SORT_OPTIONS: &[(&str, &str)] = &[
    ("new", "New Arrivals"),
    ("rec", "Recommended"),
    ("pop", "Best Sellers"),
];

pub const FORMAT_OPTIONS: &[(&str, &str)] = &[
    ("all", "Any Format"),
    ("digital", "Digital"),
    ("vinyl", "Vinyl"),
    ("cd", "CD"),
    ("cassette", "Cassette"),
];
