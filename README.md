# Rustine

Socle d'applications Android en Rust : interface tactile moderne et 3D en
OpenGL ES 3, dans un APK de moins de 300 Ko.

![Interface de Rustine : panneaux latéraux, pastilles et cube 3D](docs/apercu.png)

## Télécharger

**[Télécharger l'APK (version 0.1)](https://github.com/s1pierro/Rustine/releases/download/v0.1/rustine-0.1.apk)**

Pour les appareils ARM 64 bits, sous Android 8.0 ou plus récent, avec OpenGL ES 3.
L'APK étant signé avec une clé de debug, Android demandera d'autoriser
l'installation depuis une source inconnue.

## Fonctions

- Espace de travail qui s'adapte aux panneaux, la 3D suit sa forme
- Surface de travail partagée en fenêtres, chacune avec sa vue (3D, liste,
  aide, paramètres) et une seule barre en bas : icône et titre de la vue, ligne
  d'état, poignée. Glisser la pastille + partage la fenêtre, toucher le titre
  change sa vue ou la ferme, la poignée la redimensionne et montre par un tiret
  chaque bord qu'elle peut déplacer
- Vue Paramètres : taille commune des pastilles, pilules, boutons et onglets,
  arrondi, taille du texte, hauteur des lignes, thème sombre et couleur d'accent
- Réglages de l'interface et disposition des fenêtres conservés dans un fichier
  texte modifiable à la main, `Rustine/interface.txt` à la racine du stockage
  partagé, relu au retour dans l'application
- Accès à tous les fichiers demandé au lancement (JNI, sans code Java) ; refusé,
  les réglages restent dans le dossier privé de l'application
- Panneaux latéraux à onglets : gauche et droite en paysage, haut et bas en
  portrait, redimensionnables au doigt
- Pastilles flottantes qui déploient les panneaux
- Bibliothèque tactile : pilules, onglets, listes défilantes, interrupteurs,
  curseurs et boutons
- Chaque doigt reste à ce qu'il a touché en premier ; hors des boutons, un doigt
  tourne la vue 3D qu'il touche, deux doigts zooment et déplacent
- Rien n'est redessiné tant que rien ne bouge

## Organisation

- `src/ui.rs`, `shell.rs`, `surface.rs`, `widgets.rs`, `reglages.rs`, `draw.rs`, `text.rs` : l'interface, sans
  aucun appel OpenGL, donc testable partout
- `src/gles.rs`, `renderer.rs`, `android.rs`, `acces.rs` : OpenGL ES et Android
- `src/demo.rs` : l'application de démonstration
- `examples/apercu.rs` : dessine l'interface hors écran sur le processeur
  graphique de l'appareil et écrit des images

OpenGL est chargé à l'exécution avec `dlopen`. La seule dépendance Rust est
[fontdue](https://crates.io/crates/fontdue), pour le texte, dessiné à partir des
polices du système.

## Construire l'APK

Le build se fait directement sur Android, dans [Termux](https://termux.dev),
avec le Rust natif de Termux :

    pkg install rust aapt apksigner zip unzip curl openjdk-17
    bash build.sh

L'APK est produit dans `dist/rustine.apk`. La compilation a lieu dans
`~/.cache/rust-apk/rustine`, pour fonctionner aussi depuis le stockage partagé,
où rien ne peut s'exécuter.

## Licence

Rustine est réutilisable, modifiable et redistribuable à des fins non
commerciales, selon la [PolyForm Noncommercial License 1.0.0](LICENSE.md). La
mention de copyright et la licence doivent accompagner chaque copie. Pour un
usage commercial, contactez l'auteur.
