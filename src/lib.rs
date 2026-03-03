use extism_pdk::{http, log, plugin_fn, FnResult, HttpRequest, Json, LogLevel, WithReturnCode};
use std::collections::HashSet;

use rs_plugin_common_interfaces::{
    domain::external_images::ExternalImage,
    lookup::{
        RsLookupMatchType, RsLookupMetadataResults, RsLookupMovie, RsLookupPerson,
        RsLookupQuery, RsLookupSerie, RsLookupWrapper,
    },
    CredentialType, PluginInformation, PluginType,
};

mod convert;
mod tmdb;

use convert::{tmdb_person_to_images, tmdb_person_to_metadata, tmdb_result_to_images, tmdb_result_to_metadata};
use tmdb::{
    build_movie_detail_url, build_movie_search_url, build_person_detail_url,
    build_person_search_url, build_tv_detail_url, build_tv_search_url, parse_movie_detail_json,
    parse_movie_search_json, parse_person_detail_json, parse_person_search_json, parse_tmdb_id,
    parse_tmdb_person_id, parse_tv_detail_json, parse_tv_search_json, TmdbMediaType,
    TmdbPersonResult, TmdbResult,
};

enum LookupTarget {
    DirectMovie(u64),
    DirectTv(u64),
    DirectUnknown(u64),
    SearchMovie(String),
    SearchTv(String),
}

enum PersonLookupTarget {
    DirectPerson(u64),
    SearchPerson(String),
}

#[plugin_fn]
pub fn infos() -> FnResult<Json<PluginInformation>> {
    Ok(Json(PluginInformation {
        name: "tmdb_metadata".into(),
        capabilities: vec![PluginType::LookupMetadata],
        version: 3,
        interface_version: 1,
        repo: Some("https://github.com/flashthepublic/rs-plugin-tmdb".to_string()),
        publisher: "neckaros".into(),
        description: "Look up movie and TV show metadata from The Movie Database (TMDB)".into(),
        credential_kind: Some(CredentialType::Token),
        settings: vec![],
        ..Default::default()
    }))
}

fn build_http_request(url: String) -> HttpRequest {
    let mut request = HttpRequest {
        url,
        headers: Default::default(),
        method: Some("GET".into()),
    };

    request
        .headers
        .insert("Accept".to_string(), "application/json".to_string());
    request.headers.insert(
        "User-Agent".to_string(),
        "rs-plugin-tmdb/0.1 (+https://www.themoviedb.org)".to_string(),
    );

    request
}

const DEFAULT_API_KEY: &str = "4a01db3a73eed5cf17e9c7c27fd9d008";

fn extract_api_key(lookup: &RsLookupWrapper) -> FnResult<String> {
    if let Some(key) = lookup
        .credential
        .as_ref()
        .and_then(|c| c.password.as_deref())
        .map(str::trim)
        .filter(|k| !k.is_empty())
    {
        return Ok(key.to_string());
    }

    Ok(DEFAULT_API_KEY.to_string())
}

fn execute_json_request(url: String) -> FnResult<String> {
    let request = build_http_request(url);
    let res = http::request::<Vec<u8>>(&request, None);

    match res {
        Ok(res) if res.status_code() >= 200 && res.status_code() < 300 => {
            Ok(String::from_utf8_lossy(&res.body()).to_string())
        }
        Ok(res) => {
            log!(
                LogLevel::Error,
                "TMDB HTTP error {}: {}",
                res.status_code(),
                String::from_utf8_lossy(&res.body())
            );
            Err(WithReturnCode::new(
                extism_pdk::Error::msg(format!("HTTP error: {}", res.status_code())),
                res.status_code() as i32,
            ))
        }
        Err(e) => {
            log!(LogLevel::Error, "TMDB request failed: {}", e);
            Err(WithReturnCode(e, 500))
        }
    }
}

fn execute_movie_search_request(
    api_key: &str,
    query: &str,
    page: Option<u32>,
) -> FnResult<(Vec<TmdbResult>, Option<String>)> {
    let url = build_movie_search_url(api_key, query, page)
        .ok_or_else(|| WithReturnCode::new(extism_pdk::Error::msg("Empty search query"), 404))?;

    let body = execute_json_request(url)?;
    parse_movie_search_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB search response"),
            500,
        )
    })
}

