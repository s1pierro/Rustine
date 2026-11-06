//! Surface de travail, à la manière de forg.x : l'espace de travail se partage en
//! fenêtres, chacune avec sa vue.
//!
//! - La surface est un arbre : une fenêtre, ou un partage en deux (côte à côte ou l'une
//!   sur l'autre) selon une proportion. Chaque fenêtre a un identifiant stable, et une
//!   vue que l'application interprète (3D, liste, texte…).
//! - Chaque fenêtre a une barre en haut (son titre, son menu) et une barre d'état en bas
//!   (une ligne pour l'application, une poignée dans le coin).
//! - **Partager** : glisser le doigt sur la barre du haut. Au-delà d'un seuil, le partage
//!   s'engage, côte à côte si le geste est plutôt horizontal, l'une sur l'autre sinon.
//!   Un aperçu suit le doigt, et la fenêtre se coupe au lâcher. La nouvelle fenêtre
//!   prend la part de l'aperçu (de bord jusqu'au doigt), avec la même vue ; l'ancienne
//!   garde le reste.
//! - **Redimensionner** : tirer la poignée du coin. Elle déplace le bord droit et le bord
//!   bas de la fenêtre (les partages les plus proches qui les portent), de 10 à 90 %.
//! - **Fermer** : par le menu. La fenêtre voisine reprend la place.
//! - **Menu** : le choix de la vue de la fenêtre, et la fermeture.
//!
//! L'application appelle `frame` (disposition, barres, gestes) avant de dessiner le
//! contenu des fenêtres, puis `dessus` (aperçu du partage, menu), qui passe au-dessus de
//! tout. Les changements (fenêtre partagée, fermée, vue changée) se lisent avec
//! `changements`, et la disposition s'écrit et se relit en texte (`ecrire`, `lire`).

use crate::draw::Rect;
use crate::ui::{id, sub_id, Id, Sense};
use crate::widgets::{Cx, Icon};

/// L'identifiant stable d'une fenêtre.
pub type FenetreId = u64;

/// Hauteur des barres et des lignes du menu, en part de la taille des commandes du
/// thème ; seuil du geste de partage (dp), côté de la poignée (dp).
const BARRE: f32 = 0.75;
const ETAT: f32 = 0.5;
const MENU_LIGNE: f32 = 0.875;
const SEUIL: f32 = 64.0;
const POIGNEE: f32 = 28.0;
const MENU_LARGE: f32 = 220.0;
/// Bornes d'une proportion.
const MIN: f32 = 0.1;
const MAX: f32 = 0.9;

const BARRE_ID: Id = id("surface/barre");
const MENU_ID: Id = id("surface/menu");
const POIGNEE_ID: Id = id("surface/poignee");
const ETAT_ID: Id = id("surface/etat");
const FOND_MENU: Id = id("surface/fond-menu");
const LIGNE_MENU: Id = id("surface/ligne-menu");

/// Le sens d'un partage : `Colonnes`, côte à côte (la première à gauche) ; `Lignes`,
/// l'une sur l'autre (la première en haut).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sens {
    Colonnes,
    Lignes,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Noeud {
    Fenetre { id: FenetreId, vue: u32 },
    Partage { sens: Sens, ratio: f32, a: Box<Noeud>, b: Box<Noeud> },
}

/// Une fenêtre à l'écran : tout son cadre, son contenu (entre les barres) et la ligne
/// d'état laissée à l'application.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placee {
    pub id: FenetreId,
    pub vue: u32,
    pub cadre: Rect,
    pub contenu: Rect,
    pub etat: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Changement {
    /// `de` s'est partagée ; `nouvelle` est née, avec la même vue.
    Partagee { de: FenetreId, nouvelle: FenetreId },
    Fermee(FenetreId),
    Vue { id: FenetreId, vue: u32 },
}

/// Un geste de partage en cours, sur la barre d'une fenêtre.
#[derive(Clone, Copy, Debug)]
struct Geste {
    fenetre: FenetreId,
    depart: (f32, f32),
    doigt: (f32, f32),
    sens: Option<Sens>,
    vu: bool,
}

