use extism::*;
use rs_plugin_common_interfaces::{
    domain::{external_images::ExternalImage, rs_ids::RsIds},
    lookup::{
        RsLookupEpisode, RsLookupMetadataResult, RsLookupMetadataResults, RsLookupMovie,
        RsLookupPerson, RsLookupPersonFilter, RsLookupQuery, RsLookupSerie, RsLookupSerieFilter,
        RsLookupTagFilter, RsLookupWrapper,
    },
};

fn build_plugin() -> Plugin {
    let wasm = Wasm::file("target/wasm32-unknown-unknown/release/rs_plugin_tmdb.wasm");
    let manifest = Manifest::new([wasm]).with_allowed_hosts(
        ["api.themoviedb.org", "image.tmdb.org"]
            .iter()
            .map(|s| s.to_string()),
    );
    Plugin::new(&manifest, [], true).expect("Failed to create plugin")
}

fn call_lookup(plugin: &mut Plugin, input: &RsLookupWrapper) -> RsLookupMetadataResults {
    let input_str = serde_json::to_string(input).unwrap();
    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect("lookup_metadata call failed");
    serde_json::from_slice(output).expect("Failed to parse lookup output")
}

fn call_lookup_images(plugin: &mut Plugin, input: &RsLookupWrapper) -> Vec<ExternalImage> {
    let input_str = serde_json::to_string(input).unwrap();
    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata_images", &input_str)
        .expect("lookup_metadata_images call failed");
    serde_json::from_slice(output).expect("Failed to parse images output")
}

fn provider_ids(key: &str, value: u64) -> RsIds {
    let mut ids = RsIds::default();
    ids.set(key, value);
    ids
}

#[test]
fn test_lookup_no_credential_uses_default_key() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("Fight Club".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(
        !results.results.is_empty(),
        "Expected results using default API key"
    );
}

#[test]
fn test_lookup_movie_search() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("Fight Club".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(
        !results.results.is_empty(),
        "Expected at least one result for 'Fight Club'"
    );

    let first = &results.results[0];
    let movie = match &first.metadata {
        RsLookupMetadataResult::Movie(movie) => movie,
        _ => panic!("Expected Movie metadata"),
    };
    assert!(
        !movie.name.trim().is_empty(),
        "Expected a non-empty movie name"
    );
    assert!(movie.tmdb.is_some(), "Expected tmdb ID to be set");
    println!(
        "Movie search returned {} results, first: {} (tmdb:{})",
        results.results.len(),
        movie.name,
        movie.tmdb.unwrap_or(0)
    );
}

#[test]
fn test_lookup_movie_direct_id() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("tmdb:550".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(!results.results.is_empty(), "Expected result for tmdb:550");

    let first = &results.results[0];
    let movie = match &first.metadata {
        RsLookupMetadataResult::Movie(movie) => movie,
        _ => panic!("Expected Movie metadata"),
    };

    assert_eq!(movie.tmdb, Some(550), "Expected tmdb ID 550");
    assert!(movie.imdb.is_some(), "Expected IMDB ID for detail lookup");
    assert!(
        movie.duration.is_some(),
        "Expected runtime for detail lookup"
    );
    assert!(movie.airdate.is_some(), "Expected theatrical release date");
    assert!(
        movie.digitalairdate.is_some(),
        "Expected digital release date"
    );
    println!(
        "Direct ID lookup: {} (imdb: {:?}, runtime: {:?}, airdate: {:?}, digitalairdate: {:?})",
        movie.name, movie.imdb, movie.duration, movie.airdate, movie.digitalairdate
    );

    // Check for people (cast/crew)
    let people = first
        .relations
        .as_ref()
        .and_then(|r| r.people_details.as_ref());
    assert!(
        people.map(|p| !p.is_empty()).unwrap_or(false),
        "Expected at least one person in relations"
    );

    // Check for tags (genres)
    let tags = first
        .relations
        .as_ref()
        .and_then(|r| r.tags_details.as_ref());
    assert!(
        tags.map(|t| !t.is_empty()).unwrap_or(false),
        "Expected at least one tag/genre in relations"
    );
}

#[test]
fn test_lookup_tv_search() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("Breaking Bad".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(
        !results.results.is_empty(),
        "Expected at least one result for 'Breaking Bad'"
    );

    let first = &results.results[0];
    let serie = match &first.metadata {
        RsLookupMetadataResult::Serie(serie) => serie,
        _ => panic!("Expected Serie metadata"),
    };
    assert!(
        !serie.name.trim().is_empty(),
        "Expected a non-empty serie name"
    );
    assert!(serie.tmdb.is_some(), "Expected tmdb ID to be set");
    println!(
        "TV search returned {} results, first: {} (tmdb:{})",
        results.results.len(),
        serie.name,
        serie.tmdb.unwrap_or(0)
    );
}

