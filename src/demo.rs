//! Démonstration : l'espace de travail partagé en fenêtres (un cube, une liste, une
//! aide), un panneau de chaque côté, et en bas de l'écran la barre de l'application :
//! ses pastilles aplaties (la rustine, les panneaux), l'heure et la batterie. Le bas de
//! l'écran est réservé aux gestes du système, mais les appuis y restent disponibles.

use crate::draw::{Color, Rect};
use crate::math::{self, Mat4};
use crate::clavier::{self, Clavier, Disposition, Edition};
use crate::reglages::Reglages;
use crate::shell::{Layout, Shell, Side};
use crate::surface::{Changement, FenetreId, Placee, Surface};
use crate::ui::{id, sub_id, Id};
use crate::widgets::{Cx, Icon, Scrolls, Theme, ACCENTS};
use std::collections::HashMap;

const COLORS: [(&str, Color); 14] = [
    ("Indigo", [0x4A, 0x45, 0xE0, 255]),
    ("Bleu", [0x3B, 0x82, 0xF6, 255]),
    ("Ciel", [0x38, 0xBD, 0xF8, 255]),
    ("Turquoise", [0x14, 0xB8, 0xA6, 255]),
    ("Vert", [0x22, 0xC5, 0x5E, 255]),
    ("Olive", [0x84, 0xA9, 0x1C, 255]),
    ("Jaune", [0xEA, 0xB3, 0x08, 255]),
    ("Orange", [0xF9, 0x73, 0x16, 255]),
    ("Rouille", [0xB9, 0x4A, 0x18, 255]),
    ("Rouge", [0xEF, 0x44, 0x44, 255]),
    ("Rose", [0xEC, 0x48, 0x99, 255]),
    ("Violet", [0xA8, 0x55, 0xF7, 255]),
    ("Ardoise", [0x64, 0x74, 0x8B, 255]),
    ("Sable", [0xC8, 0xB0, 0x8A, 255]),
];

const OBJECTS: Id = id("objets");
const DISPLAY: Id = id("affichage");
const HELP: Id = id("aide");
const FIT: Id = id("recadrer");
const SPIN: Id = id("rotation");
const LISTE: Id = id("liste-fenetre");
const AIDE: Id = id("aide-fenetre");

const PARAMS: Id = id("parametres");
const BARRE: Id = id("barre-application");
const RUSTINE: Id = id("rustine");
const ESSAI: Id = id("essai-clavier");

/// Les vues qu'une fenêtre de la surface peut montrer.
pub const VUES: [(&str, Icon); 4] = [("Cube", Icon::Cube), ("Couleurs", Icon::List), ("Aide", Icon::Help), ("Paramètres", Icon::Sliders)];
pub const VUE_CUBE: u32 = 0;
pub const VUE_COULEURS: u32 = 1;
pub const VUE_AIDE: u32 = 2;
pub const VUE_PARAMETRES: u32 = 3;

/// Une vue 3D : son rectangle à l'écran et sa matrice.
#[derive(Clone, Copy)]
pub struct Vue3d {
    pub rect: Rect,
    pub mvp: Mat4,
}

/// Ce que le moteur de rendu doit dessiner : un cube dans chaque vue 3D.
#[derive(Clone)]
pub struct Scene {
    pub workspace: Rect,
    pub vues: Vec<Vue3d>,
    /// Le fond de l'écran, que la 3D laisse voir.
    pub fond: Color,
    pub color: [f32; 3],
    pub tint: f32,
}

#[derive(Clone, Copy)]
struct Camera {
    yaw: f32,
    pitch: f32,
    dist: f32,
    target: [f32; 3],
}

const HOME: Camera = Camera { yaw: 0.65, pitch: 0.45, dist: 6.0, target: [0.0, 0.0, 0.0] };

#[derive(Clone, Copy, PartialEq)]
enum EndContent {
    Display,
    Help,
    APropos,
}

pub struct App {
    pub shell: Shell,
    /// L'apparence de l'interface, réglée dans la vue Paramètres.
    pub theme: Theme,
    /// Le clavier virtuel, et le champ d'essai de la vue Paramètres.
    pub clavier: Clavier,
    essai: Edition,
    /// L'heure (heures, minutes) et la batterie (niveau, en charge), données par le
    /// système ; rien hors d'Android.
    pub heure: Option<(u8, u8)>,
    pub batterie: Option<(u8, bool)>,
    /// Où les réglages sont enregistrés, pour l'indiquer dans la vue Paramètres.
    pub fichier_reglages: Option<String>,
    /// L'espace de travail, partagé en fenêtres.
    pub surface: Surface,
    /// Une caméra par fenêtre 3D.
    cams: HashMap<FenetreId, Camera>,
    end_content: EndContent,
    display_tab: usize,
    color: usize,
    spin: bool,
    speed: f32,
    fov: f32,
    size: f32,
    faces: bool,
    scrolls: Scrolls,
}

