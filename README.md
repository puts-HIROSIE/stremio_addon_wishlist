# Stremio Wishlist

Addon Stremio perso qui affiche une liste de films dans Stremio, sur tous les appareils liés au compte.
Aujourd'hui la liste contient le **top 250 IMDb**. On peut y ajouter des films.

- Addon : `https://puts-hirosie.github.io/stremio_addon_wishlist/manifest.json`
- Hébergement : GitHub Pages (fichiers statiques, aucun serveur).
- ⚠️ **Films uniquement** pour l'instant. Les séries ne sont pas encore prises en charge.

---

## 1. Installer l'addon dans Stremio (une seule fois)

1. Ouvre Stremio, puis **Addons** (icône puzzle).
2. Colle l'URL du manifest dans la barre de recherche des addons :
   `https://puts-hirosie.github.io/stremio_addon_wishlist/manifest.json`
3. Clique sur **Installer**.
4. Sur l'écran d'accueil (**Board**) ou dans **Découvrir → Films**, le catalogue **IMDb Top 250** apparaît.

L'addon est lié à ton compte Stremio : il apparaît automatiquement sur ton téléphone, ta TV, etc.

---

## 2. Trouver l'identifiant IMDb d'un film

Chaque film a un identifiant IMDb du type `tt0111161`.

1. Cherche le film sur [imdb.com](https://www.imdb.com).
2. Regarde l'URL de sa page : `https://www.imdb.com/title/tt0111161/` → l'identifiant est **`tt0111161`**.

---

## 3. Ajouter un film depuis le téléphone (sans ordinateur)

On modifie directement le catalogue publié : `backend/data/imdb-top250.json`.

1. Ouvre le dépôt sur GitHub (navigateur ou app GitHub) :
   `https://github.com/puts-HIROSIE/stremio_addon_wishlist`
2. Va dans `backend/data/imdb-top250.json`, puis touche l'icône **crayon ✏️** (*Edit this file*).
3. Descends à la fin du fichier. Le dernier film ressemble à ceci :
   ```json
     {
       "id": "tt1954470",
       "name": "Gangs of Wasseypur",
       "poster": "https://…"
     }
   ]
   ```
4. Ajoute **une virgule** après l'accolade `}` du dernier film, puis ton film avant le `]` final :
   ```json
     {
       "id": "tt1954470",
       "name": "Gangs of Wasseypur",
       "poster": "https://…"
     },
     {
       "id": "tt0166924",
       "name": "Mulholland Drive",
       "poster": "https://images.metahub.space/poster/medium/tt0166924/img"
     }
   ]
   ```
   - `id` : l'identifiant IMDb (étape 2).
   - `name` : le titre affiché dans Stremio (libre).
   - `poster` : remplace seulement l'identifiant dans l'URL `https://images.metahub.space/poster/medium/<id>/img`.
5. Touche **Commit changes…**, garde *Commit directly to the `main` branch*, puis valide.
6. Attends la publication : onglet **Actions** du dépôt, le workflow **Pages** doit passer au vert ✅ (≈ 1 à 2 min).
7. Dans Stremio, recharge le catalogue. Il peut falloir **jusqu'à ~10 min** (cache de GitHub Pages).

> Pièges fréquents :
> - Une virgule oubliée (ou en trop avant `]`) casse le fichier. Le workflow **Pages** échoue alors, et l'ancienne liste reste en ligne. Corrige la virgule et recommite.
> - L'ordre du fichier = l'ordre dans Stremio. Un film ajouté à la fin apparaît en dernier.
> - Un film déjà présent ne doit pas être ajouté deux fois (cherche son `tt…` dans le fichier avant).

---

## 4. Ajouter des films depuis l'ordinateur (titres ou liens, en lot)

Plus pratique pour ajouter beaucoup de films d'un coup. L'outil retrouve tout seul l'identifiant, le titre et le poster.

**Prérequis** : Rust installé, dépôt cloné :
```sh
git clone https://github.com/puts-HIROSIE/stremio_addon_wishlist.git
cd stremio_addon_wishlist/backend
```

1. Ouvre `backend/data/imdb-top250.txt`. Chaque film tient sur **3 lignes** : `#rang`, titre, année.
2. Ajoute tes films à la fin, au choix :
   ```text
   #251
   Le Cinquième Élément
   1997
   #252
   Mulholland Drive https://www.imdb.com/title/tt0166924/
   2001
   #253
   Drive tt0780504
   2011
   ```
   - **Titre seul** : recherche automatique par titre + année (le titre français marche).
   - **Titre + lien IMDb ou `tt…`** : le film est pris tel quel, sans recherche. À utiliser si un titre n'est pas trouvé ou si le mauvais film sort.
   - L'année est **obligatoire** dans les deux cas : sans elle, le film est ignoré.
3. Résous la liste (appelle Cinemeta, l'API publique de Stremio) :
   ```sh
   cargo run -q -- import data/imdb-top250.txt data/imdb-top250.json
   ```
   Lis le résumé : `N / N films résolus`. Pour chaque ligne `non trouvé`, ajoute le lien IMDb du film (étape 2) et relance.
4. (Optionnel) Teste en local avant de publier :
   ```sh
   cargo run -q -- serve data/imdb-top250.json
   ```
   Puis installe `http://127.0.0.1:7000/manifest.json` dans Stremio Desktop.
5. Publie :
   ```sh
   git add data/imdb-top250.txt data/imdb-top250.json
   git commit -m "feat: ajoute <films>"
   git push
   ```
6. Comme au point 3 : workflow **Pages** au vert, puis jusqu'à ~10 min de cache.

> ⚠️ `import` régénère **tout** le JSON depuis le `.txt`. Un film ajouté à la main dans le JSON (section 3) et absent du `.txt` disparaît. Pense à le reporter dans le `.txt`.

---

## 5. Retirer un film

- **Depuis le téléphone** : supprime son bloc `{ … }` dans `imdb-top250.json`, en gardant les virgules correctes. Puis commit, comme à la section 3.
- **Depuis l'ordinateur** : supprime ses 3 lignes dans `imdb-top250.txt`, relance `import`, puis commit et push.

---

## Comment ça marche

```
imdb-top250.txt ──import (Cinemeta)──▶ imdb-top250.json ──build (GitHub Actions)──▶ GitHub Pages ──▶ Stremio
```

- `import` : titres ou liens → identifiants IMDb, noms, posters.
- `build` : écrit `manifest.json` et `catalog/movie/imdb-top250.json`, les deux seuls fichiers que Stremio lit.
- Les commandes de développement sont dans [CLAUDE.md](CLAUDE.md).
