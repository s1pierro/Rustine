//! Composants tactiles : pastilles, pilules, onglets, interrupteurs, curseurs,
//! listes défilantes. Mode immédiat : chaque image redécrit l'interface.

use crate::draw::{Color, DrawList, Rect};
use crate::text::Text;
use crate::ui::{sub_id, Id, Sense, Ui};
use std::collections::HashMap;

/// Les couleurs de l'interface, tirées du thème.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Couleurs {
    pub fond: Color,
    pub surface: Color,
    pub texte: Color,
    pub pale: Color,
    pub accent: Color,
    /// Le texte et les icônes posés sur l'accent.
    pub sur_accent: Color,
    pub puce: Color,
    pub ligne: Color,
    pub ombre: Color,
}

/// Les accents proposés : nom, teinte sur fond clair, teinte sur fond sombre.
pub const ACCENTS: [(&str, Color, Color); 6] = [
    ("Indigo", [0x2A, 0x22, 0xD8, 255], [0x8B, 0x85, 0xFF, 255]),
    ("Bleu", [0x1D, 0x6F, 0xE0, 255], [0x5A, 0xA2, 0xFF, 255]),
    ("Turquoise", [0x0F, 0x76, 0x6E, 255], [0x2D, 0xD4, 0xBF, 255]),
    ("Vert", [0x15, 0x80, 0x3D, 255], [0x4A, 0xDE, 0x80, 255]),
    ("Orange", [0xC2, 0x41, 0x0C, 255], [0xFB, 0x92, 0x3C, 255]),
    ("Rose", [0xBE, 0x18, 0x5D, 255], [0xF4, 0x72, 0xB6, 255]),
];

/// L'apparence, réglable : la taille des commandes (pastilles, pilules, boutons,
/// onglets) et des lignes de liste en dp, l'échelle du texte, l'arrondi (0 : coins
/// presque droits, 1 : bouts ronds), l'accent et le thème sombre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub commande: f32,
    pub ligne: f32,
    pub texte: f32,
    pub arrondi: f32,
    pub accent: usize,
    pub sombre: bool,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme { commande: 48.0, ligne: 48.0, texte: 1.0, arrondi: 1.0, accent: 0, sombre: false }
    }
}

impl Theme {
    pub fn couleurs(&self) -> Couleurs {
        let (_, clair, fonce) = ACCENTS[self.accent.min(ACCENTS.len() - 1)];
        if self.sombre {
            Couleurs {
                fond: [0x14, 0x14, 0x1C, 255],
                surface: [0x1E, 0x1E, 0x2A, 255],
                texte: [0xEC, 0xEC, 0xF4, 255],
                pale: [0x9A, 0x9A, 0xB0, 255],
                accent: fonce,
                sur_accent: [0x14, 0x14, 0x1C, 255],
                puce: [0x2C, 0x2C, 0x3A, 255],
                ligne: [0x34, 0x34, 0x46, 255],
                ombre: [0, 0, 0, 110],
            }
        } else {
            Couleurs {
                fond: [0xF3, 0xF2, 0xFA, 255],
                surface: [255, 255, 255, 255],
                texte: [0x1B, 0x1B, 0x24, 255],
                pale: [0x6C, 0x6C, 0x80, 255],
                accent: clair,
                sur_accent: [255, 255, 255, 255],
                puce: [0xEF, 0xEE, 0xF6, 255],
                ligne: [0xE3, 0xE2, 0xEC, 255],
                ombre: [0x1B, 0x1B, 0x40, 60],
            }
        }
    }

    /// Le rayon des coins d'une commande de hauteur `h`.
    pub fn rayon(&self, h: f32) -> f32 {
        h / 2.0 * (0.12 + 0.88 * self.arrondi.clamp(0.0, 1.0))
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    List,
    Sliders,
    Help,
    Close,
    /// Trois points : le menu d'une fenêtre.
    Menu,
}

/// Défilement de chaque zone : position et hauteur du contenu.
pub type Scrolls = HashMap<Id, (f32, f32)>;

pub struct Cx<'a> {
    pub ui: &'a mut Ui,
    pub text: &'a mut Text,
    pub dl: &'a mut DrawList,
    pub dp: f32,
    pub theme: Theme,
}

