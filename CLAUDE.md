# stremio-wishlist
Addon Stremio perso : importer des films (texte collé, liens IMDb) et les servir comme catalogue sur mon compte Stremio.

## Commandes (depuis `backend/`)
- Vérifier : `cargo clippy --all-targets -q --message-format=short -- -D warnings`
- Tester : `cargo nextest run --status-level fail` · doctests : `cargo test --doc -q`
- Importer : `cargo run -q -- import data/imdb-top250.txt data/imdb-top250.json`
- Lancer : `cargo run -q -- serve data/imdb-top250.json` → installer `http://127.0.0.1:7000/manifest.json` dans Stremio
- Audit dépendances : `cargo deny check`

## Architecture
- `backend/src/chart.rs` — texte collé d'un classement IMDb → (titre, année).
- `backend/src/imdb_id.rs` — newtype `ImdbId` validé + extraction depuis texte/lien.
- `backend/src/cinemeta.rs` — recherche Cinemeta, choix du résultat par année, type `Movie` stocké.
- `backend/src/addon.rs` — routes Stremio (`/manifest.json`, `/catalog/movie/imdb-top250.json`), CORS `*`.
- `backend/src/main.rs` — CLI `import` / `serve`.
- `backend/data/` — listes sources (`.txt`) et catalogues résolus (`.json`).

## Décisions
- Pas de scraping IMDb (anti-bot + CGU) : l'utilisateur colle le texte de la page.
- Cinemeta (`v3-cinemeta.strem.io`) pour titre → ID : gratuit, sans clé, accepte les titres français.
- Correspondance par année exacte puis ±1 ; les années Cinemeta diffèrent parfois d'IMDb FR.
- Pas de SDK addon : le protocole est du JSON simple servi par axum.
- `reqwest` 0.13 (TLS rustls par défaut).

## Spécificités
- Routes addon : CORS `Access-Control-Allow-Origin: *` obligatoire (Stremio Web), exception à la règle « même origine ».
- Stremio exige HTTPS hors localhost : à prévoir pour l'accès depuis le téléphone/TV.
