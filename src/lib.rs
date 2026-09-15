use extism_pdk::{http, log, plugin_fn, FnResult, HttpRequest, Json, LogLevel, WithReturnCode};
use std::collections::HashSet;

use rs_plugin_common_interfaces::{
    domain::{external_images::ExternalImage, person::PersonType, rs_ids::RsIds},
    lookup::{
        RsLookupMatchType, RsLookupMetadataResults, RsLookupMovie, RsLookupPerson,
        RsLookupPersonFilter, RsLookupQuery, RsLookupSerie, RsLookupSerieFilter, RsLookupTagFilter,
        RsLookupWrapper,
    },
    CredentialType, PluginInformation, PluginType,
};

mod convert;
mod tmdb;

use convert::{
    tmdb_episode_stills_to_images, tmdb_episode_to_metadata, tmdb_person_to_images,
    tmdb_person_to_metadata, tmdb_result_to_images, tmdb_result_to_metadata,
};
use tmdb::{
    build_collection_detail_url, build_collection_search_url, build_episode_images_url,
    build_genre_list_url, build_movie_detail_url, build_movie_discover_url, build_movie_search_url,
    build_person_credits_url, build_person_detail_url, build_person_search_url,
    build_tv_detail_url, build_tv_discover_url, build_tv_search_url, build_tv_season_detail_url,
    credit_item_to_result, parse_collection_detail_json, parse_collection_search_json,
    parse_episode_images_json, parse_genre_list_json, parse_movie_detail_json,
    parse_movie_search_json, parse_person_credits_json, parse_person_detail_json,
    parse_person_search_json, parse_tmdb_id, parse_tmdb_person_id, parse_tv_detail_json,
    parse_tv_search_json, parse_tv_season_detail_json, TmdbCreditItem, TmdbMediaType,
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
        version: env!("CARGO_PKG_VERSION_MINOR").parse()?,
        interface_version: 1,
        repo: Some("https://github.com/neckaros/rs-plugin-tmdb".to_string()),
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

fn execute_discover_request(
    api_key: &str,
    media_type: &TmdbMediaType,
    genre_ids: &[u64],
    page: Option<u32>,
) -> FnResult<(Vec<TmdbResult>, Option<String>)> {
    let url = match media_type {
        TmdbMediaType::Movie => build_movie_discover_url(api_key, genre_ids, page),
        TmdbMediaType::Tv => build_tv_discover_url(api_key, genre_ids, page),
    };
    let body = execute_json_request(url)?;
    match media_type {
        TmdbMediaType::Movie => parse_movie_search_json(&body),
        TmdbMediaType::Tv => parse_tv_search_json(&body),
    }
    .ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB discover response"),
            500,
        )
    })
}

fn execute_genre_list_request(
    api_key: &str,
    media_type: &TmdbMediaType,
) -> FnResult<Vec<tmdb::TmdbGenre>> {
    let body = execute_json_request(build_genre_list_url(api_key, media_type))?;
    parse_genre_list_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB genre list"),
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

fn execute_episode_images_request(
    api_key: &str,
    tv_id: u64,
    season: u32,
    episode: u32,
) -> FnResult<Vec<tmdb::TmdbImage>> {
    let url = build_episode_images_url(api_key, tv_id, season, episode);
    let body = execute_json_request(url)?;
    parse_episode_images_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB episode images response"),
            500,
        )
    })
}

