# SQLite Workbench

> **[Português](README.pt-PT.md)** · **[English](README.md)** · **Français**

Application de bureau pour explorer et éditer des bases de données SQLite, développée en Rust avec [egui](https://github.com/emilk/egui).

## Fonctionnalités

- **Ouvrir des bases de données** — fichiers `.db`, `.sqlite`, `.sqlite3`
- **Onglet Données** — parcourir, filtrer, trier et paginer les enregistrements ; édition, insertion et suppression en ligne
- **Onglet Schéma** — afficher l'instruction SQL CREATE de chaque table
- **Onglet Statistiques** — statistiques par colonne (nombre de lignes, valeurs nulles, valeurs uniques, min/max/moyenne, valeurs les plus fréquentes) ; export en JSON
- **Éditeur SQL** — exécuter des requêtes libres, autocomplétion des mots-clés et des noms de tables/colonnes, raccourci Ctrl+Entrée, historique et requêtes favorites
- **Import CSV** — charger un fichier CSV dans une nouvelle table
- **Export CSV / JSON** — exporter la vue courante ou les résultats d'une requête SQL
- **Créer une table** — fenêtre de dialogue visuelle avec définition des colonnes et prise en charge des clés étrangères
- **Renommer / supprimer des tables** — via le menu contextuel (clic droit)
- **Vue MRD** — diagramme entité-association affichant les tables et les clés étrangères
- **Compacter** — optimiser et réduire la taille du fichier de base de données (VACUUM)
- **Thème sombre / clair** — basculable en cours d'exécution
- **Interface multilingue** — Portugais et Anglais

## Prérequis

- [Rust](https://rustup.rs/) 1.75 ou supérieur

## Compilation

```bash
cargo build --release
```

Le binaire est généré dans `target/release/sqlite-workbench-rs`.

## Lancement

```bash
cargo run --release
```

Ou exécuter le binaire directement et ouvrir une base de données via **Fichier → Charger la base de données**.

## Raccourcis clavier

| Raccourci | Action |
|-----------|--------|
| `Ctrl+Entrée` | Exécuter la requête SQL |

## Paramètres

Les paramètres (langue, thème, lignes par page) sont sauvegardés dans `settings.json` à côté du binaire. L'historique des requêtes est stocké dans `history.json` et les favoris dans `favorites.json`.

## Licence

Distribué sous la [Licence Publique de l'Union Européenne v1.2](LICENSE.fr) (EUPL-1.2).