impl App {
    pub fn new(dp: f32) -> App {
        App {
            shell: Shell::new(dp),
            theme: Theme::default(),
            fichier_reglages: None,
            heure: None,
            batterie: None,
            clavier: Clavier::default(),
            essai: Edition::default(),
            surface: Surface::new(VUE_CUBE),
            cams: HashMap::new(),
            end_content: EndContent::Display,
            display_tab: 0,
            color: 0,
            spin: false,
            speed: 30.0,
            fov: 45.0,
            size: 1.0,
            faces: true,
            scrolls: Scrolls::new(),
        }
    }

    /// Vrai s'il faut redessiner sans attendre un événement.
    pub fn animating(&self) -> bool {
        self.spin || self.shell.animating() || self.clavier.anime()
    }

    /// Une pastille ouvre son contenu, ou referme le panneau s'il l'affiche déjà.
    fn toggle_end(&mut self, content: EndContent) {
        if self.shell.end.open && self.end_content == content {
            self.shell.end.open = false;
        } else {
            self.end_content = content;
            self.shell.end.open = true;
        }
    }

    /// Les réglages de l'interface à enregistrer : l'apparence et les fenêtres.
    pub fn reglages(&self) -> Reglages {
        Reglages { theme: self.theme, clavier: self.clavier.reglages, fenetres: Some(self.surface.ecrire()) }
    }

    /// Applique des réglages relus. Une disposition illisible garde les fenêtres.
    pub fn appliquer(&mut self, r: Reglages) {
        self.theme = r.theme;
        self.clavier.reglages = r.clavier;
        if let Some(s) = r.fenetres.as_deref().and_then(Surface::lire) {
            if s.racine != self.surface.racine {
                self.surface = s;
                self.cams.clear();
            }
        }
    }

    fn frappes(&mut self, cx: &mut Cx, zone: Rect, dt: f32) {
        for f in self.clavier.saisir(cx, zone, dt) {
            if self.clavier.cible == Some(ESSAI) && self.essai.appliquer(f) {
                self.clavier.fermer();
            }
        }
    }

    fn cam(&mut self, f: FenetreId) -> &mut Camera {
        self.cams.entry(f).or_insert(HOME)
    }

    pub fn frame(&mut self, cx: &mut Cx, screen: Rect, dt: f32) -> Scene {
        cx.theme = self.theme;
        // La barre de l'application, tout en bas
        let hb = cx.commande().min(screen.h / 4.0);
        let barre = Rect::new(screen.x, screen.bottom() - hb, screen.w, hb);
        let screen = Rect::new(screen.x, screen.y, screen.w, screen.h - hb);
        // Le clavier ouvert prend le bas de l'écran, ou flotte par-dessus ; ses frappes
        // vont au champ qui a le focus
        let (screen, zone_clavier) = self.clavier.reserver(cx, screen);
        let flottant = self.clavier.reglages.flottant;
        if !flottant {
            self.frappes(cx, zone_clavier, dt);
        }
        let l = self.shell.layout(screen, dt);
        let placees = self.surface.frame(cx, l.workspace, &VUES);
        for c in self.surface.changements() {
            match c {
                // La nouvelle fenêtre part de la caméra de celle qu'on a partagée
                Changement::Partagee { de, nouvelle } => {
                    let cam = *self.cam(de);
                    self.cams.insert(nouvelle, cam);
                }
                Changement::Fermee(f) => {
                    self.cams.remove(&f);
                }
                Changement::Vue { .. } => {}
            }
        }
        self.gestures(cx);
        if self.spin {
            let a = self.speed.to_radians() * dt;
            for p in placees.iter().filter(|p| p.vue == VUE_CUBE) {
                self.cam(p.id).yaw += a;
            }
        }
        for p in &placees {
            self.fenetre(cx, p);
        }
        self.surface.dessus(cx, &VUES);
        self.barre(cx, barre);

        self.start_panel(cx, &l);
        self.end_panel(cx, &l);
        self.handles(cx, &l, screen);
        // Flottant, le clavier déclare ses touches en dernier : elles passent devant tout
        if flottant {
            self.frappes(cx, zone_clavier, dt);
        }
        self.clavier.dessiner(cx);
        let mut scene = self.scene(l.workspace, &placees);
        scene.fond = cx.couleurs().fond;
        scene
    }

