use rs_plugin_common_interfaces::{
    domain::{
        external_images::{ExternalImage, ImageType},
        movie::{Movie, MovieStatus},
        person::Person,
        serie::{Serie, SerieStatus, SerieType},
        tag::Tag,
        Relations,
    },
    lookup::{RsLookupMetadataResult, RsLookupMetadataResultWrapper},
    RsRequest,
};

use crate::tmdb::{
    build_image_url, TmdbCastMember, TmdbCrewMember, TmdbGenre, TmdbImage, TmdbMediaType,
    TmdbPersonResult, TmdbResult, TMDB_IMAGE_SIZE_ORIGINAL,
};

pub fn tmdb_result_to_metadata(item: TmdbResult) -> RsLookupMetadataResultWrapper {
    let images = tmdb_result_to_images(&item);
    let people_details = build_people_details(&item.cast, &item.crew);
    let tag_details = build_tag_details(&item.genres);

    let metadata = match item.media_type.as_ref() {
        Some(TmdbMediaType::Tv) => {
            let serie = Serie {
                id: item.id.to_string(),
                name: item.title,
                kind: Some(SerieType::Tv),
                year: parse_year_from_date(&item.release_date),
                tmdb: Some(item.id),
                status: map_serie_status(&item.status),
                ..Default::default()
            };
            RsLookupMetadataResult::Serie(serie)
        }
        _ => {
            let movie = Movie {
                id: item.id.to_string(),
                name: item.title,
                year: parse_year_from_date(&item.release_date),
                overview: item.overview,
                duration: item.runtime,
                tmdb: Some(item.id),
                imdb: item.imdb_id,
                lang: item.original_language,
                original: item.original_title,
                status: map_movie_status(&item.status),
                ..Default::default()
            };
            RsLookupMetadataResult::Movie(movie)
        }
    };

    RsLookupMetadataResultWrapper {
        metadata,
        relations: Some(Relations {
            ext_images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
            people_details: if people_details.is_empty() {
                None
            } else {
                Some(people_details)
            },
            tags_details: if tag_details.is_empty() {
                None
            } else {
                Some(tag_details)
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn tmdb_result_to_images(item: &TmdbResult) -> Vec<ExternalImage> {
    let mut images = Vec::new();

    // Primary poster
    if let Some(ref path) = item.poster_path {
        images.push(ExternalImage {
            kind: Some(ImageType::Poster),
            url: RsRequest {
                url: build_image_url(path, TMDB_IMAGE_SIZE_ORIGINAL),
                ..Default::default()
            },
            ..Default::default()
        });
    }

    // Primary backdrop
    if let Some(ref path) = item.backdrop_path {
        images.push(ExternalImage {
            kind: Some(ImageType::Background),
            url: RsRequest {
                url: build_image_url(path, TMDB_IMAGE_SIZE_ORIGINAL),
                ..Default::default()
            },
            ..Default::default()
        });
    }

    // Additional posters from detail images
    for img in &item.images.posters {
        let url = build_image_url(&img.file_path, TMDB_IMAGE_SIZE_ORIGINAL);
        // Skip if already added as primary poster
        if images.iter().any(|e| e.url.url == url) {
            continue;
        }
        images.push(tmdb_image_to_external(img, ImageType::Poster));
    }

    // Additional backdrops from detail images
    for img in &item.images.backdrops {
        let url = build_image_url(&img.file_path, TMDB_IMAGE_SIZE_ORIGINAL);
        if images.iter().any(|e| e.url.url == url) {
            continue;
        }
        images.push(tmdb_image_to_external(img, ImageType::Background));
    }

    images
}

fn tmdb_image_to_external(img: &TmdbImage, kind: ImageType) -> ExternalImage {
    ExternalImage {
        kind: Some(kind),
        url: RsRequest {
            url: build_image_url(&img.file_path, TMDB_IMAGE_SIZE_ORIGINAL),
            ..Default::default()
        },
        width: img.width.map(|w| w as i64),
        height: img.height.map(|h| h as i64),
        aspect_ratio: img.aspect_ratio,
        vote_average: img.vote_average,
        vote_count: img.vote_count.map(|v| v as i64),
        lang: img.iso_639_1.clone(),
        ..Default::default()
    }
}

fn build_people_details(cast: &[TmdbCastMember], crew: &[TmdbCrewMember]) -> Vec<Person> {
    let mut people = Vec::new();
    let mut seen_ids = std::collections::HashSet::<u64>::new();

    for member in cast {
        if seen_ids.insert(member.id) {
            people.push(Person {
                id: member.id.to_string(),
                name: member.name.clone(),
                tmdb: Some(member.id),
                generated: true,
                ..Default::default()
            });
        }
    }

    // Include key crew: Directors, Writers, Producers, Creators
    for member in crew {
        let job_lower = member.job.to_ascii_lowercase();
        if !matches!(
            job_lower.as_str(),
            "director" | "writer" | "screenplay" | "producer" | "creator"
        ) {
            continue;
        }
        if seen_ids.insert(member.id) {
            people.push(Person {
                id: member.id.to_string(),
                name: member.name.clone(),
                tmdb: Some(member.id),
                generated: true,
                ..Default::default()
            });
        }
    }

    people
}

fn build_tag_details(genres: &[TmdbGenre]) -> Vec<Tag> {
    genres
        .iter()
        .map(|genre| Tag {
            id: format!("tmdb-genre:{}", genre.id),
            name: genre.name.clone(),
            parent: None,
            kind: None,
            alt: None,
            thumb: None,
            params: None,
            modified: 0,
            added: 0,
            generated: true,
            otherids: Some(vec![format!("tmdb-genre:{}", genre.id)].into()),
            path: "/".to_string(),
        })
        .collect()
}

pub fn tmdb_person_to_metadata(item: TmdbPersonResult) -> RsLookupMetadataResultWrapper {
    let mut images = Vec::new();

    // Primary profile image
    if let Some(ref path) = item.profile_path {
        images.push(ExternalImage {
            kind: Some(ImageType::Poster),
            url: RsRequest {
                url: build_image_url(path, TMDB_IMAGE_SIZE_ORIGINAL),
                ..Default::default()
            },
            ..Default::default()
        });
    }

    // Additional profile images
    for img in &item.images {
        let url = build_image_url(&img.file_path, TMDB_IMAGE_SIZE_ORIGINAL);
        if images.iter().any(|e| e.url.url == url) {
            continue;
        }
        images.push(tmdb_image_to_external(img, ImageType::Poster));
    }

    let person = Person {
        id: item.id.to_string(),
        name: item.name,
        tmdb: Some(item.id),
        imdb: item.imdb_id,
        bio: item.biography,
        birthday: item.birthday.as_deref().and_then(parse_date_to_timestamp),
        death: item.deathday.as_deref().and_then(parse_date_to_timestamp),
        gender: item.gender.and_then(map_tmdb_gender),
        country: item.place_of_birth,
        kind: item.known_for_department,
        alt: if item.also_known_as.is_empty() {
            None
        } else {
            Some(item.also_known_as)
        },
        portrait: item
            .profile_path
            .as_deref()
            .map(|p| build_image_url(p, TMDB_IMAGE_SIZE_ORIGINAL)),
        generated: true,
        ..Default::default()
    };

    RsLookupMetadataResultWrapper {
        metadata: RsLookupMetadataResult::Person(person),
        relations: Some(Relations {
            ext_images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn tmdb_person_to_images(item: &TmdbPersonResult) -> Vec<ExternalImage> {
    let mut images = Vec::new();

    if let Some(ref path) = item.profile_path {
        images.push(ExternalImage {
            kind: Some(ImageType::Poster),
            url: RsRequest {
                url: build_image_url(path, TMDB_IMAGE_SIZE_ORIGINAL),
                ..Default::default()
            },
            ..Default::default()
        });
    }

    for img in &item.images {
        let url = build_image_url(&img.file_path, TMDB_IMAGE_SIZE_ORIGINAL);
        if images.iter().any(|e| e.url.url == url) {
            continue;
        }
        images.push(tmdb_image_to_external(img, ImageType::Poster));
    }

    images
}

fn parse_date_to_timestamp(date: &str) -> Option<i64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }

    // Approximate: days since Unix epoch
    let days_from_year = (year as i64 - 1970) * 365 + ((year as i64 - 1969) / 4);
    let month_days: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let days = days_from_year + month_days[(month - 1) as usize] + day as i64 - 1;
    Some(days * 86400)
}

fn map_tmdb_gender(gender: u8) -> Option<rs_plugin_common_interfaces::Gender> {
    use rs_plugin_common_interfaces::Gender;
    match gender {
        0 => None,
        1 => Some(Gender::Female),
        2 => Some(Gender::Male),
        3 => Some(Gender::Other),
        _ => None,
    }
}

fn parse_year_from_date(date: &Option<String>) -> Option<u16> {
    date.as_ref()?
        .split('-')
        .next()?
        .parse::<u16>()
        .ok()
        .filter(|y| *y > 0)
}

fn map_movie_status(status: &Option<String>) -> Option<MovieStatus> {
    let s = status.as_ref()?;
    Some(match s.as_str() {
        "Released" => MovieStatus::Released,
        "In Production" => MovieStatus::InProduction,
        "Post Production" => MovieStatus::PostProduction,
        "Planned" => MovieStatus::Planned,
        "Rumored" => MovieStatus::Rumored,
        "Canceled" => MovieStatus::Canceled,
        _ => MovieStatus::Other(s.clone()),
    })
}

fn map_serie_status(status: &Option<String>) -> Option<SerieStatus> {
    let s = status.as_ref()?;
    Some(match s.as_str() {
        "Returning Series" => SerieStatus::Returning,
        "Ended" => SerieStatus::Ended,
        "Canceled" => SerieStatus::Canceled,
        "In Production" => SerieStatus::InProduction,
        "Pilot" => SerieStatus::Pilot,
        "Planned" => SerieStatus::Planned,
        _ => SerieStatus::Other(s.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmdb::{TmdbCastMember, TmdbCrewMember, TmdbGenre, TmdbImages};

    #[test]
    fn maps_movie_result_to_metadata() {
        let result = tmdb_result_to_metadata(TmdbResult {
            media_type: Some(TmdbMediaType::Movie),
            id: 550,
            title: "Fight Club".to_string(),
            original_title: Some("Fight Club".to_string()),
            overview: Some("An insomniac.".to_string()),
            release_date: Some("1999-10-15".to_string()),
            poster_path: Some("/poster.jpg".to_string()),
            imdb_id: Some("tt0137523".to_string()),
            runtime: Some(139),
            original_language: Some("en".to_string()),
            status: Some("Released".to_string()),
            ..Default::default()
        });

        if let RsLookupMetadataResult::Movie(movie) = result.metadata {
            assert_eq!(movie.id, "550");
            assert_eq!(movie.name, "Fight Club");
            assert_eq!(movie.tmdb, Some(550));
            assert_eq!(movie.imdb, Some("tt0137523".to_string()));
            assert_eq!(movie.year, Some(1999));
            assert_eq!(movie.duration, Some(139));
            assert_eq!(movie.lang, Some("en".to_string()));
            assert_eq!(movie.original, Some("Fight Club".to_string()));
        } else {
            panic!("Expected Movie metadata");
        }

        let images = result
            .relations
            .as_ref()
            .and_then(|r| r.ext_images.as_ref())
            .expect("expected images");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].kind, Some(ImageType::Poster));
    }

    #[test]
    fn maps_tv_result_to_serie_metadata() {
        let result = tmdb_result_to_metadata(TmdbResult {
            media_type: Some(TmdbMediaType::Tv),
            id: 1396,
            title: "Breaking Bad".to_string(),
            release_date: Some("2008-01-20".to_string()),
            status: Some("Ended".to_string()),
            ..Default::default()
        });

        if let RsLookupMetadataResult::Serie(serie) = result.metadata {
            assert_eq!(serie.id, "1396");
            assert_eq!(serie.name, "Breaking Bad");
            assert_eq!(serie.tmdb, Some(1396));
            assert_eq!(serie.year, Some(2008));
            assert_eq!(serie.kind, Some(SerieType::Tv));
        } else {
            panic!("Expected Serie metadata");
        }
    }

    #[test]
    fn maps_cast_and_crew_to_people() {
        let result = tmdb_result_to_metadata(TmdbResult {
            media_type: Some(TmdbMediaType::Movie),
            id: 550,
            title: "Fight Club".to_string(),
            cast: vec![
                TmdbCastMember {
                    id: 819,
                    name: "Edward Norton".to_string(),
                    ..Default::default()
                },
                TmdbCastMember {
                    id: 287,
                    name: "Brad Pitt".to_string(),
                    ..Default::default()
                },
            ],
            crew: vec![TmdbCrewMember {
                id: 7467,
                name: "David Fincher".to_string(),
                job: "Director".to_string(),
                department: "Directing".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let people = result
            .relations
            .as_ref()
            .and_then(|r| r.people_details.as_ref())
            .expect("expected people");
        assert_eq!(people.len(), 3);
        assert_eq!(people[0].id, "819");
        assert_eq!(people[0].name, "Edward Norton");
        assert_eq!(people[2].id, "7467");
        assert_eq!(people[2].name, "David Fincher");
    }

    #[test]
    fn maps_genres_to_tags() {
        let result = tmdb_result_to_metadata(TmdbResult {
            media_type: Some(TmdbMediaType::Movie),
            id: 550,
            title: "Fight Club".to_string(),
            genres: vec![
                TmdbGenre {
                    id: 18,
                    name: "Drama".to_string(),
                },
                TmdbGenre {
                    id: 53,
                    name: "Thriller".to_string(),
                },
            ],
            ..Default::default()
        });

        let tags = result
            .relations
            .as_ref()
            .and_then(|r| r.tags_details.as_ref())
            .expect("expected tags");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].id, "tmdb-genre:18");
        assert_eq!(tags[0].name, "Drama");
        assert_eq!(tags[1].id, "tmdb-genre:53");
        assert_eq!(tags[1].name, "Thriller");
    }

    #[test]
    fn maps_poster_and_backdrop_images() {
        let item = TmdbResult {
            poster_path: Some("/poster.jpg".to_string()),
            backdrop_path: Some("/backdrop.jpg".to_string()),
            ..Default::default()
        };

        let images = tmdb_result_to_images(&item);
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].kind, Some(ImageType::Poster));
        assert_eq!(
            images[0].url.url,
            "https://image.tmdb.org/t/p/original/poster.jpg"
        );
        assert_eq!(images[1].kind, Some(ImageType::Background));
        assert_eq!(
            images[1].url.url,
            "https://image.tmdb.org/t/p/original/backdrop.jpg"
        );
    }

    #[test]
    fn maps_additional_images_from_detail() {
        let item = TmdbResult {
            poster_path: Some("/poster.jpg".to_string()),
            images: TmdbImages {
                posters: vec![
                    crate::tmdb::TmdbImage {
                        file_path: "/poster.jpg".to_string(),
                        width: Some(500),
                        height: Some(750),
                        ..Default::default()
                    },
                    crate::tmdb::TmdbImage {
                        file_path: "/poster2.jpg".to_string(),
                        width: Some(500),
                        height: Some(750),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let images = tmdb_result_to_images(&item);
        // 1 primary poster + 1 additional (primary dupe is skipped)
        assert_eq!(images.len(), 2);
        assert_eq!(
            images[1].url.url,
            "https://image.tmdb.org/t/p/original/poster2.jpg"
        );
        assert_eq!(images[1].width, Some(500));
        assert_eq!(images[1].height, Some(750));
    }

    #[test]
    fn parse_year_from_date_works() {
        assert_eq!(
            parse_year_from_date(&Some("1999-10-15".to_string())),
            Some(1999)
        );
        assert_eq!(
            parse_year_from_date(&Some("2024-01-01".to_string())),
            Some(2024)
        );
        assert_eq!(parse_year_from_date(&None), None);
        assert_eq!(parse_year_from_date(&Some("".to_string())), None);
    }

    #[test]
    fn map_movie_status_works() {
        assert!(matches!(
            map_movie_status(&Some("Released".to_string())),
            Some(MovieStatus::Released)
        ));
        assert!(matches!(
            map_movie_status(&Some("In Production".to_string())),
            Some(MovieStatus::InProduction)
        ));
        assert!(matches!(
            map_movie_status(&Some("Canceled".to_string())),
            Some(MovieStatus::Canceled)
        ));
        assert!(matches!(map_movie_status(&None), None));
    }

    #[test]
    fn map_serie_status_works() {
        assert!(matches!(
            map_serie_status(&Some("Returning Series".to_string())),
            Some(SerieStatus::Returning)
        ));
        assert!(matches!(
            map_serie_status(&Some("Ended".to_string())),
            Some(SerieStatus::Ended)
        ));
        assert!(matches!(
            map_serie_status(&Some("Canceled".to_string())),
            Some(SerieStatus::Canceled)
        ));
        assert!(matches!(map_serie_status(&None), None));
    }

    #[test]
    fn no_duplicate_crew_when_also_in_cast() {
        let people = build_people_details(
            &[TmdbCastMember {
                id: 100,
                name: "Person A".to_string(),
                ..Default::default()
            }],
            &[TmdbCrewMember {
                id: 100,
                name: "Person A".to_string(),
                job: "Director".to_string(),
                department: "Directing".to_string(),
                ..Default::default()
            }],
        );
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].id, "100");
    }
}