#[test]
fn test_lookup_tv_direct_id() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("tmdb:1396".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(!results.results.is_empty(), "Expected result for tmdb:1396");

    let first = &results.results[0];
    let serie = match &first.metadata {
        RsLookupMetadataResult::Serie(serie) => serie,
        _ => panic!("Expected Serie metadata"),
    };

    assert_eq!(serie.tmdb, Some(1396), "Expected tmdb ID 1396");
    assert_eq!(serie.tvdb, Some(81189), "Expected tvdb ID 81189");
    println!("Direct TV ID lookup: {} (tmdb:1396)", serie.name);
}

#[test]
fn test_lookup_movie_pagination() {
    let mut plugin = build_plugin();

    let page1_input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("love".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let page1 = call_lookup(&mut plugin, &page1_input);
    assert!(!page1.results.is_empty(), "Expected page 1 results");
    assert!(
        page1.next_page_key.is_some(),
        "Expected next_page_key for broad search"
    );

    let page2_input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("love".to_string()),
            ids: None,
            page_key: page1.next_page_key.clone(),
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let page2 = call_lookup(&mut plugin, &page2_input);
    assert!(!page2.results.is_empty(), "Expected page 2 results");

    let page1_first_id = match &page1.results[0].metadata {
        RsLookupMetadataResult::Movie(m) => m.id.clone(),
        _ => panic!("Expected Movie"),
    };
    let page2_first_id = match &page2.results[0].metadata {
        RsLookupMetadataResult::Movie(m) => m.id.clone(),
        _ => panic!("Expected Movie"),
    };

    assert_ne!(
        page1_first_id, page2_first_id,
        "Expected different results on page 1 and 2"
    );
    println!(
        "Page 1 first: {}, Page 2 first: {}",
        page1_first_id, page2_first_id
    );
}

#[test]
fn test_lookup_images() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("tmdb:550".to_string()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let images = call_lookup_images(&mut plugin, &input);
    assert!(
        !images.is_empty(),
        "Expected at least one image for tmdb:550"
    );
    println!("Got {} images for tmdb:550", images.len());
    for img in &images {
        println!("  {:?}: {}", img.kind, img.url.url);
    }
}

#[test]
fn test_lookup_person_tmdb_5719226() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Person(RsLookupPerson {
            name: Some("tmdb:5719226".to_string()),
            ids: Some(RsIds::from_tmdb(5719226)),
            page_key: None,
        }),
        credential: None,
        params: None,
    };

    let input_str = serde_json::to_string(&input).unwrap();
    match plugin.call::<&str, &[u8]>("lookup_metadata", &input_str) {
        Ok(output) => {
            let results: RsLookupMetadataResults =
                serde_json::from_slice(output).expect("Failed to parse output");
            println!(
                "Got {} results for person tmdb:5719226",
                results.results.len()
            );
            for r in &results.results {
                match &r.metadata {
                    RsLookupMetadataResult::Person(p) => {
                        println!("  Person: {} (id: {})", p.name, p.id);
                        println!("  Full: {:?}", p);
                    }
                    other => println!("  Other type: {:?}", other),
                }
                if let Some(rel) = &r.relations {
                    if let Some(imgs) = &rel.ext_images {
                        println!("  Images: {} entries", imgs.len());
                        for img in imgs.iter().take(5) {
                            println!("    - {:?}: {}", img.kind, img.url.url);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error for person tmdb:5719226: {}", e);
        }
    }
}

#[test]
fn test_lookup_episode_images() {
    let mut plugin = build_plugin();

    // Breaking Bad S01E01 (TMDB TV ID 1396)
    let input = RsLookupWrapper {
        query: RsLookupQuery::Episode(RsLookupEpisode {
            name: None,
            ids: Some(RsIds::from_tmdb(1396)),
            season: 1,
            number: Some(1),
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let images = call_lookup_images(&mut plugin, &input);
    assert!(
        !images.is_empty(),
        "Expected at least one still image for Breaking Bad S01E01"
    );

    for img in &images {
        assert_eq!(
            img.kind,
            Some(rs_plugin_common_interfaces::domain::external_images::ImageType::Still),
            "Expected all episode images to be Stills"
        );
        assert!(
            img.url
                .url
                .starts_with("https://image.tmdb.org/t/p/original/"),
            "Expected TMDB image URL"
        );
    }

    println!("Got {} still images for Breaking Bad S01E01", images.len());
    for img in images.iter().take(3) {
        println!(
            "  {:?}: {} (status: {:?})",
            img.kind, img.url.url, img.url.status
        );
    }
}

#[test]
fn test_lookup_episode_metadata_by_tmdb_show_id() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Episode(RsLookupEpisode {
            name: None,
            ids: Some(RsIds::from_tmdb(1396)),
            season: 1,
            number: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(
        !results.results.is_empty(),
        "Expected episode metadata results for TMDB TV ID 1396 season 1"
    );

    let first = &results.results[0];
    let episode = match &first.metadata {
        RsLookupMetadataResult::Episode(episode) => episode,
        _ => panic!("Expected Episode metadata"),
    };

    assert_eq!(episode.season, 1);
    assert!(episode.number >= 1);
    assert_eq!(
        first.match_type,
        Some(rs_plugin_common_interfaces::lookup::RsLookupMatchType::ExactId)
    );
}

#[test]
fn test_lookup_empty_movie_name_returns_error() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some(String::new()),
            ids: None,
            page_key: None,
            ..Default::default()
        }),
        credential: None,
        params: None,
    };

    let input_str = serde_json::to_string(&input).unwrap();
    let err = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect_err("Expected error for empty search");

    let message = err.to_string();
    assert!(
        message.contains("Empty") || message.contains("404"),
        "Expected empty query error, got: {message}"
    );
}

#[test]
fn test_lookup_person_type_uses_canonical_string() {
    let mut plugin = build_plugin();
    let input = RsLookupWrapper {
        query: RsLookupQuery::Person(RsLookupPerson {
            name: None,
            ids: Some(RsIds::from_tmdb(287)),
            page_key: None,
        }),
        credential: None,
        params: None,
    };
    let results = call_lookup(&mut plugin, &input);
    let person = results
        .results
        .into_iter()
        .find_map(|result| match result.metadata {
            RsLookupMetadataResult::Person(person) => Some(person),
            _ => None,
        })
        .expect("Expected full person metadata for TMDB 287");
    assert_eq!(person.tmdb, Some(287));
    assert_eq!(
        person.kind,
        Some(rs_plugin_common_interfaces::domain::person::PersonType::Actor)
    );
    assert_eq!(serde_json::to_value(person).unwrap()["type"], "Actor");
}

#[test]
fn test_lookup_unlimited_cast_keeps_crew_selection_for_movies_and_shows() {
    use rs_plugin_common_interfaces::domain::person::PersonType;
    let cases = [
        (
            RsLookupQuery::Movie(RsLookupMovie {
                ids: Some(RsIds::from_tmdb(550)),
                ..Default::default()
            }),
            PersonType::Director,
            11, // Fight Club has more than ten actors; reject the previous cap.
        ),
        (
            RsLookupQuery::Serie(RsLookupSerie {
                ids: Some(RsIds::from_tmdb(1396)),
                ..Default::default()
            }),
            PersonType::Creator,
            1, // Show main-cast lists may contain fewer than eleven actors.
        ),
    ];
    let mut plugin = build_plugin();
    for (query, expected_crew, minimum_actors) in cases {
        let results = call_lookup(
            &mut plugin,
            &RsLookupWrapper {
                query,
                credential: None,
                params: None,
            },
        );
        let people = results
            .results
            .into_iter()
            .find_map(|result| {
                result
                    .relations
                    .and_then(|relations| relations.people_details)
            })
            .expect("Expected people");
        assert!(people
            .iter()
            .any(|credit| credit.person.kind.as_ref() == Some(&expected_crew)));
        let actors = people
            .iter()
            .filter(|credit| credit.person.kind == Some(PersonType::Actor))
            .count();
        assert!(
            actors >= minimum_actors,
            "Expected at least {minimum_actors} actors, got {actors}"
        );
        let unique_ids: std::collections::HashSet<_> =
            people.iter().map(|credit| &credit.person.id).collect();
        assert_eq!(
            unique_ids.len(),
            people.len(),
            "Credits must not duplicate people"
        );
        assert!(people
            .iter()
            .all(|credit| credit.person.kind == Some(PersonType::Actor)
                || credit.person.kind.as_ref() == Some(&expected_crew)));
    }
}

#[test]
fn test_lookup_movies_by_director_filter() {
    use rs_plugin_common_interfaces::domain::person::PersonType;

    let mut plugin = build_plugin();
    let results = call_lookup(
        &mut plugin,
        &RsLookupWrapper {
            query: RsLookupQuery::Movie(RsLookupMovie {
                people: Some(vec![RsLookupPersonFilter {
                    name: Some("David Fincher".into()),
                    role: Some(PersonType::Director),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            credential: None,
            params: None,
        },
    );

    assert!(!results.results.is_empty(), "Expected David Fincher movies");
    assert!(results.results.iter().all(|result| {
        result
            .relations
            .as_ref()
            .and_then(|relations| relations.people_details.as_ref())
            .is_some_and(|people| {
                people.iter().any(|credit| {
                    credit.person.tmdb == Some(7467)
                        && credit.person.kind == Some(PersonType::Director)
                })
            })
    }));
}

#[test]
fn test_lookup_relationship_filters_accept_optional_roles_and_external_ids() {
    let mut plugin = build_plugin();

    let broad_person_results = call_lookup(
        &mut plugin,
        &RsLookupWrapper {
            query: RsLookupQuery::Serie(RsLookupSerie {
                ids: Some(RsIds::from_tmdb(1396)),
                people: Some(vec![RsLookupPersonFilter {
                    ids: Some(provider_ids("tmdb-person", 17419)),
                    role: None,
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            credential: None,
            params: None,
        },
    );
    assert_eq!(broad_person_results.results.len(), 1);

    let collection_and_tag_results = call_lookup(
        &mut plugin,
        &RsLookupWrapper {
            query: RsLookupQuery::Movie(RsLookupMovie {
                ids: Some(RsIds::from_tmdb(11)),
                series: Some(vec![RsLookupSerieFilter {
                    ids: Some(provider_ids("tmdb-collection", 10)),
                    ..Default::default()
                }]),
                tags: Some(vec![RsLookupTagFilter {
                    ids: Some(provider_ids("tmdb-genre", 12)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            credential: None,
            params: None,
        },
    );
    let relations = collection_and_tag_results
        .results
        .first()
        .and_then(|result| result.relations.as_ref())
        .expect("Expected Star Wars collection and genre relations");
    assert!(relations
        .series_details
        .as_ref()
        .is_some_and(|series| series.iter().any(|serie| serie.tmdb == Some(10))));
    assert!(relations
        .tags_details
        .as_ref()
        .is_some_and(|tags| tags.iter().any(|tag| tag.id == "tmdb-genre:12")));
}

#[test]
fn test_lookup_movies_by_tag_and_collection_filters() {
    let mut plugin = build_plugin();

    let tagged = call_lookup(
        &mut plugin,
        &RsLookupWrapper {
            query: RsLookupQuery::Movie(RsLookupMovie {
                tags: Some(vec![RsLookupTagFilter {
                    ids: Some(provider_ids("tmdb-genre", 99)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            credential: None,
            params: None,
        },
    );
    assert!(!tagged.results.is_empty(), "Expected documentary movies");
    assert!(tagged.results.iter().all(|result| {
        result
            .relations
            .as_ref()
            .and_then(|relations| relations.tags_details.as_ref())
            .is_some_and(|tags| tags.iter().any(|tag| tag.id == "tmdb-genre:99"))
    }));

    let collection = call_lookup(
        &mut plugin,
        &RsLookupWrapper {
            query: RsLookupQuery::Movie(RsLookupMovie {
                series: Some(vec![RsLookupSerieFilter {
                    ids: Some(provider_ids("tmdb-collection", 10)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            credential: None,
            params: None,
        },
    );
    assert!(!collection.results.is_empty(), "Expected Star Wars movies");
    assert!(collection.results.iter().all(|result| {
        result
            .relations
            .as_ref()
            .and_then(|relations| relations.series_details.as_ref())
            .is_some_and(|series| series.iter().any(|serie| serie.tmdb == Some(10)))
    }));
}

#[test]
fn test_infos_version_matches_package_release() {
    let mut plugin = build_plugin();
    let output = plugin
        .call::<&str, &[u8]>("infos", "")
        .expect("infos call failed");
    let info: rs_plugin_common_interfaces::PluginInformation =
        serde_json::from_slice(output).expect("Invalid plugin information");
    // This plugin's 0.N.0 releases advertise N to the host.
    let release_version: u16 = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
    assert_eq!(info.version, release_version);
}