    /// Les gestes de l'espace de travail pilotent la vue 3D où ils ont commencé.
    fn gestures(&mut self, cx: &mut Cx) {
        let g = cx.ui.take_gesture();
        let Some(p) = g.at.and_then(|(x, y)| self.surface.fenetre_en(x, y).copied()).filter(|p| p.vue == VUE_CUBE) else { return };
        let fov = self.fov;
        let c = self.cam(p.id);
        c.yaw -= g.orbit.0 * 0.008;
        c.pitch = (c.pitch + g.orbit.1 * 0.008).clamp(-1.5, 1.5);
        c.dist = (c.dist / g.zoom).clamp(1.8, 24.0);
        // Déplacement à l'échelle de l'image : le cube suit les doigts.
        let k = 2.0 * c.dist * (fov.to_radians() / 2.0).tan() / p.contenu.h.max(1.0);
        let (sy, cy, sp, cp) = (c.yaw.sin(), c.yaw.cos(), c.pitch.sin(), c.pitch.cos());
        let (right, up) = ([cy, 0.0, -sy], [-sy * sp, cp, -cy * sp]);
        for i in 0..3 {
            c.target[i] += -g.pan.0 * k * right[i] + g.pan.1 * k * up[i];
        }
    }

    /// Le contenu d'une fenêtre, selon sa vue, et sa ligne d'état.
    fn fenetre(&mut self, cx: &mut Cx, p: &Placee) {
        let (d, col) = (cx.dp, cx.couleurs());
        let k = p.id as usize;
        let r = p.contenu;
        let etat = match p.vue {
            VUE_CUBE => {
                let (gap, m, h) = (cx.ecart(), cx.coin(), cx.commande());
                let (w1, w2) = (cx.pill_width("Recadrer"), cx.pill_width("Rotation"));
                if r.h > 3.0 * h && r.w > w1 + w2 + gap + 2.0 * m {
                    let x = r.cx() - (w1 + w2 + gap) / 2.0;
                    let y = r.bottom() - h - m;
                    if cx.pill(sub_id(FIT, k), x, y, "Recadrer", false) {
                        self.cams.insert(p.id, HOME);
                    }
                    if cx.pill(sub_id(SPIN, k), x + w1 + gap, y, "Rotation", self.spin) {
                        self.spin = !self.spin;
                    }
                }
                format!("1 objet · 12 tri · n°{}", p.id)
            }
            VUE_COULEURS => {
                cx.dl.rrect(r, 0.0, col.surface);
                let scroll = sub_id(LISTE, k);
                let mut b = cx.body(scroll, r, &self.scrolls);
                for (i, (name, color)) in COLORS.iter().enumerate() {
                    if cx.row(&mut b, sub_id(scroll, i), name, Some(*color), i == self.color) {
                        self.color = i;
                    }
                }
                cx.body_end(b, &mut self.scrolls);
                format!("{} couleurs · {}", COLORS.len(), COLORS[self.color].0)
            }
            VUE_PARAMETRES => {
                cx.dl.rrect(r, 0.0, col.surface);
                self.parametres(cx, sub_id(PARAMS, k), r);
                "Apparence de l'interface".to_string()
            }
            _ => {
                cx.dl.rrect(r, 0.0, col.surface);
                let mut b = cx.body(sub_id(AIDE, k), r, &self.scrolls);
                cx.section(&mut b, "FENÊTRES");
                cx.paragraph(&mut b, "Attrapez la pastille + en haut à gauche d'une fenêtre et glissez-la pour la partager : vers le côté, les fenêtres se placent côte à côte ; vers le haut ou le bas, l'une sur l'autre.");
                cx.paragraph(&mut b, "La poignée du coin bas droit déplace les bords de la fenêtre : un tiret montre chaque bord qu'elle peut déplacer.");
                cx.paragraph(&mut b, "Touchez l'icône ou le titre pour choisir ce que montre la fenêtre, ou la fermer.");
                cx.section(&mut b, "GESTES");
                cx.paragraph(&mut b, "Un doigt sur une vue 3D la fait tourner ; deux doigts zooment et déplacent.");
                cx.body_end(b, &mut self.scrolls);
                "Aide".to_string()
            }
        };
        // La ligne d'état, à droite du titre, en chasse fixe
        let e = p.etat;
        cx.clip(Some(e));
        cx.mono_at(e.x + 4.0 * d, e.cy(), &etat, 12.0, col.pale);
        cx.clip(None);
    }

