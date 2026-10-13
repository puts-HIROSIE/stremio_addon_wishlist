use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use stremio_wishlist::addon;
use stremio_wishlist::chart::{ChartEntry, parse_chart};
use stremio_wishlist::cinemeta::{
    MetaResponse, Movie, SearchResponse, meta_url, pick_match, search_url,
};
use stremio_wishlist::imdb_id::ImdbId;

const USAGE: &str = "usage :\n  stremio-wishlist import <classement.txt> <catalogue.json>\n  stremio-wishlist build <catalogue.json> <dossier>   (addon en fichiers statiques)\n  stremio-wishlist serve <catalogue.json>   (PORT, défaut 7000)";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["import", input, output] => import(Path::new(input), Path::new(output)).await,
        ["build", catalog, dir] => build(Path::new(catalog), Path::new(dir)),
        ["serve", catalog] => serve(Path::new(catalog)).await,
        _ => bail!("{USAGE}"),
    }
}

/// Texte collé → films résolus via Cinemeta → fichier JSON du catalogue.
async fn import(input: &Path, output: &Path) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(input)
        .with_context(|| format!("lecture de {}", input.display()))?;
    let entries = parse_chart(&text);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("stremio-wishlist/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let mut movies: Vec<Movie> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for entry in &entries {
        let found = match resolve(&client, entry).await {
            Ok(found) => found,
            Err(err) => {
                missing.push(format!(
                    "{} ({}) : erreur réseau ({err})",
                    entry.title, entry.year
                ));
                continue;
            }
        };
        match found {
            Some(movie) if !movies.iter().any(|m| m.id == movie.id) => movies.push(movie),
            Some(movie) => missing.push(format!(
                "{} ({}) : doublon de {}",
                entry.title, entry.year, movie.id
            )),
            None => missing.push(format!("{} ({})", entry.title, entry.year)),
        }
    }

    std::fs::write(output, serde_json::to_string_pretty(&movies)?)
        .with_context(|| format!("écriture de {}", output.display()))?;
    println!(
        "{} / {} films résolus → {}",
        movies.len(),
        entries.len(),
        output.display()
    );
    for line in &missing {
        println!("  non trouvé : {line}");
    }
    Ok(())
}

/// Un `tt…` dans la ligne titre force le film ; sinon recherche par titre + année.
async fn resolve(client: &reqwest::Client, entry: &ChartEntry) -> reqwest::Result<Option<Movie>> {
    if let Some(id) = ImdbId::find_all(&entry.title).first() {
        let meta: MetaResponse = fetch(client, &meta_url(id)).await?;
        return Ok(meta.into_movie());
    }
    let response: SearchResponse = fetch(client, &search_url(entry)).await?;
    Ok(pick_match(&response.metas, entry.year))
}

/// Requête Cinemeta, réessayée 2 fois (le service renvoie parfois des 504).
async fn fetch<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> reqwest::Result<T> {
    let mut attempt = 1;
    loop {
        let result = async {
            client
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }
        .await;
        match result {
            Err(_) if attempt < 3 => attempt += 1,
            other => return other,
        }
    }
}

fn read_catalog(catalog: &Path) -> anyhow::Result<Vec<Movie>> {
    let json = std::fs::read_to_string(catalog)
        .with_context(|| format!("lecture de {}", catalog.display()))?;
    serde_json::from_str(&json).context("catalogue JSON invalide")
}

fn build(catalog: &Path, dir: &Path) -> anyhow::Result<()> {
    let movies = read_catalog(catalog)?;
    addon::write_static(dir, &movies)
        .with_context(|| format!("écriture dans {}", dir.display()))?;
    println!("{} films → {}", movies.len(), dir.display());
    Ok(())
}

async fn serve(catalog: &Path) -> anyhow::Result<()> {
    let movies = read_catalog(catalog)?;
    let port: u16 = std::env::var("PORT")
        .ok()
        .map_or(Ok(7000), |p| p.parse())
        .context("PORT invalide")?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!(
        "{} films · installe http://{addr}/manifest.json dans Stremio",
        movies.len()
    );
    axum::serve(listener, addon::router(Arc::new(movies))).await?;
    Ok(())
}
