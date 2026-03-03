use regex::Regex;
use serde::Deserialize;

pub const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/";
pub const TMDB_IMAGE_SIZE_ORIGINAL: &str = "original";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmdbMediaType {
    Movie,
    Tv,
}

#[derive(Debug, Clone, Default)]
pub struct TmdbResult {
    pub media_type: Option<TmdbMediaType>,
    pub id: u64,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub original_language: Option<String>,
    pub genre_ids: Vec<u32>,
    pub popularity: Option<f64>,

    // Detail-only fields
    pub imdb_id: Option<String>,
    pub runtime: Option<u32>,
    pub status: Option<String>,
    pub tagline: Option<String>,
    pub genres: Vec<TmdbGenre>,
    pub cast: Vec<TmdbCastMember>,
    pub crew: Vec<TmdbCrewMember>,
    pub images: TmdbImages,

    // TV-specific
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct TmdbGenre {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TmdbCastMember {
    pub id: u64,
    pub name: String,
    pub character: Option<String>,
    pub profile_path: Option<String>,
    pub order: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TmdbCrewMember {
    pub id: u64,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TmdbImages {
    #[serde(default)]
    pub posters: Vec<TmdbImage>,
    #[serde(default)]
    pub backdrops: Vec<TmdbImage>,
    #[serde(default)]
    pub logos: Vec<TmdbImage>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TmdbImage {
    pub file_path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub aspect_ratio: Option<f64>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub iso_639_1: Option<String>,
}

// --- Raw API response structs ---

#[derive(Debug, Deserialize)]
pub struct TmdbSearchResponse {
    pub page: u32,
    pub total_pages: u32,
    pub total_results: u32,
    pub results: Vec<TmdbSearchItem>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbSearchItem {
    pub id: u64,
    pub title: Option<String>,
    pub name: Option<String>,
    pub original_title: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub original_language: Option<String>,
    pub genre_ids: Option<Vec<u32>>,
    pub popularity: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbMovieDetail {
    pub id: u64,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub original_language: Option<String>,
    pub genres: Option<Vec<TmdbGenre>>,
    pub runtime: Option<u32>,
    pub status: Option<String>,
    pub tagline: Option<String>,
    pub imdb_id: Option<String>,
    pub popularity: Option<f64>,
    pub credits: Option<TmdbCredits>,
    pub images: Option<TmdbImagesResponse>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbTvDetail {
    pub id: u64,
    pub name: String,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub original_language: Option<String>,
    pub genres: Option<Vec<TmdbGenre>>,
    pub episode_run_time: Option<Vec<u32>>,
    pub status: Option<String>,
    pub tagline: Option<String>,
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
    pub popularity: Option<f64>,
    pub credits: Option<TmdbCredits>,
    pub images: Option<TmdbImagesResponse>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TmdbCredits {
    pub cast: Option<Vec<TmdbCastMember>>,
    pub crew: Option<Vec<TmdbCrewMember>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TmdbImagesResponse {
    pub posters: Option<Vec<TmdbImage>>,
    pub backdrops: Option<Vec<TmdbImage>>,
    pub logos: Option<Vec<TmdbImage>>,
}

// --- Person structs ---

#[derive(Debug, Deserialize)]
pub struct TmdbPersonDetail {
    pub id: u64,
    pub name: String,
    pub also_known_as: Option<Vec<String>>,
    pub biography: Option<String>,
    pub birthday: Option<String>,
    pub deathday: Option<String>,
    pub gender: Option<u8>,
    pub imdb_id: Option<String>,
    pub known_for_department: Option<String>,
    pub place_of_birth: Option<String>,
    pub profile_path: Option<String>,
    pub popularity: Option<f64>,
    pub images: Option<TmdbPersonImages>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TmdbPersonImages {
    pub profiles: Option<Vec<TmdbImage>>,
}

#[derive(Debug, Clone, Default)]
pub struct TmdbPersonResult {
    pub id: u64,
    pub name: String,
    pub also_known_as: Vec<String>,
    pub biography: Option<String>,
    pub birthday: Option<String>,
    pub deathday: Option<String>,
    pub gender: Option<u8>,
    pub imdb_id: Option<String>,
    pub known_for_department: Option<String>,
    pub place_of_birth: Option<String>,
    pub profile_path: Option<String>,
    pub images: Vec<TmdbImage>,
}

// --- Public functions ---

pub fn build_image_url(file_path: &str, size: &str) -> String {
    format!("{TMDB_IMAGE_BASE}{size}{file_path}")
}

pub fn build_movie_search_url(api_key: &str, query: &str, page: Option<u32>) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    let encoded = encode_query_component(trimmed);
    let page_num = page.unwrap_or(1);
    Some(format!(
        "https://api.themoviedb.org/3/search/movie?api_key={api_key}&query={encoded}&page={page_num}"
    ))
}

pub fn build_tv_search_url(api_key: &str, query: &str, page: Option<u32>) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    let encoded = encode_query_component(trimmed);
    let page_num = page.unwrap_or(1);
    Some(format!(
        "https://api.themoviedb.org/3/search/tv?api_key={api_key}&query={encoded}&page={page_num}"
    ))
}

pub fn build_movie_detail_url(api_key: &str, movie_id: u64) -> String {
    format!(
        "https://api.themoviedb.org/3/movie/{movie_id}?api_key={api_key}&append_to_response=credits,images"
    )
}

pub fn build_tv_detail_url(api_key: &str, tv_id: u64) -> String {
    format!(
        "https://api.themoviedb.org/3/tv/{tv_id}?api_key={api_key}&append_to_response=credits,images"
    )
}

pub fn build_person_detail_url(api_key: &str, person_id: u64) -> String {
    format!(
        "https://api.themoviedb.org/3/person/{person_id}?api_key={api_key}&append_to_response=images"
    )
}

pub fn build_person_search_url(api_key: &str, query: &str, page: Option<u32>) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    let encoded = encode_query_component(trimmed);
    let page_num = page.unwrap_or(1);
    Some(format!(
        "https://api.themoviedb.org/3/search/person?api_key={api_key}&query={encoded}&page={page_num}"
    ))
}

pub fn parse_person_detail_json(json: &str) -> Option<TmdbPersonResult> {
    let detail: TmdbPersonDetail = serde_json::from_str(json).ok()?;
    Some(person_detail_to_result(detail))
}

pub fn parse_person_search_json(json: &str) -> Option<(Vec<TmdbPersonResult>, Option<String>)> {
    let response: TmdbSearchResponse = serde_json::from_str(json).ok()?;
    let next_page_key = if response.page < response.total_pages {
        Some((response.page + 1).to_string())
    } else {
        None
    };
    let results = response
        .results
        .into_iter()
        .map(|item| TmdbPersonResult {
            id: item.id,
            name: item.name.unwrap_or_else(|| item.title.unwrap_or_default()),
            profile_path: item.poster_path,
            ..Default::default()
        })
        .collect();
    Some((results, next_page_key))
}

pub fn parse_movie_search_json(json: &str) -> Option<(Vec<TmdbResult>, Option<String>)> {
    let response: TmdbSearchResponse = serde_json::from_str(json).ok()?;
    let next_page_key = if response.page < response.total_pages {
        Some((response.page + 1).to_string())
    } else {
        None
    };
    let results = response
        .results
        .into_iter()
        .map(|item| search_item_to_result(item, TmdbMediaType::Movie))
        .collect();
    Some((results, next_page_key))
}

pub fn parse_tv_search_json(json: &str) -> Option<(Vec<TmdbResult>, Option<String>)> {
    let response: TmdbSearchResponse = serde_json::from_str(json).ok()?;
    let next_page_key = if response.page < response.total_pages {
        Some((response.page + 1).to_string())
    } else {
        None
    };
    let results = response
        .results
        .into_iter()
        .map(|item| search_item_to_result(item, TmdbMediaType::Tv))
        .collect();
    Some((results, next_page_key))
}

pub fn parse_movie_detail_json(json: &str) -> Option<TmdbResult> {
    let detail: TmdbMovieDetail = serde_json::from_str(json).ok()?;
    Some(movie_detail_to_result(detail))
}

pub fn parse_tv_detail_json(json: &str) -> Option<TmdbResult> {
    let detail: TmdbTvDetail = serde_json::from_str(json).ok()?;
    Some(tv_detail_to_result(detail))
}

pub fn parse_tmdb_id(value: &str) -> Option<(u64, Option<TmdbMediaType>)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();

    // tmdb-movie:550
    if let Some(id_str) = lower.strip_prefix("tmdb-movie:") {
        return id_str.parse::<u64>().ok().map(|id| (id, Some(TmdbMediaType::Movie)));
    }

    // tmdb-tv:1396
    if let Some(id_str) = lower.strip_prefix("tmdb-tv:") {
        return id_str.parse::<u64>().ok().map(|id| (id, Some(TmdbMediaType::Tv)));
    }

    // tmdb:550
    if let Some(id_str) = lower.strip_prefix("tmdb:") {
        return id_str.parse::<u64>().ok().map(|id| (id, None));
    }

    // URL format: https://www.themoviedb.org/movie/550-fight-club or /tv/1396-breaking-bad
    let re = Regex::new(
        r"(?i)(?:https?://)?(?:www\.)?themoviedb\.org/(movie|tv)/(\d+)"
    ).ok()?;

    if let Some(caps) = re.captures(trimmed) {
        let media_type = match caps.get(1)?.as_str().to_ascii_lowercase().as_str() {
            "movie" => Some(TmdbMediaType::Movie),
            "tv" => Some(TmdbMediaType::Tv),
            _ => None,
        };
        let id = caps.get(2)?.as_str().parse::<u64>().ok()?;
        return Some((id, media_type));
    }

    None
}

pub fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for b in value.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }

    encoded
}

// --- Internal conversion helpers ---

fn search_item_to_result(item: TmdbSearchItem, media_type: TmdbMediaType) -> TmdbResult {
    let title = match &media_type {
        TmdbMediaType::Movie => item.title.unwrap_or_default(),
        TmdbMediaType::Tv => item.name.unwrap_or_default(),
    };
    let original_title = match &media_type {
        TmdbMediaType::Movie => item.original_title,
        TmdbMediaType::Tv => item.original_name,
    };
    let release_date = match &media_type {
        TmdbMediaType::Movie => item.release_date,
        TmdbMediaType::Tv => item.first_air_date,
    };

    TmdbResult {
        media_type: Some(media_type),
        id: item.id,
        title,
        original_title,
        overview: item.overview,
        release_date,
        poster_path: item.poster_path,
        backdrop_path: item.backdrop_path,
        vote_average: item.vote_average,
        vote_count: item.vote_count,
        original_language: item.original_language,
        genre_ids: item.genre_ids.unwrap_or_default(),
        popularity: item.popularity,
        ..Default::default()
    }
}

fn movie_detail_to_result(detail: TmdbMovieDetail) -> TmdbResult {
    let credits = detail.credits.unwrap_or_default();
    let images_resp = detail.images.unwrap_or_default();

    TmdbResult {
        media_type: Some(TmdbMediaType::Movie),
        id: detail.id,
        title: detail.title,
        original_title: detail.original_title,
        overview: detail.overview,
        release_date: detail.release_date,
        poster_path: detail.poster_path,
        backdrop_path: detail.backdrop_path,
        vote_average: detail.vote_average,
        vote_count: detail.vote_count,
        original_language: detail.original_language,
        genres: detail.genres.unwrap_or_default(),
        runtime: detail.runtime,
        status: detail.status,
        tagline: detail.tagline,
        imdb_id: detail.imdb_id,
        popularity: detail.popularity,
        cast: credits.cast.unwrap_or_default(),
        crew: credits.crew.unwrap_or_default(),
        images: TmdbImages {
            posters: images_resp.posters.unwrap_or_default(),
            backdrops: images_resp.backdrops.unwrap_or_default(),
            logos: images_resp.logos.unwrap_or_default(),
        },
        ..Default::default()
    }
}

fn tv_detail_to_result(detail: TmdbTvDetail) -> TmdbResult {
    let credits = detail.credits.unwrap_or_default();
    let images_resp = detail.images.unwrap_or_default();
    let runtime = detail
        .episode_run_time
        .as_ref()
        .and_then(|v| v.first().copied());

    TmdbResult {
        media_type: Some(TmdbMediaType::Tv),
        id: detail.id,
        title: detail.name,
        original_title: detail.original_name,
        overview: detail.overview,
        release_date: detail.first_air_date,
        poster_path: detail.poster_path,
        backdrop_path: detail.backdrop_path,
        vote_average: detail.vote_average,
        vote_count: detail.vote_count,
        original_language: detail.original_language,
        genres: detail.genres.unwrap_or_default(),
        runtime,
        status: detail.status,
        tagline: detail.tagline,
        popularity: detail.popularity,
        number_of_seasons: detail.number_of_seasons,
        number_of_episodes: detail.number_of_episodes,
        cast: credits.cast.unwrap_or_default(),
        crew: credits.crew.unwrap_or_default(),
        images: TmdbImages {
            posters: images_resp.posters.unwrap_or_default(),
            backdrops: images_resp.backdrops.unwrap_or_default(),
            logos: images_resp.logos.unwrap_or_default(),
        },
        ..Default::default()
    }
}

fn person_detail_to_result(detail: TmdbPersonDetail) -> TmdbPersonResult {
    let images_resp = detail.images.unwrap_or_default();

    TmdbPersonResult {
        id: detail.id,
        name: detail.name,
        also_known_as: detail.also_known_as.unwrap_or_default(),
        biography: detail.biography,
        birthday: detail.birthday,
        deathday: detail.deathday,
        gender: detail.gender,
        imdb_id: detail.imdb_id,
        known_for_department: detail.known_for_department,
        place_of_birth: detail.place_of_birth,
        profile_path: detail.profile_path,
        images: images_resp.profiles.unwrap_or_default(),
    }
}

/// Parse a TMDB person ID from a string. Accepts:
/// - `tmdb-person:5719226` → Some(5719226)
/// - `tmdb:5719226` → Some(5719226) (generic, used in person context)
pub fn parse_tmdb_person_id(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();

    if let Some(id_str) = lower.strip_prefix("tmdb-person:") {
        return id_str.parse::<u64>().ok();
    }

    if let Some(id_str) = lower.strip_prefix("tmdb:") {
        return id_str.parse::<u64>().ok();
    }

    // URL format: https://www.themoviedb.org/person/5719226
    let re = Regex::new(r"(?i)(?:https?://)?(?:www\.)?themoviedb\.org/person/(\d+)").ok()?;
    if let Some(caps) = re.captures(trimmed) {
        return caps.get(1)?.as_str().parse::<u64>().ok();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_movie_search_url_basic() {
        let url = build_movie_search_url("test_key", "Fight Club", None).expect("url");
        assert_eq!(
            url,
            "https://api.themoviedb.org/3/search/movie?api_key=test_key&query=Fight+Club&page=1"
        );
    }

    #[test]
    fn build_movie_search_url_with_page() {
        let url = build_movie_search_url("test_key", "Fight Club", Some(3)).expect("url");
        assert_eq!(
            url,
            "https://api.themoviedb.org/3/search/movie?api_key=test_key&query=Fight+Club&page=3"
        );
    }

    #[test]
    fn build_movie_search_url_empty_returns_none() {
        assert!(build_movie_search_url("key", "", None).is_none());
        assert!(build_movie_search_url("key", "  ", None).is_none());
    }

    #[test]
    fn build_tv_search_url_basic() {
        let url = build_tv_search_url("test_key", "Breaking Bad", None).expect("url");
        assert_eq!(
            url,
            "https://api.themoviedb.org/3/search/tv?api_key=test_key&query=Breaking+Bad&page=1"
        );
    }

    #[test]
    fn build_movie_detail_url_basic() {
        let url = build_movie_detail_url("test_key", 550);
        assert_eq!(
            url,
            "https://api.themoviedb.org/3/movie/550?api_key=test_key&append_to_response=credits,images"
        );
    }

    #[test]
    fn build_tv_detail_url_basic() {
        let url = build_tv_detail_url("test_key", 1396);
        assert_eq!(
            url,
            "https://api.themoviedb.org/3/tv/1396?api_key=test_key&append_to_response=credits,images"
        );
    }

    #[test]
    fn build_image_url_basic() {
        assert_eq!(
            build_image_url("/abc123.jpg", "original"),
            "https://image.tmdb.org/t/p/original/abc123.jpg"
        );
        assert_eq!(
            build_image_url("/abc123.jpg", "w500"),
            "https://image.tmdb.org/t/p/w500/abc123.jpg"
        );
    }

    #[test]
    fn parse_tmdb_id_prefix_format() {
        assert_eq!(parse_tmdb_id("tmdb:550"), Some((550, None)));
        assert_eq!(
            parse_tmdb_id("tmdb-movie:550"),
            Some((550, Some(TmdbMediaType::Movie)))
        );
        assert_eq!(
            parse_tmdb_id("tmdb-tv:1396"),
            Some((1396, Some(TmdbMediaType::Tv)))
        );
    }

    #[test]
    fn parse_tmdb_id_url_format() {
        assert_eq!(
            parse_tmdb_id("https://www.themoviedb.org/movie/550-fight-club"),
            Some((550, Some(TmdbMediaType::Movie)))
        );
        assert_eq!(
            parse_tmdb_id("https://www.themoviedb.org/tv/1396-breaking-bad"),
            Some((1396, Some(TmdbMediaType::Tv)))
        );
        assert_eq!(
            parse_tmdb_id("https://themoviedb.org/movie/550"),
            Some((550, Some(TmdbMediaType::Movie)))
        );
    }

    #[test]
    fn parse_tmdb_id_invalid() {
        assert_eq!(parse_tmdb_id(""), None);
        assert_eq!(parse_tmdb_id("some random text"), None);
        assert_eq!(parse_tmdb_id("tmdb:abc"), None);
    }

    #[test]
    fn parse_tmdb_id_case_insensitive() {
        assert_eq!(parse_tmdb_id("TMDB:550"), Some((550, None)));
        assert_eq!(
            parse_tmdb_id("TMDB-MOVIE:550"),
            Some((550, Some(TmdbMediaType::Movie)))
        );
    }

    #[test]
    fn parse_movie_search_json_basic() {
        let json = r#"{
            "page": 1,
            "total_pages": 3,
            "total_results": 50,
            "results": [
                {
                    "id": 550,
                    "title": "Fight Club",
                    "original_title": "Fight Club",
                    "overview": "A ticking-Loss.",
                    "release_date": "1999-10-15",
                    "poster_path": "/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg",
                    "backdrop_path": "/hZkgoQYus5dXo3H8T7Uef6DNknx.jpg",
                    "vote_average": 8.4,
                    "vote_count": 26000,
                    "original_language": "en",
                    "genre_ids": [18, 53, 35],
                    "popularity": 60.0
                }
            ]
        }"#;

        let (results, next_page) = parse_movie_search_json(json).expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 550);
        assert_eq!(results[0].title, "Fight Club");
        assert_eq!(results[0].media_type, Some(TmdbMediaType::Movie));
        assert_eq!(results[0].poster_path, Some("/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg".to_string()));
        assert_eq!(results[0].genre_ids, vec![18, 53, 35]);
        assert_eq!(next_page, Some("2".to_string()));
    }

    #[test]
    fn parse_movie_search_json_last_page() {
        let json = r#"{
            "page": 3,
            "total_pages": 3,
            "total_results": 50,
            "results": []
        }"#;

        let (results, next_page) = parse_movie_search_json(json).expect("parse");
        assert!(results.is_empty());
        assert_eq!(next_page, None);
    }

    #[test]
    fn parse_tv_search_json_basic() {
        let json = r#"{
            "page": 1,
            "total_pages": 1,
            "total_results": 1,
            "results": [
                {
                    "id": 1396,
                    "name": "Breaking Bad",
                    "original_name": "Breaking Bad",
                    "overview": "A chemistry teacher.",
                    "first_air_date": "2008-01-20",
                    "poster_path": "/poster.jpg",
                    "backdrop_path": "/backdrop.jpg",
                    "vote_average": 8.9,
                    "vote_count": 12000,
                    "original_language": "en",
                    "genre_ids": [18, 80],
                    "popularity": 100.0
                }
            ]
        }"#;

        let (results, next_page) = parse_tv_search_json(json).expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1396);
        assert_eq!(results[0].title, "Breaking Bad");
        assert_eq!(results[0].media_type, Some(TmdbMediaType::Tv));
        assert_eq!(results[0].release_date, Some("2008-01-20".to_string()));
        assert_eq!(next_page, None);
    }

    #[test]
    fn parse_movie_detail_json_basic() {
        let json = r#"{
            "id": 550,
            "title": "Fight Club",
            "original_title": "Fight Club",
            "overview": "An insomniac.",
            "release_date": "1999-10-15",
            "poster_path": "/poster.jpg",
            "backdrop_path": "/backdrop.jpg",
            "vote_average": 8.4,
            "vote_count": 26000,
            "original_language": "en",
            "genres": [{"id": 18, "name": "Drama"}, {"id": 53, "name": "Thriller"}],
            "runtime": 139,
            "status": "Released",
            "tagline": "Mischief. Mayhem. Soap.",
            "imdb_id": "tt0137523",
            "popularity": 60.0,
            "credits": {
                "cast": [
                    {"id": 819, "name": "Edward Norton", "character": "The Narrator", "profile_path": "/norton.jpg", "order": 0},
                    {"id": 287, "name": "Brad Pitt", "character": "Tyler Durden", "profile_path": "/pitt.jpg", "order": 1}
                ],
                "crew": [
                    {"id": 7467, "name": "David Fincher", "job": "Director", "department": "Directing", "profile_path": "/fincher.jpg"}
                ]
            },
            "images": {
                "posters": [{"file_path": "/poster1.jpg", "width": 500, "height": 750}],
                "backdrops": [{"file_path": "/bg1.jpg", "width": 1920, "height": 1080}],
                "logos": []
            }
        }"#;

        let result = parse_movie_detail_json(json).expect("parse");
        assert_eq!(result.id, 550);
        assert_eq!(result.title, "Fight Club");
        assert_eq!(result.media_type, Some(TmdbMediaType::Movie));
        assert_eq!(result.imdb_id, Some("tt0137523".to_string()));
        assert_eq!(result.runtime, Some(139));
        assert_eq!(result.status, Some("Released".to_string()));
        assert_eq!(result.genres.len(), 2);
        assert_eq!(result.genres[0].name, "Drama");
        assert_eq!(result.cast.len(), 2);
        assert_eq!(result.cast[0].name, "Edward Norton");
        assert_eq!(result.crew.len(), 1);
        assert_eq!(result.crew[0].name, "David Fincher");
        assert_eq!(result.images.posters.len(), 1);
        assert_eq!(result.images.backdrops.len(), 1);
    }

