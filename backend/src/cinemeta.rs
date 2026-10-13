//! Résolution titre + année → identifiant IMDb via Cinemeta (addon officiel de Stremio).

use serde::{Deserialize, Serialize};

use crate::chart::{ChartEntry, leading_year};
use crate::imdb_id::ImdbId;

/// Un résultat de recherche Cinemeta (seuls les champs utiles).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub poster: Option<String>,
    #[serde(rename = "releaseInfo")]
    pub release_info: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub metas: Vec<SearchHit>,
}

/// Film résolu, stocké puis servi dans le catalogue Stremio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Movie {
    pub id: ImdbId,
    pub name: String,
    pub poster: Option<String>,
}

/// Choisit le résultat qui correspond à l'année : exacte d'abord, puis à ±1 an
/// (les dates de sortie diffèrent parfois d'un pays à l'autre).
pub fn pick_match(hits: &[SearchHit], year: u16) -> Option<Movie> {
    let candidates: Vec<(u16, Movie)> = hits
        .iter()
        .filter_map(|hit| {
            let hit_year = leading_year(hit.release_info.as_deref()?)?;
            let id = ImdbId::parse(&hit.id).ok()?;
            Some((
                hit_year,
                Movie {
                    id,
                    name: hit.name.clone(),
                    poster: hit.poster.clone(),
                },
            ))
        })
        .collect();
    let exact = candidates.iter().find(|(y, _)| *y == year);
    let close = || candidates.iter().find(|(y, _)| y.abs_diff(year) <= 1);
    exact.or_else(close).map(|(_, movie)| movie.clone())
}

/// Réponse de `/meta/movie/{id}.json`.
#[derive(Debug, Deserialize)]
pub struct MetaResponse {
    pub meta: Option<SearchHit>,
}

impl MetaResponse {
    pub fn into_movie(self) -> Option<Movie> {
        let hit = self.meta?;
        let id = ImdbId::parse(&hit.id).ok()?;
        Some(Movie {
            id,
            name: hit.name,
            poster: hit.poster,
        })
    }
}

/// URL de la fiche Cinemeta d'un film.
pub fn meta_url(id: &ImdbId) -> String {
    format!("{BASE_URL}/meta/movie/{id}.json")
}

/// URL de recherche Cinemeta pour un film (titre encodé pour l'URL).
pub fn search_url(entry: &ChartEntry) -> String {
    format!(
        "{BASE_URL}/catalog/movie/top/search={}.json",
        percent_encode(&entry.title)
    )
}

const BASE_URL: &str = "https://v3-cinemeta.strem.io";

/// Encodage d'URL : tout sauf les caractères non réservés (RFC 3986) devient `%XX`.
fn percent_encode(text: &str) -> String {
    use std::fmt::Write;
    text.bytes().fold(String::new(), |mut out, b| {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str, year: &str) -> SearchHit {
        SearchHit {
            id: id.to_owned(),
            name: format!("film {id}"),
            poster: None,
            release_info: Some(year.to_owned()),
        }
    }

    #[test]
    fn picks_exact_year_over_first_result() {
        let hits = [hit("tt0000001", "1997"), hit("tt0038650", "1946")];
        let picked = pick_match(&hits, 1946).map(|r| r.id.to_string());
        assert_eq!(picked.as_deref(), Some("tt0038650"));
    }

    #[test]
    fn exact_year_wins_over_close_year() {
        let hits = [hit("tt0000001", "1995"), hit("tt0000002", "1994")];
        let picked = pick_match(&hits, 1994).map(|r| r.id.to_string());
        assert_eq!(picked.as_deref(), Some("tt0000002"));
    }

    #[test]
    fn falls_back_to_one_year_apart() {
        let hits = [hit("tt0000001", "1990"), hit("tt0000002", "1995")];
        let picked = pick_match(&hits, 1994).map(|r| r.id.to_string());
        assert_eq!(picked.as_deref(), Some("tt0000002"));
    }

    #[test]
    fn none_when_no_year_matches() {
        assert_eq!(pick_match(&[hit("tt0000001", "1980")], 1994), None);
        assert_eq!(pick_match(&[], 1994), None);
    }

    #[test]
    fn skips_hits_with_invalid_id_or_missing_year() {
        let mut no_year = hit("tt0000003", "");
        no_year.release_info = None;
        let hits = [hit("kitsu:123", "1994"), no_year, hit("tt0000004", "1994")];
        let picked = pick_match(&hits, 1994).map(|r| r.id.to_string());
        assert_eq!(picked.as_deref(), Some("tt0000004"));
    }

    #[test]
    fn parses_real_cinemeta_response() {
        // Extrait réel de v3-cinemeta.strem.io (recherche « les evades »), raccourci.
        let json = r#"{"query":"les evades","metas":[{"id":"tt0111161","imdb_id":"tt0111161","type":"movie","name":"The Shawshank Redemption","poster":"https://m.media-amazon.com/x.jpg","releaseInfo":"1994","links":[]}]}"#;
        let resp: SearchResponse = serde_json::from_str(json).unwrap();
        let picked = pick_match(&resp.metas, 1994).unwrap();
        assert_eq!(picked.id.as_str(), "tt0111161");
        assert_eq!(picked.name, "The Shawshank Redemption");
    }

    #[test]
    fn meta_url_targets_movie_endpoint() {
        let id = ImdbId::parse("tt0088763").unwrap();
        assert_eq!(
            meta_url(&id),
            "https://v3-cinemeta.strem.io/meta/movie/tt0088763.json"
        );
    }

    #[test]
    fn parses_real_meta_response() {
        // Extrait réel de v3-cinemeta.strem.io/meta/movie/tt0088763.json, raccourci.
        let json = r#"{"meta":{"id":"tt0088763","imdb_id":"tt0088763","type":"movie","name":"Back to the Future","poster":"https://images.metahub.space/poster/medium/tt0088763/img","releaseInfo":"1985"}}"#;
        let resp: MetaResponse = serde_json::from_str(json).unwrap();
        let movie = resp.into_movie().unwrap();
        assert_eq!(movie.id.as_str(), "tt0088763");
        assert_eq!(movie.name, "Back to the Future");
        assert!(movie.poster.is_some());
    }

    #[test]
    fn empty_or_invalid_meta_gives_none() {
        let empty: MetaResponse = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.into_movie(), None);
        let bad: MetaResponse =
            serde_json::from_str(r#"{"meta":{"id":"kitsu:1","name":"x"}}"#).unwrap();
        assert_eq!(bad.into_movie(), None);
    }

    #[test]
    fn search_url_encodes_title() {
        let entry = ChartEntry {
            title: "Le Bon, la Brute & le Truand".into(),
            year: 1966,
        };
        assert_eq!(
            search_url(&entry),
            "https://v3-cinemeta.strem.io/catalog/movie/top/search=Le%20Bon%2C%20la%20Brute%20%26%20le%20Truand.json"
        );
    }
}