pub struct Surface {
    pub racine: Noeud,
    suivant: FenetreId,
    geste: Option<Geste>,
    /// La poignée tirée : sa fenêtre, et l'écart du doigt aux bords droit et bas.
    tirage: Option<(FenetreId, f32, f32)>,
    menu: Option<FenetreId>,
    rect: Rect,
    placees: Vec<Placee>,
    changements: Vec<Changement>,
    /// La disposition a changé depuis la dernière fois qu'on l'a lu : à enregistrer.
    pub modifiee: bool,
}

/// Les deux parts d'un rectangle partagé.
fn couper(r: Rect, sens: Sens, ratio: f32) -> (Rect, Rect) {
    match sens {
        Sens::Colonnes => {
            let w = (r.w * ratio).round();
            (Rect::new(r.x, r.y, w, r.h), Rect::new(r.x + w, r.y, r.w - w, r.h))
        }
        Sens::Lignes => {
            let h = (r.h * ratio).round();
            (Rect::new(r.x, r.y, r.w, h), Rect::new(r.x, r.y + h, r.w, r.h - h))
        }
    }
}

impl Noeud {
    fn disposer(&self, r: Rect, out: &mut Vec<(FenetreId, u32, Rect)>) {
        match self {
            Noeud::Fenetre { id, vue } => out.push((*id, *vue, r)),
            Noeud::Partage { sens, ratio, a, b } => {
                let (ra, rb) = couper(r, *sens, *ratio);
                a.disposer(ra, out);
                b.disposer(rb, out);
            }
        }
    }

    fn max_id(&self) -> FenetreId {
        match self {
            Noeud::Fenetre { id, .. } => *id,
            Noeud::Partage { a, b, .. } => a.max_id().max(b.max_id()),
        }
    }

    fn compte(&self) -> usize {
        match self {
            Noeud::Fenetre { .. } => 1,
            Noeud::Partage { a, b, .. } => a.compte() + b.compte(),
        }
    }

    /// Déplace les bords droit et bas de la fenêtre `f` vers (x, y) : pour chaque axe,
    /// le partage le plus profond où `f` est dans la première part. `fait` : les axes
    /// déjà servis (colonnes, lignes). Renvoie vrai si `f` est dans ce nœud.
    fn tirer(&mut self, r: Rect, f: FenetreId, x: f32, y: f32, mini: f32, fait: &mut (bool, bool)) -> bool {
        let Noeud::Partage { sens, ratio, a, b } = self else {
            return matches!(self, Noeud::Fenetre { id, .. } if *id == f);
        };
        let (ra, rb) = couper(r, *sens, *ratio);
        if a.tirer(ra, f, x, y, mini, fait) {
            let (pos, debut, long, deja) = match sens {
                Sens::Colonnes => (x, r.x, r.w, &mut fait.0),
                Sens::Lignes => (y, r.y, r.h, &mut fait.1),
            };
            if !*deja && long > 0.0 {
                *deja = true;
                // Chaque part garde au moins `mini` pixels, si la place le permet
                let m = (mini / long).min(0.5);
                *ratio = ((pos - debut) / long).clamp(MIN.max(m), MAX.min(1.0 - m));
            }
            true
        } else {
            b.tirer(rb, f, x, y, mini, fait)
        }
    }

    /// Les axes sur lesquels la poignée de `f` a prise : (colonnes, lignes).
    fn prises(&self, f: FenetreId) -> Option<(bool, bool)> {
        match self {
            Noeud::Fenetre { id, .. } => (*id == f).then_some((false, false)),
            Noeud::Partage { sens, a, b, .. } => {
                if let Some((c, l)) = a.prises(f) {
                    Some(match sens {
                        Sens::Colonnes => (true, l),
                        Sens::Lignes => (c, true),
                    })
                } else {
                    b.prises(f)
                }
            }
        }
    }