    /// La vue Paramètres : l'apparence, appliquée dès l'image suivante.
    fn parametres(&mut self, cx: &mut Cx, scroll: Id, r: Rect) {
        let t = &mut self.theme;
        let mut b = cx.body(scroll, r, &self.scrolls);
        cx.section(&mut b, "COMMANDES");
        let shown = format!("{:.0} dp", t.commande);
        cx.slider(&mut b, sub_id(scroll, 0), "Taille des commandes", &shown, &mut t.commande, 36.0, 64.0);
        t.commande = t.commande.round();
        let shown = format!("{:.0} %", t.arrondi * 100.0);
        cx.slider(&mut b, sub_id(scroll, 1), "Arrondi", &shown, &mut t.arrondi, 0.0, 1.0);
        let shown = if t.marge < 0.5 { "aplati".to_string() } else { format!("{:.0} dp", t.marge) };
        cx.slider(&mut b, sub_id(scroll, 30), "Marges", &shown, &mut t.marge, 0.0, 20.0);
        t.marge = t.marge.round();
        let shown = format!("{:.0} dp", t.padding);
        cx.slider(&mut b, sub_id(scroll, 31), "Padding", &shown, &mut t.padding, 4.0, 24.0);
        t.padding = t.padding.round();
        cx.section(&mut b, "LECTURE");
        let shown = format!("{:.0} %", t.texte * 100.0);
        cx.slider(&mut b, sub_id(scroll, 2), "Taille du texte", &shown, &mut t.texte, 0.85, 1.3);
        t.texte = (t.texte * 20.0).round() / 20.0;
        let shown = format!("{:.0} dp", t.ligne);
        cx.slider(&mut b, sub_id(scroll, 3), "Hauteur des lignes", &shown, &mut t.ligne, 40.0, 64.0);
        t.ligne = t.ligne.round();
        cx.section(&mut b, "COULEURS");
        cx.toggle(&mut b, sub_id(scroll, 4), "Thème sombre", &mut t.sombre);
        for (i, (nom, clair, fonce)) in ACCENTS.iter().enumerate() {
            let teinte = if t.sombre { *fonce } else { *clair };
            if cx.row(&mut b, sub_id(scroll, 10 + i), nom, Some(teinte), i == t.accent) {
                t.accent = i;
            }
        }
        if cx.button(&mut b, sub_id(scroll, 5), "Rétablir l'apparence") {
            *t = Theme::default();
        }
        cx.section(&mut b, "CLAVIER");
        let c = &mut self.clavier.reglages;
        for (i, disp) in Disposition::TOUTES.iter().enumerate() {
            if cx.row(&mut b, sub_id(scroll, 20 + i), disp.nom(), None, *disp == c.disposition) {
                c.disposition = *disp;
            }
        }
        let shown = format!("{:.0} dp", c.touche);
        cx.slider(&mut b, sub_id(scroll, 6), "Hauteur des touches", &shown, &mut c.touche, 40.0, 64.0);
        c.touche = c.touche.round();
        let shown = format!("{:.0} ms", c.appui_long * 1000.0);
        cx.slider(&mut b, sub_id(scroll, 7), "Appui long (variantes)", &shown, &mut c.appui_long, 0.2, 0.8);
        c.appui_long = (c.appui_long * 100.0).round() / 100.0;
        cx.toggle(&mut b, sub_id(scroll, 8), "Aperçu des touches", &mut c.apercu);
        cx.toggle(&mut b, sub_id(scroll, 9), "Clavier flottant", &mut c.flottant);
        clavier::champ(cx, &mut b, ESSAI, "Essayer le clavier…", &mut self.essai, &mut self.clavier);
        cx.section(&mut b, "FICHIER");
        let ou = match &self.fichier_reglages {
            Some(f) => format!("Ces réglages sont enregistrés dans {f}, un fichier texte qu'on peut modifier à la main."),
            None => "Ces réglages ne sont pas enregistrés.".to_string(),
        };
        cx.paragraph(&mut b, &ou);
        cx.body_end(b, &mut self.scrolls);
    }