fn execute_tv_season_detail_request(
    api_key: &str,
    tv_id: u64,
    season: u32,
) -> FnResult<Vec<tmdb::TmdbEpisodeResult>> {
    let url = build_tv_season_detail_url(api_key, tv_id, season);
    let body = execute_json_request(url)?;
    parse_tv_season_detail_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB season detail response"),
            500,
        )
    })
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
        if let Some(tmdb_id) = ids.tmdb() {
            return Some(LookupTarget::DirectMovie(tmdb_id));
        }

        // Check all IDs for "tmdb:12345" patterns
        if let Some(id) = ids
            .as_all_ids()
            .iter()
            .find_map(|value| parse_tmdb_id(value))
        {
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
        if let Some(tmdb_id) = ids.tmdb() {
            return Some(LookupTarget::DirectTv(tmdb_id));
        }

        if let Some(id) = ids
            .as_all_ids()
            .iter()
            .find_map(|value| parse_tmdb_id(value))
        {
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
        if let Some(tmdb_id) = ids.tmdb() {
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

fn normalized(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn relation_filters<'a>(
    query: &'a RsLookupQuery,
) -> Option<(
    &'a [RsLookupPersonFilter],
    &'a [RsLookupSerieFilter],
    &'a [RsLookupTagFilter],
    TmdbMediaType,
    Option<u32>,
)> {
    let (people, series, tags, media_type, page_key) = match query {
        RsLookupQuery::Movie(movie) => (
            movie.people.as_deref().unwrap_or_default(),
            movie.series.as_deref().unwrap_or_default(),
            movie.tags.as_deref().unwrap_or_default(),
            TmdbMediaType::Movie,
            movie.page_key.as_deref(),
        ),
        RsLookupQuery::Serie(serie) => (
            serie.people.as_deref().unwrap_or_default(),
            serie.series.as_deref().unwrap_or_default(),
            serie.tags.as_deref().unwrap_or_default(),
            TmdbMediaType::Tv,
            serie.page_key.as_deref(),
        ),
        _ => return None,
    };
    (!people.is_empty() || !series.is_empty() || !tags.is_empty()).then_some((
        people,
        series,
        tags,
        media_type,
        page_key.and_then(|key| key.parse().ok()),
    ))
}

fn filter_id(ids: Option<&RsIds>, provider_key: &str) -> Option<u64> {
    ids.and_then(|ids| ids.get_u64(provider_key).or_else(|| ids.tmdb()))
}

fn crew_role_matches(job: &str, department: &str, role: &PersonType) -> bool {
    let job = normalized(job);
    let department = normalized(department);
    match role {
        PersonType::Director => job == "director" || department == "directing",
        PersonType::Writer => {
            matches!(job.as_str(), "writer" | "screenplay" | "story") || department == "writing"
        }
        PersonType::Producer => job.contains("producer") || department == "production",
        PersonType::Creator => job.contains("creator"),
        PersonType::Custom(value) => {
            let value = normalized(value);
            !value.is_empty() && (job == value || department == value)
        }
        _ => false,
    }
}

fn person_filter_matches(result: &TmdbResult, filter: &RsLookupPersonFilter) -> bool {
    let expected_id = filter_id(filter.ids.as_ref(), "tmdb-person");
    let expected_name = filter
        .name
        .as_deref()
        .map(normalized)
        .filter(|name| !name.is_empty());
    let identity_matches = |id: u64, name: &str| {
        expected_id.is_some_and(|expected| expected == id)
            || expected_name
                .as_ref()
                .is_some_and(|expected| normalized(name) == *expected)
    };

    match filter.role.as_ref() {
        None => {
            result
                .cast
                .iter()
                .any(|member| identity_matches(member.id, &member.name))
                || result
                    .crew
                    .iter()
                    .any(|member| identity_matches(member.id, &member.name))
        }
        Some(PersonType::Actor) => result
            .cast
            .iter()
            .any(|member| identity_matches(member.id, &member.name)),
        Some(role) => result.crew.iter().any(|member| {
            identity_matches(member.id, &member.name)
                && crew_role_matches(&member.job, &member.department, role)
        }),
    }
}

fn tag_filter_matches(result: &TmdbResult, filter: &RsLookupTagFilter) -> bool {
    let expected_id = filter_id(filter.ids.as_ref(), "tmdb-genre");
    let expected_name = filter
        .name
        .as_deref()
        .map(normalized)
        .filter(|name| !name.is_empty());
    result.genres.iter().any(|genre| {
        expected_id.is_some_and(|id| id == genre.id as u64)
            || expected_name
                .as_ref()
                .is_some_and(|name| normalized(&genre.name) == *name)
    })
}

fn series_filter_matches(result: &TmdbResult, filter: &RsLookupSerieFilter) -> bool {
    let Some(collection) = result.collection.as_ref() else {
        return false;
    };
    let expected_id = filter_id(filter.ids.as_ref(), "tmdb-collection");
    let expected_name = filter
        .name
        .as_deref()
        .map(normalized)
        .filter(|name| !name.is_empty());
    expected_id.is_some_and(|id| id == collection.id)
        || expected_name
            .as_ref()
            .is_some_and(|name| normalized(&collection.name) == *name)
}

fn result_matches_relation_filters(
    result: &TmdbResult,
    people: &[RsLookupPersonFilter],
    series: &[RsLookupSerieFilter],
    tags: &[RsLookupTagFilter],
) -> bool {
    people
        .iter()
        .all(|filter| person_filter_matches(result, filter))
        && series
            .iter()
            .all(|filter| series_filter_matches(result, filter))
        && tags.iter().all(|filter| tag_filter_matches(result, filter))
}

fn resolve_person_filter_id(api_key: &str, filter: &RsLookupPersonFilter) -> FnResult<Option<u64>> {
    if let Some(id) = filter_id(filter.ids.as_ref(), "tmdb-person") {
        return Ok(Some(id));
    }
    let Some(name) = filter
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    else {
        return Ok(None);
    };
    let (people, _) = execute_person_search_request(api_key, name, None)?;
    Ok(people
        .iter()
        .find(|person| normalized(&person.name) == normalized(name))
        .or_else(|| people.first())
        .map(|person| person.id))
}

fn credit_matches_role(credit: &TmdbCreditItem, role: &PersonType, cast: bool) -> bool {
    if role == &PersonType::Actor {
        return cast;
    }
    !cast
        && crew_role_matches(
            credit.job.as_deref().unwrap_or_default(),
            credit.department.as_deref().unwrap_or_default(),
            role,
        )
}

fn paginate_results(
    results: Vec<TmdbResult>,
    page: Option<u32>,
) -> (Vec<TmdbResult>, Option<String>) {
    const PAGE_SIZE: usize = 20;
    let page = page.unwrap_or(1).max(1) as usize;
    let start = (page - 1).saturating_mul(PAGE_SIZE);
    if start >= results.len() {
        return (Vec::new(), None);
    }
    let end = (start + PAGE_SIZE).min(results.len());
    let next = (end < results.len()).then(|| (page + 1).to_string());
    (
        results.into_iter().skip(start).take(PAGE_SIZE).collect(),
        next,
    )
}

fn person_seed_results(
    api_key: &str,
    filter: &RsLookupPersonFilter,
    media_type: &TmdbMediaType,
    page: Option<u32>,
) -> FnResult<(Vec<TmdbResult>, Option<String>)> {
    let Some(person_id) = resolve_person_filter_id(api_key, filter)? else {
        return Ok((Vec::new(), None));
    };
    let body = execute_json_request(build_person_credits_url(api_key, person_id, media_type))?;
    let credits = parse_person_credits_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB person credits"),
            500,
        )
    })?;

    let mut seen = HashSet::new();
    let mut results = Vec::new();
    for (credit, cast) in credits
        .cast
        .into_iter()
        .map(|credit| (credit, true))
        .chain(credits.crew.into_iter().map(|credit| (credit, false)))
    {
        if filter
            .role
            .as_ref()
            .is_some_and(|role| !credit_matches_role(&credit, role, cast))
        {
            continue;
        }
        if seen.insert(credit.item.id) {
            results.push(credit_item_to_result(credit, media_type.clone()));
        }
    }
    results.sort_by(|left, right| {
        right
            .popularity
            .partial_cmp(&left.popularity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(paginate_results(results, page))
}

fn resolve_genre_ids(
    api_key: &str,
    tags: &[RsLookupTagFilter],
    media_type: &TmdbMediaType,
) -> FnResult<Option<Vec<u64>>> {
    let needs_names = tags
        .iter()
        .any(|tag| filter_id(tag.ids.as_ref(), "tmdb-genre").is_none());
    let genres = if needs_names {
        execute_genre_list_request(api_key, media_type)?
    } else {
        Vec::new()
    };
    let mut ids = Vec::new();
    for tag in tags {
        let id = filter_id(tag.ids.as_ref(), "tmdb-genre").or_else(|| {
            let name = tag.name.as_deref()?;
            genres
                .iter()
                .find(|genre| normalized(&genre.name) == normalized(name))
                .map(|genre| genre.id as u64)
        });
        let Some(id) = id else {
            return Ok(None);
        };
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Ok(Some(ids))
}

fn tag_seed_results(
    api_key: &str,
    tags: &[RsLookupTagFilter],
    media_type: &TmdbMediaType,
    page: Option<u32>,
) -> FnResult<(Vec<TmdbResult>, Option<String>)> {
    let Some(ids) = resolve_genre_ids(api_key, tags, media_type)? else {
        return Ok((Vec::new(), None));
    };
    execute_discover_request(api_key, media_type, &ids, page)
}

fn resolve_collection_id(api_key: &str, filter: &RsLookupSerieFilter) -> FnResult<Option<u64>> {
    if let Some(id) = filter_id(filter.ids.as_ref(), "tmdb-collection") {
        return Ok(Some(id));
    }
    let Some(name) = filter
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    else {
        return Ok(None);
    };
    let Some(url) = build_collection_search_url(api_key, name) else {
        return Ok(None);
    };
    let body = execute_json_request(url)?;
    let collections = parse_collection_search_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB collection search"),
            500,
        )
    })?;
    Ok(collections
        .iter()
        .find(|collection| normalized(&collection.name) == normalized(name))
        .or_else(|| collections.first())
        .map(|collection| collection.id))
}

fn collection_seed_results(
    api_key: &str,
    filter: &RsLookupSerieFilter,
    page: Option<u32>,
) -> FnResult<(Vec<TmdbResult>, Option<String>)> {
    let Some(collection_id) = resolve_collection_id(api_key, filter)? else {
        return Ok((Vec::new(), None));
    };
    let body = execute_json_request(build_collection_detail_url(api_key, collection_id))?;
    let (collection, mut results) = parse_collection_detail_json(&body).ok_or_else(|| {
        WithReturnCode::new(
            extism_pdk::Error::msg("Failed to parse TMDB collection details"),
            500,
        )
    })?;
    for result in &mut results {
        result.collection = Some(collection.clone());
    }
    Ok(paginate_results(results, page))
}

fn relation_seed_results(
    lookup: &RsLookupWrapper,
    api_key: &str,
) -> FnResult<Option<(Vec<TmdbResult>, Option<String>)>> {
    let Some((people, series, tags, media_type, page)) = relation_filters(&lookup.query) else {
        return Ok(None);
    };
    if let Some(person) = people.first() {
        return person_seed_results(api_key, person, &media_type, page).map(Some);
    }
    if !tags.is_empty() {
        return tag_seed_results(api_key, tags, &media_type, page).map(Some);
    }
    if let Some(series) = series.first() {
        if media_type == TmdbMediaType::Movie {
            return collection_seed_results(api_key, series, page).map(Some);
        }
        return Ok(Some((Vec::new(), None)));
    }
    Ok(Some((Vec::new(), None)))
}

fn apply_relation_filters(
    lookup: &RsLookupWrapper,
    api_key: &str,
    results: Vec<TmdbResult>,
) -> FnResult<Vec<TmdbResult>> {
    let Some((people, series, tags, default_media_type, _)) = relation_filters(&lookup.query)
    else {
        return Ok(results);
    };
    let mut filtered = Vec::new();
    for result in results {
        let media_type = result
            .media_type
            .clone()
            .unwrap_or_else(|| default_media_type.clone());
        let detail = match media_type {
            TmdbMediaType::Movie => execute_movie_detail_request(api_key, result.id),
            TmdbMediaType::Tv => execute_tv_detail_request(api_key, result.id),
        }?;
        if let Some(detail) = detail {
            if result_matches_relation_filters(&detail, people, series, tags) {
                filtered.push(detail);
            }
        }
    }
    Ok(filtered)
}

fn lookup_tmdb(
    lookup: &RsLookupWrapper,
    api_key: &str,
) -> FnResult<(Vec<TmdbResult>, Option<String>, Option<RsLookupMatchType>)> {
    let (results, next_page_key, match_type) = match &lookup.query {
        RsLookupQuery::Movie(movie) => {
            let page = movie
                .page_key
                .as_deref()
                .and_then(|k| k.parse::<u32>().ok());

            match resolve_movie_lookup_target(movie) {
                Some(LookupTarget::DirectMovie(id)) => {
                    let result = execute_movie_detail_request(api_key, id)?;
                    (
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    )
                }
                Some(LookupTarget::DirectTv(id)) => {
                    let result = execute_tv_detail_request(api_key, id)?;
                    (
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    )
                }
                Some(LookupTarget::DirectUnknown(id)) => {
                    // Try movie first, then TV
                    if let Ok(Some(result)) = execute_movie_detail_request(api_key, id) {
                        (vec![result], None, Some(RsLookupMatchType::ExactId))
                    } else {
                        let result = execute_tv_detail_request(api_key, id)?;
                        (
                            result.into_iter().collect(),
                            None,
                            Some(RsLookupMatchType::ExactId),
                        )
                    }
                }
                Some(LookupTarget::SearchMovie(query)) => {
                    let (results, next_page_key) =
                        execute_movie_search_request(api_key, &query, page)?;
                    (results, next_page_key, None)
                }
                Some(LookupTarget::SearchTv(query)) => {
                    let (results, next_page_key) =
                        execute_tv_search_request(api_key, &query, page)?;
                    (results, next_page_key, None)
                }
                None => match relation_seed_results(lookup, api_key)? {
                    Some((results, next_page_key)) => (results, next_page_key, None),
                    None => {
                        return Err(WithReturnCode::new(
                            extism_pdk::Error::msg("Empty movie query"),
                            404,
                        ));
                    }
                },
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
                    (
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    )
                }
                Some(LookupTarget::DirectMovie(id)) => {
                    let result = execute_movie_detail_request(api_key, id)?;
                    (
                        result.into_iter().collect(),
                        None,
                        Some(RsLookupMatchType::ExactId),
                    )
                }
                Some(LookupTarget::DirectUnknown(id)) => {
                    // Try TV first, then movie
                    if let Ok(Some(result)) = execute_tv_detail_request(api_key, id) {
                        (vec![result], None, Some(RsLookupMatchType::ExactId))
                    } else {
                        let result = execute_movie_detail_request(api_key, id)?;
                        (
                            result.into_iter().collect(),
                            None,
                            Some(RsLookupMatchType::ExactId),
                        )
                    }
                }
                Some(LookupTarget::SearchTv(query)) => {
                    let (results, next_page_key) =
                        execute_tv_search_request(api_key, &query, page)?;
                    (results, next_page_key, None)
                }
                Some(LookupTarget::SearchMovie(query)) => {
                    let (results, next_page_key) =
                        execute_movie_search_request(api_key, &query, page)?;
                    (results, next_page_key, None)
                }
                None => match relation_seed_results(lookup, api_key)? {
                    Some((results, next_page_key)) => (results, next_page_key, None),
                    None => {
                        return Err(WithReturnCode::new(
                            extism_pdk::Error::msg("Empty serie query"),
                            404,
                        ));
                    }
                },
            }
        }
        _ => return Ok((vec![], None, None)),
    };

    let results = apply_relation_filters(lookup, api_key, results)?;
    Ok((results, next_page_key, match_type))
}