    #[test]
    fn parse_tv_detail_json_basic() {
        let json = r#"{
            "id": 1396,
            "name": "Breaking Bad",
            "original_name": "Breaking Bad",
            "overview": "A chemistry teacher.",
            "first_air_date": "2008-01-20",
            "poster_path": "/poster.jpg",
            "backdrop_path": "/backdrop.jpg",
            "vote_average": 8.9,
            "vote_count": 12000,
            "original_language": "en",
            "genres": [{"id": 18, "name": "Drama"}, {"id": 80, "name": "Crime"}],
            "episode_run_time": [45, 47],
            "status": "Ended",
            "tagline": "Remember my name.",
            "number_of_seasons": 5,
            "number_of_episodes": 62,
            "popularity": 100.0,
            "credits": {
                "cast": [
                    {"id": 17419, "name": "Bryan Cranston", "character": "Walter White", "profile_path": "/cranston.jpg", "order": 0}
                ],
                "crew": [
                    {"id": 66633, "name": "Vince Gilligan", "job": "Creator", "department": "Production", "profile_path": "/gilligan.jpg"}
                ]
            },
            "images": {
                "posters": [{"file_path": "/tvposter.jpg", "width": 500, "height": 750}],
                "backdrops": [],
                "logos": []
            }
        }"#;

        let result = parse_tv_detail_json(json).expect("parse");
        assert_eq!(result.id, 1396);
        assert_eq!(result.title, "Breaking Bad");
        assert_eq!(result.media_type, Some(TmdbMediaType::Tv));
        assert_eq!(result.number_of_seasons, Some(5));
        assert_eq!(result.number_of_episodes, Some(62));
        assert_eq!(result.runtime, Some(45));
        assert_eq!(result.status, Some("Ended".to_string()));
        assert_eq!(result.cast.len(), 1);
        assert_eq!(result.cast[0].name, "Bryan Cranston");
    }

    #[test]
    fn encode_query_component_basic() {
        assert_eq!(encode_query_component("Fight Club"), "Fight+Club");
        assert_eq!(encode_query_component("hello world"), "hello+world");
        assert_eq!(encode_query_component("test&value=1"), "test%26value%3D1");
    }
}