    /// La barre de l'application : pastilles aplaties à gauche (la rustine, puis les
    /// panneaux), l'heure et la batterie à droite.
    fn barre(&mut self, cx: &mut Cx, r: Rect) {
        if r.h < 1.0 {
            return;
        }
        let (d, col) = (cx.dp, cx.couleurs());
        cx.ui.region_add(BARRE, r, crate::ui::Sense::Sink);
        cx.dl.rrect(r, 0.0, col.surface);
        cx.dl.rrect(Rect::new(r.x, r.y, r.w, d.max(1.0)), 0.0, col.ligne);
        let case = |i: f32| Rect::new(r.x + i * r.h, r.y, r.h, r.h);
        let (end, content) = (self.shell.end.open, self.end_content);
        if cx.pastille_plate(RUSTINE, case(0.0), Icon::Rustine, end && content == EndContent::APropos) {
            self.toggle_end(EndContent::APropos);
        }
        if cx.pastille_plate(OBJECTS, case(1.0), Icon::List, self.shell.start.open) {
            self.shell.start.toggle();
        }
        if cx.pastille_plate(DISPLAY, case(2.0), Icon::Sliders, end && content == EndContent::Display) {
            self.toggle_end(EndContent::Display);
        }
        if cx.pastille_plate(HELP, case(3.0), Icon::Help, end && content == EndContent::Help) {
            self.toggle_end(EndContent::Help);
        }
        // À droite : la batterie, puis l'heure tout au bord
        let mut x = r.right() - cx.pad();
        if let Some((h, m)) = self.heure {
            let s = format!("{h:02}:{m:02}");
            let w = cx.measure(&s, 15.0);
            x -= w;
            cx.text_at(x, r.cy(), &s, 15.0, col.texte);
            x -= cx.pad();
        }
        if let Some((niveau, charge)) = self.batterie {
            let s = format!("{niveau} %");
            let w = cx.measure(&s, 13.0);
            x -= w;
            cx.text_at(x, r.cy(), &s, 13.0, col.pale);
            let ic = (r.h * 0.5).round();
            x -= ic + 4.0 * d;
            cx.icon(Rect::new(x, r.cy() - ic / 2.0, ic, ic), Icon::Batterie { niveau, charge }, col.texte);
        }
    }

    fn panel_frame(&self, cx: &mut Cx, id: Id, r: Rect, side: Side, landscape: bool) {
        cx.ui.region_add(id, r, crate::ui::Sense::Sink);
        let col = cx.couleurs();
        cx.dl.rrect(r, 0.0, col.surface);
        let t = cx.dp.max(1.0);
        let line = match (side, landscape) {
            (Side::Start, true) => Rect::new(r.right() - t, r.y, t, r.h),
            (Side::End, true) => Rect::new(r.x, r.y, t, r.h),
            (Side::Start, false) => Rect::new(r.x, r.bottom() - t, r.w, t),
            (Side::End, false) => Rect::new(r.x, r.y, r.w, t),
        };
        cx.dl.rrect(line, 0.0, col.ligne);
    }

    /// En-tête d'un panneau : renvoie le rectangle du corps, et vrai si ✕ a été touché.
    fn panel_header(&self, cx: &mut Cx, id_close: Id, r: Rect) -> (Rect, Rect, bool) {
        let (d, col, h) = (cx.dp, cx.couleurs(), cx.commande());
        let head = h + 12.0 * d;
        let close = Rect::new(r.right() - h - 6.0 * d, r.y + 6.0 * d, h, h);
        let clicked = cx.ui.click(id_close, close, None);
        cx.dl.rrect(close, cx.theme.rayon(h), col.puce);
        cx.icon(close.inset(h * 0.25), Icon::Close, col.texte);
        cx.dl.rrect(Rect::new(r.x, r.y + head - d, r.w, d.max(1.0)), 0.0, col.ligne);
        // Les onglets partent de (x, y) ; le clip les garde à gauche du ✕
        (Rect::new(r.x + 12.0 * d, r.y + 6.0 * d, r.w - h - 30.0 * d, h), Rect::new(r.x, r.y + head, r.w, r.h - head), clicked)
    }

