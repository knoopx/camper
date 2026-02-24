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
    pub subgenre: u32,
    pub sort: String,
    pub format: String,
    pub page: u32,
}

impl Default for DiscoverParams {
    fn default() -> Self {
        Self {
            genre: "all".to_string(),
            subgenre: 0,
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

/// Subgenres grouped by parent genre slug. Each entry is (id, label).
pub const SUBGENRES: &[(&str, &[(u32, &str)])] = &[
    ("acoustic", &[
        (1001, "Folk"), (1002, "Singer-Songwriter"), (1003, "Rock"), (1004, "Pop"),
        (1005, "Guitar"), (1006, "Americana"), (1007, "Electro-Acoustic"),
        (1008, "Instrumental"), (1009, "Piano"), (1010, "Bluegrass"), (1011, "Roots"),
    ]),
    ("alternative", &[
        (1012, "Indie Rock"), (1013, "Industrial"), (1014, "Shoegaze"), (1015, "Grunge"),
        (1016, "Goth"), (1017, "Dream Pop"), (1018, "Emo"), (1019, "Math Rock"),
        (1020, "Britpop"), (1021, "Jangle Pop"),
    ]),
    ("ambient", &[
        (1022, "Chill-Out"), (1023, "Drone"), (1024, "Dark Ambient"), (1025, "Electronic"),
        (1026, "Soundscapes"), (1027, "Field Recordings"), (1028, "Atmospheric"),
        (1029, "Meditation"), (1030, "Noise"), (1031, "New Age"), (1032, "IDM"),
        (1033, "Industrial"),
    ]),
    ("blues", &[
        (1034, "Rhythm & Blues"), (1035, "Blues Rock"), (1036, "Country Blues"),
        (1037, "Boogie-Woogie"), (1038, "Delta Blues"), (1039, "Americana"),
        (1040, "Electric Blues"), (1041, "Gospel"), (1042, "Bluegrass"),
    ]),
    ("classical", &[
        (1043, "Orchestral"), (1044, "Neo-Classical"), (1045, "Chamber Music"),
        (1046, "Classical Piano"), (1047, "Contemporary Classical"), (1048, "Baroque"),
        (1049, "Opera"), (1050, "Choral"), (1051, "Modern Classical"), (1052, "Avant Garde"),
    ]),
    ("comedy", &[(1053, "Improv"), (1054, "Stand-Up")]),
    ("country", &[
        (1055, "Bluegrass"), (1056, "Country Rock"), (1057, "Americana"),
        (1058, "Country Folk"), (1059, "Alt-Country"), (1060, "Country Blues"),
        (1061, "Western"), (1062, "Singer-Songwriter"), (1063, "Outlaw"),
        (1064, "Honky-Tonk"), (1065, "Roots"), (1066, "Hillbilly"),
    ]),
    ("devotional", &[
        (1067, "Christian"), (1068, "Gospel"), (1069, "Meditation"),
        (1070, "Spiritual"), (1071, "Worship"), (1072, "Inspirational"),
    ]),
    ("electronic", &[
        (1073, "House"), (1074, "Electronica"), (1075, "Downtempo"), (1076, "Techno"),
        (1077, "Electro"), (1078, "Dubstep"), (1079, "Beats"), (1080, "Dance"),
        (1081, "IDM"), (1082, "Drum & Bass"), (1083, "Breaks"), (1084, "Trance"),
        (1085, "Glitch"), (1086, "Chiptune"), (1087, "Chillwave"), (1088, "Dub"),
        (1089, "EDM"), (1090, "Instrumental"), (1091, "Witch House"), (1092, "Garage"),
        (1093, "Juke"), (1094, "Footwork"), (1095, "Vaporwave"), (1096, "Synthwave"),
    ]),
    ("experimental", &[
        (1097, "Noise"), (1098, "Drone"), (1099, "Avant Garde"),
        (1100, "Experimental Rock"), (1101, "Improvisation"), (1102, "Sound Art"),
        (1103, "Musique Concrète"),
    ]),
    ("folk", &[
        (1104, "Singer-Songwriter"), (1105, "Folk Rock"), (1106, "Indie Folk"),
        (1107, "Pop Folk"), (1108, "Traditional"), (1109, "Experimental Folk"),
        (1110, "Roots"),
    ]),
    ("funk", &[
        (1111, "Funk Jam"), (1112, "Deep Funk"), (1113, "Funk Rock"),
        (1114, "Jazz Funk"), (1115, "Boogie"), (1116, "G-Funk"),
        (1117, "Rare Groove"), (1118, "Electro"), (1119, "Go-Go"),
    ]),
    ("hip-hop-rap", &[
        (1120, "Rap"), (1121, "Underground Hip-Hop"), (1122, "Instrumental Hip-Hop"),
        (1123, "Trap"), (1124, "Conscious Hip-Hop"), (1125, "Boom-Bap"),
        (1126, "Beat-Tape"), (1127, "Hardcore"), (1128, "Grime"),
    ]),
    ("jazz", &[
        (1129, "Fusion"), (1130, "Big Band"), (1131, "Nu Jazz"),
        (1132, "Modern Jazz"), (1133, "Swing"), (1134, "Free Jazz"),
        (1135, "Soul Jazz"), (1136, "Latin Jazz"), (1137, "Vocal Jazz"),
        (1138, "Bebop"), (1139, "Spiritual Jazz"),
    ]),
    ("kids", &[
        (1140, "Family Music"), (1141, "Educational"), (1142, "Music Therapy"),
        (1143, "Lullaby"), (1144, "Baby"),
    ]),
    ("latin", &[
        (1145, "Brazilian"), (1146, "Cumbia"), (1147, "Tango"),
        (1148, "Latin Rock"), (1149, "Flamenco"), (1150, "Salsa"),
        (1151, "Reggaeton"), (1152, "Merengue"), (1153, "Bolero"),
        (1154, "México D.F."), (1155, "Bachata"),
    ]),
    ("metal", &[
        (1156, "Hardcore"), (1157, "Black Metal"), (1158, "Death Metal"),
        (1159, "Thrash Metal"), (1160, "Grindcore"), (1161, "Doom"),
        (1162, "Post Hardcore"), (1163, "Progressive Metal"), (1164, "Metalcore"),
        (1165, "Sludge Metal"), (1166, "Heavy Metal"), (1167, "Deathcore"), (1168, "Noise"),
    ]),
    ("pop", &[
        (1169, "Indie Pop"), (1170, "Synth Pop"), (1171, "Power Pop"),
        (1172, "New Wave"), (1173, "Dream Pop"), (1174, "Noise Pop"),
        (1175, "Experimental Pop"), (1176, "Electro Pop"), (1177, "Adult Contemporary"),
        (1178, "Jangle Pop"), (1179, "J-Pop"),
    ]),
    ("punk", &[
        (1180, "Hardcore Punk"), (1181, "Garage"), (1182, "Pop Punk"),
        (1183, "Punk Rock"), (1184, "Post-Punk"), (1185, "Post-Hardcore"),
        (1186, "Thrash"), (1187, "Crust Punk"), (1188, "Folk Punk"),
        (1189, "Emo"), (1190, "Ska"), (1191, "No Wave"),
    ]),
    ("r-b-soul", &[
        (1192, "Soul"), (1193, "R&B"), (1194, "Neo-Soul"),
        (1195, "Gospel"), (1196, "Contemporary R&B"), (1197, "Motown"), (1198, "Urban"),
    ]),
    ("reggae", &[
        (1199, "Dub"), (1200, "Ska"), (1201, "Roots"), (1202, "Dancehall"),
        (1203, "Rocksteady"), (1204, "Ragga"), (1205, "Lovers Rock"),
    ]),
    ("rock", &[
        (1206, "Indie"), (1207, "Prog Rock"), (1208, "Post-Rock"),
        (1209, "Rock & Roll"), (1210, "Psychedelic Rock"), (1211, "Hard Rock"),
        (1212, "Garage Rock"), (1213, "Surf Rock"), (1214, "Instrumental"),
        (1215, "Math Rock"), (1216, "Rockabilly"),
    ]),
    ("soundtrack", &[
        (1217, "Film Music"), (1218, "Video Game Music"), (1219, "Video Game"), (1220, "OST"),
    ]),
    ("spoken-word", &[
        (1221, "Poetry"), (1222, "Inspirational"), (1223, "Storytelling"), (1224, "Self-Help"),
    ]),
    ("world", &[
        (1225, "Latin"), (1226, "Roots"), (1227, "African"), (1228, "Tropical"),
        (1229, "Tribal"), (1230, "Brazilian"), (1231, "Celtic"), (1232, "World Fusion"),
        (1233, "Cumbia"), (1234, "Gypsy"), (1235, "New Age"), (1236, "Balkan"),
        (1237, "Reggaeton"),
    ]),
];

pub fn subgenres_for(genre: &str) -> &'static [(u32, &'static str)] {
    SUBGENRES.iter()
        .find(|(slug, _)| *slug == genre)
        .map(|(_, subs)| *subs)
        .unwrap_or(&[])
}
