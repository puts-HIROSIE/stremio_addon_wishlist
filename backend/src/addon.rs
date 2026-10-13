//! Routes HTTP du protocole addon Stremio : `manifest.json` + catalogue.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router, middleware};
use serde::Serialize;
use serde_json::{Value, json};

use crate::cinemeta::Movie;
use crate::imdb_id::ImdbId;

type Movies = Arc<Vec<Movie>>;

pub const CATALOG_ID: &str = "imdb-top250";

/// Construit le routeur de l'addon à partir des films à servir.
pub fn router(movies: Movies) -> Router {
    Router::new()
        .route("/manifest.json", get(manifest))
        .route("/catalog/{kind}/{file}", get(catalog))
        .with_state(movies)
        // Stremio Web appelle l'addon depuis une autre origine.
        .layer(middleware::map_response(allow_any_origin))
}

async fn allow_any_origin(mut res: Response) -> Response {
    res.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    res
}

async fn manifest() -> Json<Value> {
    Json(manifest_json())
}

pub fn manifest_json() -> Value {
    json!({
        "id": "community.stremio-wishlist",
        "version": env!("CARGO_PKG_VERSION"),
        "name": "Wishlist",
        "description": "Mes listes de films à voir.",
        "resources": ["catalog"],
        "types": ["movie"],
        "catalogs": [{ "type": "movie", "id": CATALOG_ID, "name": "IMDb Top 250" }],
    })
}

/// Aperçu de film au format attendu par Stremio dans un catalogue.
#[derive(Serialize)]
struct MetaPreview<'a> {
    id: &'a ImdbId,
    #[serde(rename = "type")]
    kind: &'static str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    poster: Option<&'a str>,
}

async fn catalog(
    State(movies): State<Movies>,
    Path((kind, file)): Path<(String, String)>,
) -> Response {
    if kind != "movie" || file.strip_suffix(".json") != Some(CATALOG_ID) {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(catalog_json(&movies)).into_response()
}

pub fn catalog_json(movies: &[Movie]) -> Value {
    let metas: Vec<MetaPreview<'_>> = movies
        .iter()
        .map(|m| MetaPreview {
            id: &m.id,
            kind: "movie",
            name: &m.name,
            poster: m.poster.as_deref(),
        })
        .collect();
    json!({ "metas": metas })
}

/// Écrit l'addon en fichiers statiques (mêmes chemins que les routes) pour un hébergement sans serveur.
pub fn write_static(dir: &std::path::Path, movies: &[Movie]) -> std::io::Result<()> {
    let catalog_dir = dir.join("catalog").join("movie");
    std::fs::create_dir_all(&catalog_dir)?;
    std::fs::write(dir.join("manifest.json"), manifest_json().to_string())?;
    std::fs::write(
        catalog_dir.join(format!("{CATALOG_ID}.json")),
        catalog_json(movies).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imdb_id::ImdbId;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    fn movies() -> Arc<Vec<Movie>> {
        Arc::new(vec![
            Movie {
                id: ImdbId::parse("tt0111161").unwrap(),
                name: "The Shawshank Redemption".into(),
                poster: Some("https://m.media-amazon.com/x.jpg".into()),
            },
            Movie {
                id: ImdbId::parse("tt0068646").unwrap(),
                name: "The Godfather".into(),
                poster: None,
            },
        ])
    }

    async fn get(uri: &str) -> (StatusCode, Option<String>, Value) {
        let req = Request::get(uri).body(Body::empty()).unwrap();
        let res = router(movies()).oneshot(req).await.unwrap();
        let status = res.status();
        let cors = res
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .map(|v| v.to_str().unwrap().to_owned());
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, cors, json)
    }

    #[tokio::test]
    async fn manifest_declares_movie_catalog() {
        let (status, cors, json) = get("/manifest.json").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cors.as_deref(), Some("*"));
        assert_eq!(json["resources"], serde_json::json!(["catalog"]));
        assert_eq!(json["types"], serde_json::json!(["movie"]));
        assert_eq!(json["catalogs"][0]["type"], "movie");
        assert_eq!(json["catalogs"][0]["id"], CATALOG_ID);
        assert!(json["id"].is_string() && json["version"].is_string() && json["name"].is_string());
    }

    #[tokio::test]
    async fn catalog_lists_movies_in_order() {
        let (status, cors, json) = get("/catalog/movie/imdb-top250.json").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cors.as_deref(), Some("*"));
        assert_eq!(
            json["metas"],
            serde_json::json!([
                {"id": "tt0111161", "type": "movie", "name": "The Shawshank Redemption", "poster": "https://m.media-amazon.com/x.jpg"},
                {"id": "tt0068646", "type": "movie", "name": "The Godfather"}
            ])
        );
    }

    #[test]
    fn write_static_matches_served_json() {
        let dir =
            std::env::temp_dir().join(format!("stremio-wishlist-test-{}", std::process::id()));
        write_static(&dir, &movies()).unwrap();
        let read = |rel: &str| -> Value {
            serde_json::from_str(&std::fs::read_to_string(dir.join(rel)).unwrap()).unwrap()
        };
        assert_eq!(read("manifest.json"), manifest_json());
        assert_eq!(
            read("catalog/movie/imdb-top250.json"),
            catalog_json(&movies())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn unknown_catalog_or_type_is_404() {
        for uri in [
            "/catalog/movie/autre.json",
            "/catalog/series/imdb-top250.json",
            "/catalog/movie/imdb-top250",
            "/meta/movie/tt0111161.json",
        ] {
            let (status, _, _) = get(uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
    }
}