    fn ecrire(&self, s: &mut String) {
        match self {
            Noeud::Fenetre { id, vue } => s.push_str(&format!("F{id}:{vue}")),
            Noeud::Partage { sens, ratio, a, b } => {
                s.push(if *sens == Sens::Colonnes { 'C' } else { 'L' });
                s.push_str(&format!("{ratio:.4}("));
                a.ecrire(s);
                s.push(',');
                b.ecrire(s);
                s.push(')');
            }
        }
    }

    fn lire(s: &mut &str) -> Option<Noeud> {
        let c = s.chars().next()?;
        *s = &s[c.len_utf8()..];
        let nombre = |s: &mut &str| -> Option<f64> {
            let n = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(s.len());
            let v = s[..n].parse().ok()?;
            *s = &s[n..];
            Some(v)
        };
        match c {
            'F' => {
                let id = nombre(s)? as FenetreId;
                *s = s.strip_prefix(':')?;
                let vue = nombre(s)? as u32;
                Some(Noeud::Fenetre { id, vue })
            }
            'C' | 'L' => {
                let ratio = (nombre(s)? as f32).clamp(MIN, MAX);
                *s = s.strip_prefix('(')?;
                let a = Noeud::lire(s)?;
                *s = s.strip_prefix(',')?;
                let b = Noeud::lire(s)?;
                *s = s.strip_prefix(')')?;
                Some(Noeud::Partage { sens: if c == 'C' { Sens::Colonnes } else { Sens::Lignes }, ratio, a: Box::new(a), b: Box::new(b) })
            }
            _ => None,
        }
    }
}

impl Surface {
    /// Une seule fenêtre, avec cette vue.
    pub fn new(vue: u32) -> Surface {
        Surface::de(Noeud::Fenetre { id: 1, vue })
    }

    fn de(racine: Noeud) -> Surface {
        let suivant = racine.max_id() + 1;
        Surface { racine, suivant, geste: None, tirage: None, menu: None, rect: Rect::default(), placees: Vec::new(), changements: Vec::new(), modifiee: false }
    }

    /// Les fenêtres dans le rectangle `r` : identifiant, vue et cadre.
    pub fn disposer(&self, r: Rect) -> Vec<(FenetreId, u32, Rect)> {
        let mut out = Vec::new();
        self.racine.disposer(r, &mut out);
        out
    }

    pub fn nombre(&self) -> usize {
        self.racine.compte()
    }

    /// Les fenêtres placées à la dernière image.
    pub fn placees(&self) -> &[Placee] {
        &self.placees
    }

    /// La fenêtre dont le contenu contient (x, y), à la dernière image.
    pub fn fenetre_en(&self, x: f32, y: f32) -> Option<&Placee> {
        self.placees.iter().find(|p| p.contenu.contains(x, y))
    }

    pub fn vue(&self, f: FenetreId) -> Option<u32> {
        fn chercher(n: &Noeud, f: FenetreId) -> Option<u32> {
            match n {
                Noeud::Fenetre { id, vue } => (*id == f).then_some(*vue),
                Noeud::Partage { a, b, .. } => chercher(a, f).or_else(|| chercher(b, f)),
            }
        }
        chercher(&self.racine, f)
    }

    /// Change la vue d'une fenêtre.
    pub fn changer_vue(&mut self, f: FenetreId, nouvelle: u32) {
        fn changer(n: &mut Noeud, f: FenetreId, nouvelle: u32) -> bool {
            match n {
                Noeud::Fenetre { id, vue } if *id == f => {
                    *vue = nouvelle;
                    true
                }
                Noeud::Fenetre { .. } => false,
                Noeud::Partage { a, b, .. } => changer(a, f, nouvelle) || changer(b, f, nouvelle),
            }
        }
        if changer(&mut self.racine, f, nouvelle) {
            self.changements.push(Changement::Vue { id: f, vue: nouvelle });
            self.modifiee = true;
        }
    }

