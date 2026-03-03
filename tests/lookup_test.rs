use extism::*;
use rs_plugin_common_interfaces::{
    domain::external_images::ExternalImage,
    lookup::{
        RsLookupMetadataResult, RsLookupMetadataResults, RsLookupMovie, RsLookupPerson,
        RsLookupQuery, RsLookupSerie, RsLookupWrapper,
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

#[test]
fn test_lookup_no_credential_uses_default_key() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some("Fight Club".to_string()),
            ids: None,
            page_key: None,
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
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(
        !results.results.is_empty(),
        "Expected result for tmdb:550"
    );

    let first = &results.results[0];
    let movie = match &first.metadata {
        RsLookupMetadataResult::Movie(movie) => movie,
        _ => panic!("Expected Movie metadata"),
    };

    assert_eq!(movie.tmdb, Some(550), "Expected tmdb ID 550");
    assert!(movie.imdb.is_some(), "Expected IMDB ID for detail lookup");
    assert!(movie.duration.is_some(), "Expected runtime for detail lookup");
    println!(
        "Direct ID lookup: {} (imdb: {:?}, runtime: {:?})",
        movie.name, movie.imdb, movie.duration
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
            name: Some("tmdb-tv:1396".to_string()),
            ids: None,
            page_key: None,
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    assert!(
        !results.results.is_empty(),
        "Expected result for tmdb-tv:1396"
    );

    let first = &results.results[0];
    let serie = match &first.metadata {
        RsLookupMetadataResult::Serie(serie) => serie,
        _ => panic!("Expected Serie metadata"),
    };

    assert_eq!(serie.tmdb, Some(1396), "Expected tmdb ID 1396");
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
            ids: None,
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
            println!("Got {} results for person tmdb:5719226", results.results.len());
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
fn test_lookup_empty_movie_name_returns_error() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Movie(RsLookupMovie {
            name: Some(String::new()),
            ids: None,
            page_key: None,
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