    fn start_panel(&mut self, cx: &mut Cx, l: &Layout) {
        if self.shell.start.anim == 0.0 {
            return;
        }
        let r = l.start;
        self.panel_frame(cx, id("panneau-debut"), r, Side::Start, l.landscape);
        let (tabs, body, close) = self.panel_header(cx, id("fermer-debut"), r);
        if close {
            self.shell.start.open = false;
        }
        cx.tabs(id("onglets-debut"), tabs.x, tabs.y, &["Couleurs"], 0);
        let scroll = id("liste-couleurs");
        let mut b = cx.body(scroll, body, &self.scrolls);
        for (i, (name, color)) in COLORS.iter().enumerate() {
            if cx.row(&mut b, sub_id(scroll, i), name, Some(*color), i == self.color) {
                self.color = i;
            }
        }
        cx.body_end(b, &mut self.scrolls);
    }

    fn end_panel(&mut self, cx: &mut Cx, l: &Layout) {
        if self.shell.end.anim == 0.0 {
            return;
        }
        let r = l.end;
        self.panel_frame(cx, id("panneau-fin"), r, Side::End, l.landscape);
        let (tabs, body, close) = self.panel_header(cx, id("fermer-fin"), r);
        if close {
            self.shell.end.open = false;
        }
        match self.end_content {
            EndContent::Display => {
                self.display_tab = cx.tabs(id("onglets-affichage"), tabs.x, tabs.y, &["Vue", "Cube"], self.display_tab);
                let scroll = sub_id(id("corps-affichage"), self.display_tab);
                let mut b = cx.body(scroll, body, &self.scrolls);
                if self.display_tab == 0 {
                    cx.section(&mut b, "CAMÉRA");
                    let shown = format!("{:.0}°", self.fov);
                    cx.slider(&mut b, id("champ"), "Champ de vision", &shown, &mut self.fov, 25.0, 90.0);
                    if cx.button(&mut b, id("recadrer-panneau"), "Recadrer") {
                        self.cams.clear();
                    }
                    cx.section(&mut b, "ROTATION");
                    cx.toggle(&mut b, id("rotation-panneau"), "Rotation automatique", &mut self.spin);
                    let shown = format!("{:.0}°/s", self.speed);
                    cx.slider(&mut b, id("vitesse"), "Vitesse", &shown, &mut self.speed, 5.0, 180.0);
                } else {
                    cx.section(&mut b, "FORME");
                    let shown = format!("{:.2}", self.size);
                    cx.slider(&mut b, id("taille"), "Taille", &shown, &mut self.size, 0.4, 2.0);
                    cx.section(&mut b, "APPARENCE");
                    cx.toggle(&mut b, id("faces"), "Faces colorées", &mut self.faces);
                }
                cx.body_end(b, &mut self.scrolls);
            }
            EndContent::APropos => {
                cx.tabs(id("onglets-a-propos"), tabs.x, tabs.y, &["Rustine"], 0);
                let mut b = cx.body(id("corps-a-propos"), body, &self.scrolls);
                cx.section(&mut b, &format!("RUSTINE {}", env!("CARGO_PKG_VERSION")));
                cx.paragraph(&mut b, "Socle d'applications Android en Rust : interface tactile et 3D en OpenGL ES 3, sans code Java.");
                cx.paragraph(&mut b, "Réutilisable à des fins non commerciales, selon la licence PolyForm Noncommercial 1.0.0.");
                cx.section(&mut b, "BARRE");
                cx.paragraph(&mut b, "Le centre du bas de l'écran est réservé aux gestes du système, mais les appuis y restent libres : la barre de l'application y range ses pastilles.");
                if let Some(f) = &self.fichier_reglages {
                    cx.section(&mut b, "RÉGLAGES");
                    cx.paragraph(&mut b, f);
                }
                cx.body_end(b, &mut self.scrolls);
            }
            EndContent::Help => {
                cx.tabs(id("onglets-aide"), tabs.x, tabs.y, &["Aide"], 0);
                let mut b = cx.body(id("corps-aide"), body, &self.scrolls);
                cx.section(&mut b, "GESTES");
                cx.paragraph(&mut b, "Un doigt sur l'espace de travail fait tourner la vue.");
                cx.paragraph(&mut b, "Deux doigts zooment et déplacent la vue.");
                cx.section(&mut b, "PANNEAUX");
                cx.paragraph(&mut b, "Les pastilles de la barre du bas déploient les panneaux. L'espace de travail s'adapte à la place restante.");
                cx.paragraph(&mut b, "Glissez la frontière d'un panneau pour le redimensionner ; un double appui rétablit sa taille.");
                cx.body_end(b, &mut self.scrolls);
            }
        }
    }

