<div align="center">
  <h1>🔥 SpotFlamer</h1>
  <p>
    <b>Téléchargez de la musique depuis Spotify & YouTube en haute qualité, sans aucune configuration.</b>
  </p>
  <p>
    <a href="https://github.com/rust-lang/rust"><img src="https://img.shields.io/badge/Made_with-Rust-orange?style=flat-square&logo=rust" alt="Made with Rust" /></a>
    <img src="https://img.shields.io/badge/UI-egui-blue?style=flat-square" alt="egui UI" />
    <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey?style=flat-square" alt="Platforms" />
  </p>
</div>

---

SpotFlamer est une application de bureau moderne, légère et minimaliste (Dark Mode) développée en Rust. Elle permet de télécharger vos titres, albums ou playlists depuis **Spotify** (ou vos vidéos musicales depuis **YouTube**) en MP3 320 kbps.

**Zero Configuration** : Collez simplement votre lien, SpotFlamer s'occupe de tout. L'application récupère automatiquement la pochette de l'album et les tags officiels (titre, artiste, n° de piste) sans que vous ayez besoin de créer un compte développeur Spotify.

## ✨ Fonctionnalités

*   **100% Autonome** : Pas d'API key requise. L'app extrait publiquement les métadonnées de Spotify.
*   **Téléchargements par lot** : Support des playlists et des albums complets.
*   **Qualité MP3 optimale** : Convertit l'audio en **320 kbps** via `ffmpeg`.
*   **Métadonnées propres** : Injecte les tags ID3v2 officiels et la pochette d'album (`CoverFront`) dans le fichier MP3 final.
*   **Performance asynchrone** : Télécharge et convertit jusqu'à 3 musiques simultanément grâce à `tokio`.
*   **Score intelligent** : Analyse et filtre intelligemment les résultats YouTube (pénalise les "Cover", "Live", "Remix" pour ne garder que la piste officielle).
*   **Interface fluide** : UI moderne avec animations et micro-interactions en `egui`.

---

## 🚀 Installation & Utilisation

SpotFlamer s'appuie sur `yt-dlp` et `ffmpeg` pour extraire et convertir les flux audio. Pour que l'application soit totalement portable, ces deux outils doivent se trouver **dans le même dossier** que SpotFlamer.

### 1. Prérequis

1.  **yt-dlp** : Téléchargez l'exécutable (`yt-dlp.exe` pour Windows) depuis les [releases de yt-dlp](https://github.com/yt-dlp/yt-dlp/releases).
2.  **FFmpeg** : Téléchargez l'exécutable (`ffmpeg.exe` pour Windows) depuis le [site officiel de FFmpeg](https://ffmpeg.org/download.html) ou via [Gyan](https://www.gyan.dev/ffmpeg/builds/) (prenez l'archive de release et extrayez le fichier situé dans `bin/ffmpeg.exe`).

### 2. Organisation des fichiers

Placez les fichiers de la manière suivante dans votre dossier :

```text
📁 SpotFlamer-Folder/
├── 🎵 spotflamer.exe  (L'application)
├── ⬇️ yt-dlp.exe      (Pour le téléchargement)
└── ⚙️ ffmpeg.exe      (Pour la conversion MP3)
```

### 3. Lancer l'application

Double-cliquez sur `spotflamer.exe`. 
Dans l'interface :
1.  **Collez un lien** : Spotify (Titre, Album, Playlist) ou YouTube.
2.  Appuyez sur `Entrée` ou cliquez sur **Télécharger**.
3.  Admirez la file d'attente s'animer (Recherche ➔ Téléchargement ➔ Conversion ➔ Métadonnées).
4.  Retrouvez vos MP3 directement dans le dossier de destination !

> ⚙️ **Paramètres** : Vous pouvez modifier le dossier de destination des MP3 ou l'ajout du numéro de piste dans le nom du fichier en cliquant sur l'engrenage en haut à droite.

---

## 🛠️ Compiler depuis les sources

Si vous souhaitez modifier SpotFlamer ou le compiler vous-même pour votre système :

```bash
# 1. Cloner le dépôt
git clone https://github.com/votre-nom/SpotFlamer.git
cd SpotFlamer

# 2. Compiler en mode release (optimisé)
cargo build --release

# 3. L'exécutable se trouvera dans ./target/release/
```
*(N'oubliez pas de placer `ffmpeg` et `yt-dlp` à côté de l'exécutable généré avant de le lancer).*

---

## 📜 Architecture (Sous le capot)

*   `tokio` : Runtime asynchrone orchestrant le pipeline de téléchargement (Semaphore pour limiter la concurrence).
*   `eframe` / `egui` : Bibliothèque GUI en mode immédiat (Immediate Mode), offrant des interfaces fluides à 60 FPS.
*   `reqwest` : Scraping HTTP asynchrone des pages publiques de Spotify.
*   `id3` : Manipulation et écriture des tags audios.

Le pipeline pour un lien Spotify suit ces étapes : 
1. Scraping métadonnées Spotify ➔ 2. Recherche YouTube via yt-dlp avec algorithme de scoring ➔ 3. Téléchargement brut ➔ 4. Conversion ffmpeg ➔ 5. Injection Tags ID3.

---

## 🤝 Contribuer

Les pull requests sont les bienvenues. Pour les changements majeurs, veuillez d'abord ouvrir une "Issue" pour discuter de ce que vous aimeriez changer.

## ⚖️ Licence

Ce projet est sous licence **MIT**.