/// Colonne de composants dans une zone défilante.
pub struct Body {
    pub x: f32,
    pub w: f32,
    pub y: f32,
    top: f32,
    id: Id,
    rect: Rect,
}

impl Cx<'_> {
    pub fn couleurs(&self) -> Couleurs {
        self.theme.couleurs()
    }

    /// La hauteur des commandes, en pixels.
    pub fn commande(&self) -> f32 {
        (self.theme.commande * self.dp).round()
    }

    /// Une taille de texte (dp) en pixels, à l'échelle du thème.
    pub fn px(&self, size: f32) -> f32 {
        (size * self.theme.texte * self.dp).round()
    }

    pub fn measure(&mut self, s: &str, size: f32) -> f32 {
        let px = self.px(size);
        self.text.measure(s, px)
    }

    fn label(&mut self, x: f32, cy: f32, s: &str, size: f32, color: Color) {
        let px = self.px(size);
        let base = cy + self.text.center_offset(px);
        self.text.draw(self.dl, x, base, s, px, color);
    }

    pub fn text_at(&mut self, x: f32, cy: f32, s: &str, size: f32, color: Color) {
        self.label(x, cy, s, size, color);
    }

    /// Texte calé à droite sur `right`.
    pub fn text_right(&mut self, right: f32, cy: f32, s: &str, size: f32, color: Color) {
        let w = self.measure(s, size);
        self.label(right - w, cy, s, size, color);
    }

    fn centered(&mut self, r: Rect, s: &str, size: f32, color: Color) {
        let w = self.measure(s, size);
        self.label(r.cx() - w / 2.0, r.cy(), s, size, color);
    }

    pub fn clip(&mut self, c: Option<Rect>) {
        self.dl.set_clip(c);
        self.ui.set_clip(c);
    }

    pub fn icon(&mut self, r: Rect, icon: Icon, color: Color) {
        let (cx, cy, u) = (r.cx(), r.cy(), r.w.min(r.h) / 24.0);
        let t = 2.2 * u;
        let p = |x: f32, y: f32| [cx + x * u, cy + y * u];
        match icon {
            Icon::List => {
                for y in [-6.0, 0.0, 6.0] {
                    self.dl.rrect(Rect::new(cx - 8.5 * u - 1.3 * u, cy + (y - 1.3) * u, 2.6 * u, 2.6 * u), 1.3 * u, color);
                    self.dl.line(p(-4.0, y), p(9.0, y), t, color);
                }
            }
            Icon::Sliders => {
                for (y, k) in [(-6.0, 3.0), (0.0, -3.5), (6.0, 2.0)] {
                    self.dl.line(p(-9.0, y), p(9.0, y), 1.6 * u, color);
                    self.dl.rrect(Rect::new(cx + (k - 2.5) * u, cy + (y - 2.5) * u, 5.0 * u, 5.0 * u), 2.5 * u, color);
                }
            }
            Icon::Help => self.centered(r, "?", 0.85 * r.h / self.dp / self.theme.texte, color),
            Icon::Menu => {
                for y in [-6.5, 0.0, 6.5] {
                    self.dl.rrect(Rect::new(cx - 1.7 * u, cy + (y - 1.7) * u, 3.4 * u, 3.4 * u), 1.7 * u, color);
                }
            }
            Icon::Close => {
                self.dl.line(p(-6.0, -6.0), p(6.0, 6.0), t, color);
                self.dl.line(p(6.0, -6.0), p(-6.0, 6.0), t, color);
            }
        }
    }

    /// Bouton flottant à icône, carré de la taille des commandes, à partir de (x, y).
    pub fn pastille(&mut self, id: Id, x: f32, y: f32, icon: Icon, active: bool) -> bool {
        let h = self.commande();
        let r = Rect::new(x, y, h, h);
        let c = self.couleurs();
        let clicked = self.ui.click(id, r, None);
        let pressed = self.ui.pressed(id);
        let rayon = self.theme.rayon(h);
        self.dl.shadow(Rect::new(r.x, r.y + 2.0 * self.dp, r.w, r.h), rayon, 8.0 * self.dp, c.ombre);
        let bg = if active { c.accent } else if pressed { c.puce } else { c.surface };
        self.dl.rrect(r, rayon, bg);
        self.icon(r.inset(r.w * 0.2), icon, if active { c.sur_accent } else { c.accent });
        clicked
    }

    /// Pilule flottante, à la taille des commandes, à partir de (x, y).
    pub fn pill(&mut self, id: Id, x: f32, y: f32, label: &str, active: bool) -> bool {
        let r = Rect::new(x, y, self.pill_width(label), self.commande());
        let c = self.couleurs();
        let clicked = self.ui.click(id, r, None);
        let pressed = self.ui.pressed(id);
        let rayon = self.theme.rayon(r.h);
        self.dl.shadow(Rect::new(r.x, r.y + 2.0 * self.dp, r.w, r.h), rayon, 6.0 * self.dp, c.ombre);
        self.dl.rrect(r, rayon, if active { c.accent } else if pressed { c.puce } else { c.surface });
        self.centered(r, label, 15.0, if active { c.sur_accent } else { c.texte });
        clicked
    }

    pub fn pill_width(&mut self, label: &str) -> f32 {
        self.measure(label, 15.0) + 0.75 * self.commande()
    }

    /// Rangée d'onglets, à la taille des commandes, à partir de (x, y). Renvoie
    /// l'onglet choisi.
    pub fn tabs(&mut self, id: Id, x: f32, y: f32, labels: &[&str], selected: usize) -> usize {
        let (mut x, mut result) = (x, selected);
        let (h, c) = (self.commande(), self.couleurs());
        for (i, label) in labels.iter().enumerate() {
            let w = self.measure(label, 15.0) + 0.66 * h;
            let t = Rect::new(x, y, w, h);
            if self.ui.click(sub_id(id, i), t, None) {
                result = i;
            }
            let on = i == selected;
            self.dl.rrect(t, self.theme.rayon(h), if on { c.accent } else { c.puce });
            self.centered(t, label, 15.0, if on { c.sur_accent } else { c.texte });
            x += w + 6.0 * self.dp;
        }
        result
    }

    pub fn body(&mut self, id: Id, rect: Rect, scrolls: &Scrolls) -> Body {
        self.ui.region_add(id, rect, Sense::Scroll);
        let delta = self.ui.take_scroll(id);
        let (off, content) = scrolls.get(&id).copied().unwrap_or((0.0, 0.0));
        let off = (off - delta).clamp(0.0, (content - rect.h).max(0.0));
        self.clip(Some(rect));
        let pad = 14.0 * self.dp;
        Body { x: rect.x + pad, w: rect.w - 2.0 * pad, y: rect.y + 8.0 * self.dp - off, top: rect.y - off, id, rect }
    }

    pub fn body_end(&mut self, b: Body, scrolls: &mut Scrolls) {
        self.clip(None);
        let content = b.y - b.top + 12.0 * self.dp;
        let off = b.rect.y - b.top;
        scrolls.insert(b.id, (off, content));
        if content > b.rect.h {
            let h = (b.rect.h * b.rect.h / content).max(32.0 * self.dp);
            let y = b.rect.y + off / (content - b.rect.h) * (b.rect.h - h);
            self.dl.rrect(Rect::new(b.rect.right() - 6.0 * self.dp, y, 3.0 * self.dp, h), 1.5 * self.dp, [0x6C, 0x6C, 0x80, 120]);
        }
    }

    pub fn section(&mut self, b: &mut Body, title: &str) {
        b.y += 10.0 * self.dp;
        let c = self.couleurs();
        self.label(b.x, b.y + 12.0 * self.dp * self.theme.texte, title, 12.5, c.pale);
        b.y += (10.0 + 18.0 * self.theme.texte) * self.dp;
    }

    pub fn paragraph(&mut self, b: &mut Body, s: &str) {
        let (px, c, k) = (self.px(15.0), self.couleurs(), self.theme.texte);
        for line in self.text.wrap(s, px, b.w) {
            self.label(b.x, b.y + 11.0 * self.dp * k, &line, 15.0, c.texte);
            b.y += 22.0 * self.dp * k;
        }
        b.y += 8.0 * self.dp;
    }

    /// Ligne de liste bord à bord. Renvoie vrai à l'appui.
    pub fn row(&mut self, b: &mut Body, id: Id, label: &str, swatch: Option<Color>, selected: bool) -> bool {
        let r = Rect::new(b.rect.x, b.y, b.rect.w, (self.theme.ligne * self.dp).round());
        let c = self.couleurs();
        let clicked = self.ui.click(id, r, Some(b.id));
        if self.ui.pressed(id) || selected {
            self.dl.rrect(r.inset(2.0 * self.dp), self.theme.rayon(r.h) * 0.45, c.puce);
        }
        let mut x = b.x;
        if let Some(sw) = swatch {
            let s = 22.0 * self.dp;
            self.dl.rrect(Rect::new(x, r.cy() - s / 2.0, s, s), 6.0 * self.dp * self.theme.arrondi.max(0.2), sw);
            x += s + 12.0 * self.dp;
        }
        self.label(x, r.cy(), label, 16.0, if selected { c.accent } else { c.texte });
        b.y += r.h;
        clicked
    }

    pub fn toggle(&mut self, b: &mut Body, id: Id, label: &str, value: &mut bool) {
        let r = Rect::new(b.x, b.y, b.w, (self.theme.ligne * self.dp).round());
        let c = self.couleurs();
        if self.ui.click(id, r, Some(b.id)) {
            *value = !*value;
        }
        self.label(b.x, r.cy(), label, 16.0, c.texte);
        let (w, h) = (46.0 * self.dp, 26.0 * self.dp);
        let track = Rect::new(r.right() - w, r.cy() - h / 2.0, w, h);
        let rt = self.theme.rayon(h);
        self.dl.rrect(track, rt, if *value { c.accent } else { c.ligne });
        let k = h - 6.0 * self.dp;
        let kx = if *value { track.right() - k - 3.0 * self.dp } else { track.x + 3.0 * self.dp };
        self.dl.rrect(Rect::new(kx, track.y + 3.0 * self.dp, k, k), self.theme.rayon(k), if *value { c.sur_accent } else { c.surface });
        b.y += r.h;
    }

    pub fn slider(&mut self, b: &mut Body, id: Id, label: &str, shown: &str, value: &mut f32, lo: f32, hi: f32) {
        let r = Rect::new(b.x, b.y, b.w, (64.0 * self.dp * self.theme.texte.max(1.0)).round());
        let c = self.couleurs();
        let ty = r.y + 16.0 * self.dp * self.theme.texte;
        self.label(b.x, ty, label, 16.0, c.texte);
        self.text_right(r.right(), ty, shown, 14.0, c.pale);
        let track_y = r.bottom() - 20.0 * self.dp;
        let knob = 11.0 * self.dp;
        let (x0, x1) = (r.x + knob, r.right() - knob);
        if let Some(d) = self.ui.drag(id, r) {
            *value = lo + ((d.x - x0) / (x1 - x0)).clamp(0.0, 1.0) * (hi - lo);
        }
        let t = 4.0 * self.dp;
        let at = x0 + (*value - lo) / (hi - lo) * (x1 - x0);
        self.dl.rrect(Rect::new(x0, track_y - t / 2.0, x1 - x0, t), t / 2.0, c.ligne);
        self.dl.rrect(Rect::new(x0, track_y - t / 2.0, at - x0, t), t / 2.0, c.accent);
        let grow = if self.ui.pressed(id) { 2.0 * self.dp } else { 0.0 };
        self.dl.rrect(Rect::new(at - knob - grow, track_y - knob - grow, 2.0 * (knob + grow), 2.0 * (knob + grow)), self.theme.rayon(2.0 * (knob + grow)), c.accent);
        b.y += r.h;
    }

    pub fn button(&mut self, b: &mut Body, id: Id, label: &str) -> bool {
        let r = Rect::new(b.x, b.y + 4.0 * self.dp, b.w, self.commande());
        let c = self.couleurs();
        let clicked = self.ui.click(id, r, Some(b.id));
        self.dl.rrect(r, self.theme.rayon(r.h), if self.ui.pressed(id) { c.ligne } else { c.puce });
        self.centered(r, label, 15.0, c.accent);
        b.y += r.h + 8.0 * self.dp;
        clicked
    }
}