    /// Poignées de redimensionnement, sur la frontière de chaque panneau ouvert.
    fn handles(&mut self, cx: &mut Cx, l: &Layout, screen: Rect) {
        let d = cx.dp;
        for (side, key, shown) in [(Side::Start, id("poignee-debut"), self.shell.start.anim), (Side::End, id("poignee-fin"), self.shell.end.anim)] {
            if shown < 1.0 {
                continue;
            }
            let band = self.shell.handle(l, side);
            if let Some(drag) = cx.ui.drag(key, band) {
                self.shell.resize(side, screen, drag.x, drag.y);
            }
            if cx.ui.double_tapped(key) {
                self.shell.reset_size(side, screen);
            }
            let pressed = cx.ui.pressed(key);
            let color = if pressed { cx.couleurs().accent } else { [0x6C, 0x6C, 0x80, 140] };
            let (long, thick) = (52.0 * d, 5.0 * d);
            let grip = if l.landscape {
                Rect::new(band.cx() - thick / 2.0, band.cy() - long / 2.0, thick, long)
            } else {
                Rect::new(band.cx() - long / 2.0, band.cy() - thick / 2.0, long, thick)
            };
            cx.dl.rrect(grip, thick / 2.0, color);
        }
    }

    fn scene(&self, w: Rect, placees: &[Placee]) -> Scene {
        let model = math::scale(self.size);
        let vues = placees
            .iter()
            .filter(|p| p.vue == VUE_CUBE && p.contenu.w >= 1.0 && p.contenu.h >= 1.0)
            .map(|p| {
                let c = self.cams.get(&p.id).copied().unwrap_or(HOME);
                let (sy, cy, sp, cp) = (c.yaw.sin(), c.yaw.cos(), c.pitch.sin(), c.pitch.cos());
                let eye = [c.target[0] + c.dist * sy * cp, c.target[1] + c.dist * sp, c.target[2] + c.dist * cy * cp];
                let view = math::look_at(eye, c.target, [0.0, 1.0, 0.0]);
                // Le rapport suit la fenêtre : c'est lui qui « adapte » la 3D à la place.
                let proj = math::perspective(self.fov.to_radians(), p.contenu.w / p.contenu.h, 0.1, 100.0);
                Vue3d { rect: p.contenu, mvp: math::mul(&proj, &math::mul(&view, &model)) }
            })
            .collect();
        let [r, g, b, _] = COLORS[self.color].1;
        Scene { workspace: w, vues, fond: Theme::default().couleurs().fond, color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0], tint: if self.faces { 0.25 } else { 0.0 } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawList;
    use crate::text::Text;
    use crate::ui::{Event, Ui};

    const SCREEN: Rect = Rect::new(0., 0., 1920., 1200.);

    struct Rig {
        app: App,
        ui: Ui,
        text: Text,
        dl: DrawList,
    }

    impl Rig {
        fn new() -> Rig {
            let mut r = Rig { app: App::new(1.5), ui: Ui::new(1.5), text: Text::new(), dl: DrawList::default() };
            r.frames(1);
            r
        }
        fn frame(&mut self, dt: f32) -> Scene {
            self.dl.clear();
            let mut cx = Cx { ui: &mut self.ui, text: &mut self.text, dl: &mut self.dl, dp: 1.5, theme: Default::default() };
            let s = self.app.frame(&mut cx, SCREEN, dt);
            self.ui.end_frame();
            s
        }
        fn frames(&mut self, n: usize) -> Scene {
            let mut s = self.frame(0.05);
            for _ in 1..n {
                s = self.frame(0.05);
            }
            s
        }
        fn tap(&mut self, x: f32, y: f32) {
            self.ui.event(Event::Down { id: 0, x, y }, 0.0);
            self.ui.event(Event::Up { id: 0 }, 0.05);
            self.frames(1);
        }
    }

    // Centre de la n-ième pastille des panneaux, dans la barre du bas (après la rustine).
    fn pastille(n: f32) -> (f32, f32) {
        (72.0 * (n + 1.5), 1200.0 - 36.0)
    }

    #[test]
    fn la_taille_des_commandes_suit_le_theme() {
        let mut r = Rig::new();
        r.app.theme.commande = 60.0;
        r.frames(1);
        // Les pastilles font 60 dp : la barre est plus haute, la première pastille des
        // panneaux plus loin
        let (x, y) = (90.0 * 1.5, 1200.0 - 45.0);
        r.tap(x, y);
        assert!(r.app.shell.start.open);
    }

    #[test]
    fn une_pastille_deploie_son_panneau_et_l_espace_s_adapte() {
        let mut r = Rig::new();
        let (x, y) = pastille(1.0);
        r.tap(x, y);
        assert!(r.app.shell.end.open);
        let s = r.frames(60);
        assert_eq!(s.workspace.w, 1920.0 - 576.0);
        assert!(!r.app.animating());
        let (x, y) = pastille(1.0);
        r.tap(x, y); // nouvel appui : referme
        assert!(!r.app.shell.end.open);
        assert_eq!(r.frames(60).workspace, Rect::new(0., 0., 1920., 1200. - 72.));
    }

    #[test]
    fn la_pastille_d_aide_remplace_le_contenu_sans_fermer() {
        let mut r = Rig::new();
        let (x, y) = pastille(1.0);
        r.tap(x, y);
        r.frames(60);
        let (x, y) = pastille(2.0);
        r.tap(x, y);
        assert!(r.app.shell.end.open && r.app.end_content == EndContent::Help);
    }

    #[test]
    fn toucher_la_liste_change_la_couleur_sans_tourner_la_vue() {
        let mut r = Rig::new();
        let (x, y) = pastille(0.0);
        r.tap(x, y);
        r.frames(60);
        let yaw = r.app.cam(1).yaw;
        r.tap(200.0, 1.5 * (60.0 + 8.0 + 48.0 * 3.5));
        assert_eq!(r.app.color, 3);
        assert_eq!(r.app.cam(1).yaw, yaw);
    }

    #[test]
    fn glisser_sur_l_espace_de_travail_tourne_la_vue() {
        let mut r = Rig::new();
        let yaw = r.app.cam(1).yaw;
        r.ui.event(Event::Down { id: 0, x: 900., y: 600. }, 0.0);
        r.ui.event(Event::Move { id: 0, x: 1000., y: 600. }, 0.1);
        r.ui.event(Event::Move { id: 0, x: 1100., y: 600. }, 0.1);
        r.frames(1);
        assert!(r.app.cam(1).yaw < yaw);
    }

    #[test]
    fn la_poignee_redimensionne_dans_la_demo() {
        let mut r = Rig::new();
        r.app.surface.partager(1, crate::surface::Sens::Colonnes, 0.5);
        r.frames(1);
        // La poignée de la fenêtre de gauche : 54 px de côté, dans son coin bas droit, au-dessus de la barre de l'application
        r.ui.event(Event::Down { id: 0, x: 933., y: 1101. }, 0.0);
        r.frames(1);
        for k in 1..=10 {
            r.ui.event(Event::Move { id: 0, x: 933. + 30. * k as f32, y: 1101. }, 0.1);
            r.frames(1);
        }
        r.ui.event(Event::Up { id: 0 }, 0.2);
        let s = r.frames(2);
        assert!((s.vues[0].rect.w - 1260.).abs() <= 1.0, "{:?}", s.vues[0].rect);
    }

    #[test]
    fn chaque_fenetre_3d_a_sa_camera() {
        let mut r = Rig::new();
        let nouvelle = r.app.surface.partager(1, crate::surface::Sens::Colonnes, 0.5).unwrap();
        let s = r.frames(1);
        assert_eq!(s.vues.len(), 2);
        assert_eq!(s.vues[0].rect.w, 960.0);
        // Un geste dans la fenêtre de droite ne tourne qu'elle
        let (a, b) = (r.app.cam(nouvelle).yaw, r.app.cam(1).yaw);
        r.ui.event(Event::Down { id: 0, x: 1400., y: 600. }, 0.0);
        r.ui.event(Event::Move { id: 0, x: 1500., y: 600. }, 0.1);
        r.ui.event(Event::Move { id: 0, x: 1600., y: 600. }, 0.1);
        r.frames(1);
        assert_eq!(r.app.cam(nouvelle).yaw, a);
        assert!(r.app.cam(1).yaw < b);
        // Une fenêtre passée en liste n'a plus de 3D
        r.app.surface.changer_vue(nouvelle, VUE_COULEURS);
        assert_eq!(r.frames(1).vues.len(), 1);
    }

    #[test]
    fn rien_ne_redessine_au_repos() {
        let mut r = Rig::new();
        r.frames(3);
        assert!(!r.app.animating());
    }
}