fn execute_tv_search_request(
    api_key: &str,
    query: &str,
    page: Option<u32>,
) -> FnResult<(Vec<TmdbResult>, Option<String>)> {
    let url = build_tv_search_url(api_key, query, page)
        .ok_or_else(|| WithReturnCode::new(extism_pdk::Error::msg("Empty search query"), 404))?;

    let body = execute_json_request(url)?;
    parse_tv_search_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB TV search response"),
            500,
        )
    })
}

fn execute_movie_detail_request(api_key: &str, movie_id: u64) -> FnResult<Option<TmdbResult>> {
    let url = build_movie_detail_url(api_key, movie_id);
    let body = execute_json_request(url)?;
    Ok(parse_movie_detail_json(&body))
}

fn execute_tv_detail_request(api_key: &str, tv_id: u64) -> FnResult<Option<TmdbResult>> {
    let url = build_tv_detail_url(api_key, tv_id);
    let body = execute_json_request(url)?;
    Ok(parse_tv_detail_json(&body))
}

fn resolve_movie_lookup_target(movie: &RsLookupMovie) -> Option<LookupTarget> {
    // Check name for direct ID patterns
    if let Some(name) = movie.name.as_deref() {
        if let Some((id, media_type)) = parse_tmdb_id(name) {
            return Some(match media_type {
                Some(TmdbMediaType::Movie) => LookupTarget::DirectMovie(id),
                Some(TmdbMediaType::Tv) => LookupTarget::DirectTv(id),
                None => LookupTarget::DirectMovie(id),
            });
        }
    }

    // Check ids.tmdb
    if let Some(ids) = movie.ids.as_ref() {
        if let Some(tmdb_id) = ids.tmdb {
            return Some(LookupTarget::DirectMovie(tmdb_id));
        }

        // Check other_ids for "tmdb:12345" patterns
        if let Some(id) = ids.other_ids.as_ref().and_then(|other_ids| {
            other_ids
                .as_slice()
                .iter()
                .find_map(|value| parse_tmdb_id(value).map(|(id, mt)| (id, mt)))
        }) {
            return Some(match id.1 {
                Some(TmdbMediaType::Tv) => LookupTarget::DirectTv(id.0),
                _ => LookupTarget::DirectMovie(id.0),
            });
        }
    }

    // Fall back to name search
    movie
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| LookupTarget::SearchMovie(n.to_string()))
}

fn resolve_serie_lookup_target(serie: &RsLookupSerie) -> Option<LookupTarget> {
    // Check name for direct ID patterns
    if let Some(name) = serie.name.as_deref() {
        if let Some((id, media_type)) = parse_tmdb_id(name) {
            return Some(match media_type {
                Some(TmdbMediaType::Movie) => LookupTarget::DirectMovie(id),
                Some(TmdbMediaType::Tv) => LookupTarget::DirectTv(id),
                None => LookupTarget::DirectTv(id),
            });
        }
    }

    // Check ids.tmdb
    if let Some(ids) = serie.ids.as_ref() {
        if let Some(tmdb_id) = ids.tmdb {
            return Some(LookupTarget::DirectTv(tmdb_id));
        }

        if let Some(id) = ids.other_ids.as_ref().and_then(|other_ids| {
            other_ids
                .as_slice()
                .iter()
                .find_map(|value| parse_tmdb_id(value).map(|(id, mt)| (id, mt)))
        }) {
            return Some(match id.1 {
                Some(TmdbMediaType::Movie) => LookupTarget::DirectMovie(id.0),
                _ => LookupTarget::DirectTv(id.0),
            });
        }
    }

    // Fall back to name search
    serie
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| LookupTarget::SearchTv(n.to_string()))
}

fn resolve_person_lookup_target(person: &RsLookupPerson) -> Option<PersonLookupTarget> {
    if let Some(name) = person.name.as_deref() {
        if let Some(id) = parse_tmdb_person_id(name) {
            return Some(PersonLookupTarget::DirectPerson(id));
        }
    }

    if let Some(ids) = person.ids.as_ref() {
        if let Some(tmdb_id) = ids.tmdb {
            return Some(PersonLookupTarget::DirectPerson(tmdb_id));
        }
    }

    person
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| PersonLookupTarget::SearchPerson(n.to_string()))
}

fn execute_person_detail_request(
    api_key: &str,
    person_id: u64,
) -> FnResult<Option<TmdbPersonResult>> {
    let url = build_person_detail_url(api_key, person_id);
    let body = execute_json_request(url)?;
    Ok(parse_person_detail_json(&body))
}