    /// Partage la fenêtre `f` : la nouvelle prend la première part (`ratio`), avec la même
    /// vue. Renvoie son identifiant.
    pub fn partager(&mut self, f: FenetreId, sens: Sens, ratio: f32) -> Option<FenetreId> {
        let nouvelle = self.suivant;
        fn remplacer(n: &mut Noeud, f: FenetreId, sens: Sens, ratio: f32, nouvelle: FenetreId) -> bool {
            match n {
                Noeud::Fenetre { id, vue } if *id == f => {
                    let vue = *vue;
                    *n = Noeud::Partage { sens, ratio: ratio.clamp(MIN, MAX), a: Box::new(Noeud::Fenetre { id: nouvelle, vue }), b: Box::new(Noeud::Fenetre { id: f, vue }) };
                    true
                }
                Noeud::Fenetre { .. } => false,
                Noeud::Partage { a, b, .. } => remplacer(a, f, sens, ratio, nouvelle) || remplacer(b, f, sens, ratio, nouvelle),
            }
        }
        if !remplacer(&mut self.racine, f, sens, ratio, nouvelle) {
            return None;
        }
        self.suivant += 1;
        self.changements.push(Changement::Partagee { de: f, nouvelle });
        self.modifiee = true;
        Some(nouvelle)
    }

    /// Ferme la fenêtre `f` : sa voisine reprend la place. La dernière ne se ferme pas.
    pub fn fermer(&mut self, f: FenetreId) -> bool {
        fn oter(n: &mut Noeud, f: FenetreId) -> bool {
            let Noeud::Partage { a, b, .. } = n else { return false };
            let reste = if matches!(**a, Noeud::Fenetre { id, .. } if id == f) {
                Some(std::mem::replace(&mut **b, Noeud::Fenetre { id: 0, vue: 0 }))
            } else if matches!(**b, Noeud::Fenetre { id, .. } if id == f) {
                Some(std::mem::replace(&mut **a, Noeud::Fenetre { id: 0, vue: 0 }))
            } else {
                None
            };
            match reste {
                Some(r) => {
                    *n = r;
                    true
                }
                None => oter(a, f) || oter(b, f),
            }
        }
        if !oter(&mut self.racine, f) {
            return false;
        }
        if self.menu == Some(f) {
            self.menu = None;
        }
        self.changements.push(Changement::Fermee(f));
        self.modifiee = true;
        true
    }

    /// Amène les bords droit et bas de la fenêtre `f` en (x, y), dans la surface `r`.
    pub fn tirer(&mut self, r: Rect, f: FenetreId, x: f32, y: f32, mini: f32) {
        self.racine.tirer(r, f, x, y, mini, &mut (false, false));
        self.modifiee = true;
    }

    /// Les changements depuis le dernier appel.
    pub fn changements(&mut self) -> Vec<Changement> {
        std::mem::take(&mut self.changements)
    }

    /// La disposition en texte, par exemple `C0.3000(F2:0,L0.5000(F3:1,F1:0))`.
    pub fn ecrire(&self) -> String {
        let mut s = String::new();
        self.racine.ecrire(&mut s);
        s
    }

    /// Relit `ecrire` ; rien si le texte ne se tient pas.
    pub fn lire(texte: &str) -> Option<Surface> {
        let mut s = texte.trim();
        let racine = Noeud::lire(&mut s)?;
        s.is_empty().then(|| Surface::de(racine))
    }