fn lookup_tmdb_person(
    lookup: &RsLookupWrapper,
    api_key: &str,
) -> FnResult<(
    Vec<TmdbPersonResult>,
    Option<String>,
    Option<RsLookupMatchType>,
)> {
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

    if let RsLookupQuery::Episode(ref episode) = lookup.query {
        let tv_id = episode
            .ids
            .as_ref()
            .and_then(|ids| ids.tmdb())
            .ok_or_else(|| {
                WithReturnCode::new(
                    extism_pdk::Error::msg("Episode query requires a TMDB serie ID"),
                    404,
                )
            })?;

        let mut episodes = execute_tv_season_detail_request(&api_key, tv_id, episode.season)?;
        if let Some(episode_number) = episode.number {
            episodes.retain(|entry| entry.episode_number == episode_number);
        }

        let serie_id = format!("tmdb:{tv_id}");
        let results = episodes
            .into_iter()
            .map(|entry| {
                let mut result = tmdb_episode_to_metadata(serie_id.clone(), entry);
                result.match_type = Some(RsLookupMatchType::ExactId);
                result
            })
            .collect();

        return Ok(Json(RsLookupMetadataResults {
            results,
            next_page_key: None,
        }));
    }

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

    if let RsLookupQuery::Episode(ref episode) = lookup.query {
        let tv_id = episode
            .ids
            .as_ref()
            .and_then(|ids| ids.tmdb())
            .ok_or_else(|| {
                WithReturnCode::new(
                    extism_pdk::Error::msg("Episode query requires a TMDB serie ID"),
                    404,
                )
            })?;

        let episode_number = episode.number.ok_or_else(|| {
            WithReturnCode::new(
                extism_pdk::Error::msg("Episode query requires an episode number"),
                404,
            )
        })?;

        let stills =
            execute_episode_images_request(&api_key, tv_id, episode.season, episode_number)?;

        let images: Vec<ExternalImage> = tmdb_episode_stills_to_images(&stills)
            .into_iter()
            .map(|mut img| {
                img.match_type = Some(RsLookupMatchType::ExactId);
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
            ..Default::default()
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
            ids: Some(RsIds::from_tmdb(550)),
            page_key: None,
            ..Default::default()
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
            ids: Some(RsIds::try_from(vec!["tmdb:550".to_string()]).unwrap()),
            page_key: None,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };

        assert!(resolve_movie_lookup_target(&movie).is_none());
    }

    #[test]
    fn resolve_serie_target_prefers_direct_id() {
        let serie = RsLookupSerie {
            name: Some("tmdb:1396".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    fn filtered_movie() -> TmdbResult {
        TmdbResult {
            media_type: Some(TmdbMediaType::Movie),
            id: 550,
            title: "Fight Club".to_string(),
            cast: vec![tmdb::TmdbCastMember {
                id: 287,
                name: "Brad Pitt".to_string(),
                ..Default::default()
            }],
            crew: vec![tmdb::TmdbCrewMember {
                id: 7467,
                name: "David Fincher".to_string(),
                job: "Director".to_string(),
                department: "Directing".to_string(),
                ..Default::default()
            }],
            genres: vec![tmdb::TmdbGenre {
                id: 18,
                name: "Drama".to_string(),
            }],
            collection: Some(tmdb::TmdbCollectionRef {
                id: 123,
                name: "Fight Club Collection".to_string(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn optional_person_role_matches_cast_or_crew() {
        let result = filtered_movie();
        for (id, name) in [(287, "Brad Pitt"), (7467, "David Fincher")] {
            let filter = RsLookupPersonFilter {
                name: Some(name.to_string()),
                ids: Some(RsIds::from_tmdb(id)),
                role: None,
            };
            assert!(person_filter_matches(&result, &filter));
        }
    }

    #[test]
    fn explicit_person_role_narrows_the_match() {
        let result = filtered_movie();
        let mut filter = RsLookupPersonFilter {
            name: Some("David Fincher".to_string()),
            role: Some(PersonType::Director),
            ..Default::default()
        };
        assert!(person_filter_matches(&result, &filter));
        filter.role = Some(PersonType::Actor);
        assert!(!person_filter_matches(&result, &filter));
    }

    #[test]
    fn tag_and_collection_filters_accept_names_and_external_ids() {
        let result = filtered_movie();
        let mut tag_ids = RsIds::default();
        tag_ids.set("tmdb-genre", 18);
        let mut collection_ids = RsIds::default();
        collection_ids.set("tmdb-collection", 123);

        assert!(tag_filter_matches(
            &result,
            &RsLookupTagFilter {
                ids: Some(tag_ids),
                ..Default::default()
            }
        ));
        assert!(series_filter_matches(
            &result,
            &RsLookupSerieFilter {
                name: Some("Fight Club Collection".to_string()),
                ids: Some(collection_ids),
            }
        ));
    }
}