fn execute_person_search_request(
    api_key: &str,
    query: &str,
    page: Option<u32>,
) -> FnResult<(Vec<TmdbPersonResult>, Option<String>)> {
    let url = build_person_search_url(api_key, query, page)
        .ok_or_else(|| WithReturnCode::new(extism_pdk::Error::msg("Empty search query"), 404))?;

    let body = execute_json_request(url)?;
    parse_person_search_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB person search response"),
            500,
        )
    })
}

fn lookup_tmdb(
    lookup: &RsLookupWrapper,
    api_key: &str,
) -> FnResult<(Vec<TmdbResult>, Option<String>, Option<RsLookupMatchType>)> {
    match &lookup.query {
        RsLookupQuery::Movie(movie) => {
            let page = movie
                .page_key
                .as_deref()
                .and_then(|k| k.parse::<u32>().ok());

            match resolve_movie_lookup_target(movie) {
                Some(LookupTarget::DirectMovie(id)) => {
                    let result = execute_movie_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(LookupTarget::DirectTv(id)) => {
                    let result = execute_tv_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(LookupTarget::DirectUnknown(id)) => {
                    // Try movie first, then TV
                    if let Ok(Some(result)) = execute_movie_detail_request(api_key, id) {
                        return Ok((vec![result], None, Some(RsLookupMatchType::ExactId)));
                    }
                    let result = execute_tv_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(LookupTarget::SearchMovie(query)) => {
                    let (results, next_page_key) =
                        execute_movie_search_request(api_key, &query, page)?;
                    Ok((results, next_page_key, None))
                }
                Some(LookupTarget::SearchTv(query)) => {
                    let (results, next_page_key) =
                        execute_tv_search_request(api_key, &query, page)?;
                    Ok((results, next_page_key, None))
                }
                None => Err(WithReturnCode::new(
                    extism_pdk::Error::msg("Empty movie query"),
                    404,
                )),
            }
        }
        RsLookupQuery::Serie(serie) => {
            let page = serie
                .page_key
                .as_deref()
                .and_then(|k| k.parse::<u32>().ok());

            match resolve_serie_lookup_target(serie) {
                Some(LookupTarget::DirectTv(id)) => {
                    let result = execute_tv_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(LookupTarget::DirectMovie(id)) => {
                    let result = execute_movie_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(LookupTarget::DirectUnknown(id)) => {
                    // Try TV first, then movie
                    if let Ok(Some(result)) = execute_tv_detail_request(api_key, id) {
                        return Ok((vec![result], None, Some(RsLookupMatchType::ExactId)));
                    }
                    let result = execute_movie_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(LookupTarget::SearchTv(query)) => {
                    let (results, next_page_key) =
                        execute_tv_search_request(api_key, &query, page)?;
                    Ok((results, next_page_key, None))
                }
                Some(LookupTarget::SearchMovie(query)) => {
                    let (results, next_page_key) =
                        execute_movie_search_request(api_key, &query, page)?;
                    Ok((results, next_page_key, None))
                }
                None => Err(WithReturnCode::new(
                    extism_pdk::Error::msg("Empty serie query"),
                    404,
                )),
            }
        }
        _ => Ok((vec![], None, None)),
    }
}

fn lookup_tmdb_person(
    lookup: &RsLookupWrapper,
    api_key: &str,
) -> FnResult<(Vec<TmdbPersonResult>, Option<String>, Option<RsLookupMatchType>)> {
    match &lookup.query {
        RsLookupQuery::Person(person) => {
            let page = person
                .page_key
                .as_deref()
                .and_then(|k| k.parse::<u32>().ok());

            match resolve_person_lookup_target(person) {
                Some(PersonLookupTarget::DirectPerson(id)) => {
                    let result = execute_person_detail_request(api_key, id)?;
                    Ok((
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    ))
                }
                Some(PersonLookupTarget::SearchPerson(query)) => {
                    let (results, next_page_key) =
                        execute_person_search_request(api_key, &query, page)?;
                    Ok((results, next_page_key, None))
                }
                None => Err(WithReturnCode::new(
                    extism_pdk::Error::msg("Empty person query"),
                    404,
                )),
            }
        }
        _ => Ok((vec![], None, None)),
    }
}

#[plugin_fn]
pub fn lookup_metadata(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<RsLookupMetadataResults>> {
    let api_key = extract_api_key(&lookup)?;

    if matches!(&lookup.query, RsLookupQuery::Person(_)) {
        let (results, next_page_key, match_type) = lookup_tmdb_person(&lookup, &api_key)?;
        let results = results
            .into_iter()
            .map(|r| {
                let mut result = tmdb_person_to_metadata(r);
                result.match_type = match_type.clone();
                result
            })
            .collect();
        return Ok(Json(RsLookupMetadataResults {
            results,
            next_page_key,
        }));
    }

    let (results, next_page_key, match_type) = lookup_tmdb(&lookup, &api_key)?;

    let results = results
        .into_iter()
        .map(|r| {
            let mut result = tmdb_result_to_metadata(r);
            result.match_type = match_type.clone();
            result
        })
        .collect();

    Ok(Json(RsLookupMetadataResults {
        results,
        next_page_key,
    }))
}

#[plugin_fn]
pub fn lookup_metadata_images(
    Json(lookup): Json<RsLookupWrapper>,
) -> FnResult<Json<Vec<ExternalImage>>> {
    let api_key = extract_api_key(&lookup)?;

    if matches!(&lookup.query, RsLookupQuery::Person(_)) {
        let (results, _, match_type) = lookup_tmdb_person(&lookup, &api_key)?;
        let images: Vec<ExternalImage> = results
            .iter()
            .flat_map(tmdb_person_to_images)
            .map(|mut img| {
                img.match_type = match_type.clone();
                img
            })
            .collect();
        return Ok(Json(deduplicate_images(images)));
    }

    let (results, _, match_type) = lookup_tmdb(&lookup, &api_key)?;

    let images: Vec<ExternalImage> = results
        .iter()
        .flat_map(tmdb_result_to_images)
        .map(|mut img| {
            img.match_type = match_type.clone();
            img
        })
        .collect();

    Ok(Json(deduplicate_images(images)))
}

fn deduplicate_images(images: Vec<ExternalImage>) -> Vec<ExternalImage> {
    let mut seen_urls = HashSet::new();
    let mut deduped = Vec::new();

    for image in images {
        if seen_urls.insert(image.url.url.clone()) {
            deduped.push(image);
        }
    }

    deduped
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_plugin_common_interfaces::domain::rs_ids::RsIds;

    #[test]
    fn lookup_non_movie_serie_query_returns_empty() {
        let lookup = RsLookupWrapper {
            query: RsLookupQuery::Book(Default::default()),
            credential: Some(rs_plugin_common_interfaces::PluginCredential {
                kind: CredentialType::Token,
                password: Some("test_key".to_string()),
                ..Default::default()
            }),
            params: None,
        };

        let (results, _, match_type) =
            lookup_tmdb(&lookup, "test_key").expect("lookup should succeed");
        assert!(results.is_empty());
        assert!(match_type.is_none());
    }

    #[test]
    fn extract_api_key_missing_returns_default() {
        let lookup = RsLookupWrapper {
            query: RsLookupQuery::Movie(Default::default()),
            credential: None,
            params: None,
        };

        let key = extract_api_key(&lookup).expect("should return default key");
        assert_eq!(key, DEFAULT_API_KEY);
    }

    #[test]
    fn extract_api_key_empty_returns_default() {
        let lookup = RsLookupWrapper {
            query: RsLookupQuery::Movie(Default::default()),
            credential: Some(rs_plugin_common_interfaces::PluginCredential {
                kind: CredentialType::Token,
                password: Some("  ".to_string()),
                ..Default::default()
            }),
            params: None,
        };

        let key = extract_api_key(&lookup).expect("should return default key");
        assert_eq!(key, DEFAULT_API_KEY);
    }

    #[test]
    fn extract_api_key_present() {
        let lookup = RsLookupWrapper {
            query: RsLookupQuery::Movie(Default::default()),
            credential: Some(rs_plugin_common_interfaces::PluginCredential {
                kind: CredentialType::Token,
                password: Some("my_api_key".to_string()),
                ..Default::default()
            }),
            params: None,
        };

        let key = extract_api_key(&lookup).expect("should extract key");
        assert_eq!(key, "my_api_key");
    }

    #[test]
    fn resolve_movie_target_prefers_direct_id_in_name() {
        let movie = RsLookupMovie {
            name: Some("tmdb:550".to_string()),
            ids: None,
            page_key: None,
        };

        let target = resolve_movie_lookup_target(&movie);
        match target {
            Some(LookupTarget::DirectMovie(id)) => assert_eq!(id, 550),
            _ => panic!("Expected DirectMovie target"),
        }
    }

    #[test]
    fn resolve_movie_target_reads_ids_tmdb() {
        let movie = RsLookupMovie {
            name: Some("some name".to_string()),
            ids: Some(RsIds {
                tmdb: Some(550),
                ..Default::default()
            }),
            page_key: None,
        };

        let target = resolve_movie_lookup_target(&movie);
        match target {
            Some(LookupTarget::DirectMovie(id)) => assert_eq!(id, 550),
            _ => panic!("Expected DirectMovie from ids.tmdb"),
        }
    }

    #[test]
    fn resolve_movie_target_reads_other_ids() {
        let movie = RsLookupMovie {
            name: Some("ignored".to_string()),
            ids: Some(RsIds {
                other_ids: Some(vec!["tmdb-movie:550".to_string()].into()),
                ..Default::default()
            }),
            page_key: None,
        };

        let target = resolve_movie_lookup_target(&movie);
        match target {
            Some(LookupTarget::DirectMovie(id)) => assert_eq!(id, 550),
            _ => panic!("Expected DirectMovie from other_ids"),
        }
    }

    #[test]
    fn resolve_movie_target_falls_back_to_search() {
        let movie = RsLookupMovie {
            name: Some("Fight Club".to_string()),
            ids: None,
            page_key: None,
        };

        let target = resolve_movie_lookup_target(&movie);
        match target {
            Some(LookupTarget::SearchMovie(q)) => assert_eq!(q, "Fight Club"),
            _ => panic!("Expected SearchMovie target"),
        }
    }

    #[test]
    fn resolve_movie_target_empty_name_returns_none() {
        let movie = RsLookupMovie {
            name: Some(String::new()),
            ids: None,
            page_key: None,
        };

        assert!(resolve_movie_lookup_target(&movie).is_none());
    }

    #[test]
    fn resolve_serie_target_prefers_direct_id() {
        let serie = RsLookupSerie {
            name: Some("tmdb-tv:1396".to_string()),
            ids: None,
            page_key: None,
        };

        let target = resolve_serie_lookup_target(&serie);
        match target {
            Some(LookupTarget::DirectTv(id)) => assert_eq!(id, 1396),
            _ => panic!("Expected DirectTv target"),
        }
    }

    #[test]
    fn resolve_serie_target_tmdb_prefix_defaults_to_tv() {
        let serie = RsLookupSerie {
            name: Some("tmdb:1396".to_string()),
            ids: None,
            page_key: None,
        };

        let target = resolve_serie_lookup_target(&serie);
        match target {
            Some(LookupTarget::DirectTv(id)) => assert_eq!(id, 1396),
            _ => panic!("Expected DirectTv for tmdb: prefix in serie context"),
        }
    }

    #[test]
    fn resolve_serie_target_falls_back_to_search() {
        let serie = RsLookupSerie {
            name: Some("Breaking Bad".to_string()),
            ids: None,
            page_key: None,
        };

        let target = resolve_serie_lookup_target(&serie);
        match target {
            Some(LookupTarget::SearchTv(q)) => assert_eq!(q, "Breaking Bad"),
            _ => panic!("Expected SearchTv target"),
        }
    }

    #[test]
    fn resolve_movie_target_url_format() {
        let movie = RsLookupMovie {
            name: Some("https://www.themoviedb.org/movie/550-fight-club".to_string()),
            ids: None,
            page_key: None,
        };

        let target = resolve_movie_lookup_target(&movie);
        match target {
            Some(LookupTarget::DirectMovie(id)) => assert_eq!(id, 550),
            _ => panic!("Expected DirectMovie from URL"),
        }
    }

    #[test]
    fn deduplicate_images_by_url() {
        let images = vec![
            ExternalImage {
                url: rs_plugin_common_interfaces::RsRequest {
                    url: "https://a.com/1.jpg".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ExternalImage {
                url: rs_plugin_common_interfaces::RsRequest {
                    url: "https://a.com/1.jpg".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        ];

        let deduped = deduplicate_images(images);
        assert_eq!(deduped.len(), 1);
    }
}