    /// Dispose les fenêtres dans `r`, dessine leurs barres et suit les gestes. `noms` :
    /// le nom de chaque vue, pour les titres et le menu. À appeler avant le contenu.
    pub fn frame(&mut self, cx: &mut Cx, r: Rect, noms: &[&str]) -> Vec<Placee> {
        let (d, col) = (cx.dp, cx.couleurs());
        let h = cx.commande();
        let (barre, etat, poignee) = ((BARRE * h).round(), (ETAT * h).round(), POIGNEE * d);
        self.rect = r;
        self.placees = self
            .disposer(r)
            .into_iter()
            .map(|(id, vue, c)| {
                let b = barre.min(c.h / 2.0);
                let e = etat.min((c.h - b) / 2.0);
                Placee { id, vue, cadre: c, contenu: Rect::new(c.x, c.y + b, c.w, (c.h - b - e).max(0.0)), etat: Rect::new(c.x, c.bottom() - e, c.w, e) }
            })
            .collect();
        if let Some(g) = &mut self.geste {
            g.vu = false;
        }
        let mut tire = false;
        for p in self.placees.clone() {
            let c = p.cadre;
            let k = p.id as usize;
            // La barre du haut : le titre, le menu, et le geste de partage
            let bh = p.contenu.y - c.y;
            let barre_r = Rect::new(c.x, c.y, c.w, bh);
            cx.dl.rrect(barre_r, 0.0, col.surface);
            cx.dl.rrect(Rect::new(c.x, barre_r.bottom() - d.max(1.0), c.w, d.max(1.0)), 0.0, col.ligne);
            if let Some(g) = cx.ui.drag(sub_id(BARRE_ID, k), barre_r) {
                let geste = self.geste.get_or_insert(Geste { fenetre: p.id, depart: (g.x, g.y), doigt: (g.x, g.y), sens: None, vu: true });
                if geste.fenetre == p.id {
                    geste.doigt = (g.x, g.y);
                    geste.vu = true;
                    let (dx, dy) = (g.x - geste.depart.0, g.y - geste.depart.1);
                    if geste.sens.is_none() && dx.hypot(dy) > SEUIL * d {
                        geste.sens = Some(if dx.abs() > dy.abs() { Sens::Colonnes } else { Sens::Lignes });
                    }
                }
            }
            let menu_r = Rect::new(c.right() - bh, c.y, bh, bh);
            if cx.ui.click(sub_id(MENU_ID, k), menu_r, None) {
                self.menu = if self.menu == Some(p.id) { None } else { Some(p.id) };
            }
            let ouvert = self.menu == Some(p.id);
            if ouvert || cx.ui.pressed(sub_id(MENU_ID, k)) {
                cx.dl.rrect(menu_r.inset(4.0 * d), 6.0 * d, col.puce);
            }
            cx.icon(menu_r.inset(bh * 0.25), Icon::Menu, if ouvert { col.accent } else { col.pale });
            let titre = noms.get(p.vue as usize).copied().unwrap_or("?");
            cx.clip(Some(Rect::new(c.x, c.y, (c.w - bh).max(0.0), bh)));
            cx.text_at(c.x + 10.0 * d, barre_r.cy(), titre, 14.0, col.texte);
            cx.clip(None);
            // La barre d'état, et sa poignée
            let e = p.etat;
            cx.ui.region_add(sub_id(ETAT_ID, k), e, Sense::Sink);
            cx.dl.rrect(e, 0.0, col.fond);
            cx.dl.rrect(Rect::new(c.x, e.y, c.w, d.max(1.0)), 0.0, col.ligne);
            if let Some((px, pl)) = self.racine.prises(p.id).filter(|(x, y)| *x || *y) {
                let pr = Rect::new(c.right() - poignee, c.bottom() - poignee.min(e.h.max(1.0) + 4.0 * d), poignee, poignee.min(e.h.max(1.0) + 4.0 * d));
                let pk = sub_id(POIGNEE_ID, k);
                if let Some(g) = cx.ui.drag(pk, pr) {
                    let (_, ox, oy) = *self.tirage.get_or_insert((p.id, c.right() - g.x, c.bottom() - g.y));
                    if g.moved && self.tirage.is_some_and(|t| t.0 == p.id) {
                        tire = true;
                        let (x, y) = (if px { g.x + ox } else { 0.0 }, if pl { g.y + oy } else { 0.0 });
                        self.racine.tirer(r, p.id, x, y, barre + etat + 16.0 * d, &mut (!px, !pl));
                        self.modifiee = true;
                    }
                }
                let couleur = if cx.ui.pressed(pk) { col.accent } else { col.pale };
                let (bx, by) = (c.right() - 6.0 * d, c.bottom() - 5.0 * d);
                for k in [7.0, 12.0] {
                    cx.dl.line([bx - k * d, by], [bx, by - k * d], 1.6 * d, couleur);
                }
            }
            // Les séparations, à droite et en bas, sauf au bord de la surface
            if c.right() < r.right() - 0.5 {
                cx.dl.rrect(Rect::new(c.right() - d.max(1.0), c.y, d.max(1.0), c.h), 0.0, col.ligne);
            }
        }
        if !tire && !self.placees.iter().any(|p| cx.ui.pressed(sub_id(POIGNEE_ID, p.id as usize))) {
            self.tirage = None;
        }
        // Le doigt a quitté la barre : le partage se fait, s'il était engagé
        if let Some(g) = self.geste.filter(|g| !g.vu) {
            self.geste = None;
            if let (Some(sens), Some(p)) = (g.sens, self.placees.iter().find(|p| p.id == g.fenetre)) {
                let c = p.cadre;
                let ratio = match sens {
                    Sens::Colonnes => (g.doigt.0 - c.x) / c.w.max(1.0),
                    Sens::Lignes => (g.doigt.1 - c.y) / c.h.max(1.0),
                };
                self.partager(g.fenetre, sens, ratio);
            }
        }
        self.placees.clone()
    }

    /// L'aperçu du partage et le menu ouvert, par-dessus le contenu des fenêtres.
    pub fn dessus(&mut self, cx: &mut Cx, noms: &[&str]) {
        let (d, col) = (cx.dp, cx.couleurs());
        if let Some(g) = self.geste {
            if let (Some(sens), Some(p)) = (g.sens, self.placees.iter().find(|p| p.id == g.fenetre)) {
                let c = p.cadre;
                let (part, bord) = match sens {
                    Sens::Colonnes => {
                        let x = g.doigt.0.clamp(c.x + c.w * MIN, c.x + c.w * MAX);
                        (Rect::new(c.x, c.y, x - c.x, c.h), Rect::new(x - d, c.y, 2.0 * d, c.h))
                    }
                    Sens::Lignes => {
                        let y = g.doigt.1.clamp(c.y + c.h * MIN, c.y + c.h * MAX);
                        (Rect::new(c.x, c.y, c.w, y - c.y), Rect::new(c.x, y - d, c.w, 2.0 * d))
                    }
                };
                cx.dl.rrect(part, 0.0, [col.accent[0], col.accent[1], col.accent[2], 0x40]);
                cx.dl.rrect(bord, 0.0, col.accent);
            }
        }
        let Some(p) = self.menu.and_then(|f| self.placees.iter().find(|p| p.id == f).copied()) else {
            self.menu = None;
            return;
        };
        // Toucher hors du menu le referme
        if cx.ui.click(FOND_MENU, self.rect, None) {
            self.menu = None;
            return;
        }
        let fermer = self.nombre() > 1;
        let lignes = noms.len() + fermer as usize;
        let (w, lh) = ((MENU_LARGE * d * cx.theme.texte).min(self.rect.w), (MENU_LIGNE * cx.commande()).round());
        let h = lh * lignes as f32 + 8.0 * d;
        let x = (p.cadre.right() - w - 4.0 * d).max(self.rect.x);
        let y = (p.contenu.y + 2.0 * d).min(self.rect.bottom() - h).max(self.rect.y);
        let m = Rect::new(x, y, w, h);
        cx.ui.region_add(id("surface/menu-fond"), m, Sense::Sink);
        cx.dl.shadow(Rect::new(m.x, m.y + 2.0 * d, m.w, m.h), 10.0 * d, 10.0 * d, col.ombre);
        cx.dl.rrect(m, 10.0 * d, col.surface);
        for i in 0..lignes {
            let l = Rect::new(m.x + 4.0 * d, m.y + 4.0 * d + i as f32 * lh, m.w - 8.0 * d, lh);
            let k = sub_id(LIGNE_MENU, i);
            let clic = cx.ui.click(k, l, None);
            if cx.ui.pressed(k) {
                cx.dl.rrect(l, 8.0 * d, col.puce);
            }
            if i < noms.len() {
                let actuelle = i as u32 == p.vue;
                cx.text_at(l.x + 12.0 * d, l.cy(), noms[i], 15.0, if actuelle { col.accent } else { col.texte });
                if clic {
                    self.menu = None;
                    if !actuelle {
                        self.changer_vue(p.id, i as u32);
                    }
                }
            } else {
                cx.dl.rrect(Rect::new(l.x + 8.0 * d, l.y, l.w - 16.0 * d, d.max(1.0)), 0.0, col.ligne);
                cx.text_at(l.x + 12.0 * d, l.cy(), "Fermer la fenêtre", 15.0, col.pale);
                if clic {
                    self.fermer(p.id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawList;
    use crate::text::Text;
    use crate::ui::{Event, Ui};

    const R: Rect = Rect::new(0., 0., 1000., 800.);
    const NOMS: [&str; 2] = ["3D", "Liste"];

    struct Banc {
        s: Surface,
        ui: Ui,
        text: Text,
        dl: DrawList,
        t: f64,
    }

    impl Banc {
        fn new() -> Banc {
            let mut b = Banc { s: Surface::new(0), ui: Ui::new(1.0), text: Text::new(), dl: DrawList::default(), t: 0.0 };
            b.image();
            b
        }
        fn image(&mut self) -> Vec<Placee> {
            self.dl.clear();
            let mut cx = Cx { ui: &mut self.ui, text: &mut self.text, dl: &mut self.dl, dp: 1.0, theme: Default::default() };
            let p = self.s.frame(&mut cx, R, &NOMS);
            self.s.dessus(&mut cx, &NOMS);
            self.ui.end_frame();
            p
        }
        fn evt(&mut self, e: Event) {
            self.t += 0.05;
            self.ui.event(e, self.t);
        }
        /// Un doigt glisse de a à b, une image entre chaque pas, puis se lève.
        fn glisser(&mut self, a: (f32, f32), b: (f32, f32)) {
            self.evt(Event::Down { id: 0, x: a.0, y: a.1 });
            self.image();
            for k in 1..=10 {
                let t = k as f32 / 10.0;
                self.evt(Event::Move { id: 0, x: a.0 + (b.0 - a.0) * t, y: a.1 + (b.1 - a.1) * t });
                self.image();
            }
            self.evt(Event::Up { id: 0 });
            self.image();
            self.image();
        }
        fn toucher(&mut self, x: f32, y: f32) {
            self.evt(Event::Down { id: 0, x, y });
            self.evt(Event::Up { id: 0 });
            self.image();
            self.image();
        }
    }

    #[test]
    fn les_fenetres_pavent_la_surface() {
        let mut s = Surface::new(0);
        let n = s.partager(1, Sens::Colonnes, 0.3).unwrap();
        s.partager(1, Sens::Lignes, 0.5).unwrap();
        let f = s.disposer(R);
        assert_eq!(f.len(), 3);
        assert_eq!(f.iter().map(|(_, _, r)| r.w * r.h).sum::<f32>(), R.w * R.h);
        // La nouvelle prend la première part, avec la même vue
        assert_eq!(f[0], (n, 0, Rect::new(0., 0., 300., 800.)));
        assert_eq!(s.changements(), vec![Changement::Partagee { de: 1, nouvelle: 2 }, Changement::Partagee { de: 1, nouvelle: 3 }]);
    }

    #[test]
    fn fermer_rend_la_place_a_la_voisine() {
        let mut s = Surface::new(0);
        s.partager(1, Sens::Colonnes, 0.3);
        s.partager(1, Sens::Lignes, 0.5);
        assert!(s.fermer(3));
        let f = s.disposer(R);
        assert_eq!(f.iter().map(|x| x.0).collect::<Vec<_>>(), vec![2, 1]);
        assert_eq!(f[1].2, Rect::new(300., 0., 700., 800.));
        assert!(s.fermer(2));
        assert!(!s.fermer(1), "la dernière reste");
        assert_eq!(s.racine, Noeud::Fenetre { id: 1, vue: 0 });
        // Un identifiant ne resert jamais
        assert_eq!(s.partager(1, Sens::Lignes, 0.5), Some(4));
    }

    #[test]
    fn la_disposition_se_relit() {
        let mut s = Surface::new(0);
        s.partager(1, Sens::Colonnes, 0.3);
        s.partager(1, Sens::Lignes, 0.5);
        s.changer_vue(3, 1);
        let t = s.ecrire();
        assert_eq!(t, "C0.3000(F2:0,L0.5000(F3:1,F1:0))");
        let r = Surface::lire(&t).unwrap();
        assert_eq!(r.racine, s.racine);
        assert!(Surface::lire("C0.3(F1:0").is_none());
        assert!(Surface::lire("n'importe quoi").is_none());
    }

    #[test]
    fn la_poignee_deplace_les_bords_droit_et_bas() {
        let mut s = Surface::new(0);
        s.partager(1, Sens::Colonnes, 0.5); // 2 | 1
        s.partager(2, Sens::Lignes, 0.5); // 3 sur 2, à gauche
        assert_eq!(s.racine.prises(3), Some((true, true)));
        assert_eq!(s.racine.prises(2), Some((true, false)));
        assert_eq!(s.racine.prises(1), Some((false, false)));
        s.tirer(R, 3, 700.0, 200.0, 50.0);
        let f = s.disposer(R);
        assert_eq!(f[0], (3, 0, Rect::new(0., 0., 700., 200.)));
        // Bornée à 90 %
        s.tirer(R, 3, 2000.0, 790.0, 50.0);
        let f = s.disposer(R);
        assert_eq!(f[0].2, Rect::new(0., 0., 900., 720.));
    }

    #[test]
    fn glisser_sur_la_barre_partage_la_fenetre() {
        let mut b = Banc::new();
        // Vers la droite sur la barre : côte à côte, la nouvelle jusqu'au doigt
        b.glisser((100., 15.), (400., 30.));
        let p = b.image();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].cadre, Rect::new(0., 0., 400., 800.));
        assert_eq!(p[0].id, 2);
        // Vers le bas sur la barre de droite : l'une sur l'autre
        b.glisser((700., 15.), (700., 300.));
        let p = b.image();
        assert_eq!(p.len(), 3);
        assert_eq!(p[1].cadre, Rect::new(400., 0., 600., 300.));
        // Un geste trop court ne partage pas
        b.glisser((100., 15.), (130., 15.));
        assert_eq!(b.image().len(), 3);
    }

    #[test]
    fn le_menu_change_la_vue_et_ferme() {
        let mut b = Banc::new();
        b.s.partager(1, Sens::Colonnes, 0.5);
        b.image();
        b.changements();
        // Le menu de la fenêtre de droite, puis « Liste »
        b.toucher(1000. - 18., 18.);
        assert_eq!(b.s.menu, Some(1));
        let y = 36. + 2. + 4. + 42. * 1.5;
        b.toucher(900., y);
        assert_eq!(b.s.vue(1), Some(1));
        assert_eq!(b.s.menu, None);
        // Puis « Fermer la fenêtre »
        b.toucher(1000. - 18., 18.);
        b.toucher(900., 36. + 2. + 4. + 42. * 2.5);
        assert_eq!(b.s.nombre(), 1);
        assert_eq!(b.s.changements(), vec![Changement::Vue { id: 1, vue: 1 }, Changement::Fermee(1)]);
        // Toucher à côté referme le menu sans rien changer
        b.toucher(1000. - 18., 18.);
        assert!(b.s.menu.is_some());
        b.toucher(100., 600.);
        assert_eq!(b.s.menu, None);
        assert_eq!(b.s.nombre(), 1);
    }

    #[test]
    fn la_poignee_se_tire_au_doigt() {
        let mut b = Banc::new();
        b.s.partager(1, Sens::Colonnes, 0.5);
        b.image();
        // La poignée de la fenêtre de gauche, dans son coin bas droit
        b.glisser((486., 788.), (586., 700.));
        let p = b.image();
        assert!((p[0].cadre.w - 600.).abs() <= 1.0, "{:?}", p[0].cadre);
    }

    impl Banc {
        fn changements(&mut self) -> Vec<Changement> {
            self.s.changements()
        }
    }
}
