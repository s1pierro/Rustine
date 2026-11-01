//! Démonstration : un cube dans l'espace de travail, un panneau de chaque côté,
//! des pastilles pour les déployer et des pilules flottantes.

use crate::draw::{Color, Rect};
use crate::math::{self, Mat4};
use crate::shell::{Layout, Shell, Side};
use crate::ui::{id, sub_id, Id};
use crate::widgets::{self, Cx, Icon, Scrolls};

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

/// Ce que le moteur de rendu doit dessiner dans l'espace de travail.
#[derive(Clone, Copy)]
pub struct Scene {
    pub workspace: Rect,
    pub mvp: Mat4,
    pub model: Mat4,
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
}

pub struct App {
    pub shell: Shell,
    cam: Camera,
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
            cam: HOME,
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
        self.spin || self.shell.animating()
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

    pub fn frame(&mut self, cx: &mut Cx, screen: Rect, dt: f32) -> Scene {
        let l = self.shell.layout(screen, dt);
        let w = l.workspace;
        self.gestures(cx, w);
        if self.spin {
            self.cam.yaw += self.speed.to_radians() * dt;
        }

        self.floating(cx, w);
        self.start_panel(cx, &l);
        self.end_panel(cx, &l);
        self.handles(cx, &l, screen);
        self.scene(w)
    }

    fn gestures(&mut self, cx: &mut Cx, w: Rect) {
        let g = cx.ui.take_gesture();
        let c = &mut self.cam;
        c.yaw -= g.orbit.0 * 0.008;
        c.pitch = (c.pitch + g.orbit.1 * 0.008).clamp(-1.5, 1.5);
        c.dist = (c.dist / g.zoom).clamp(1.8, 24.0);
        // Déplacement à l'échelle de l'image : le cube suit les doigts.
        let k = 2.0 * c.dist * (self.fov.to_radians() / 2.0).tan() / w.h.max(1.0);
        let (sy, cy, sp, cp) = (c.yaw.sin(), c.yaw.cos(), c.pitch.sin(), c.pitch.cos());
        let (right, up) = ([cy, 0.0, -sy], [-sy * sp, cp, -cy * sp]);
        for i in 0..3 {
            c.target[i] += -g.pan.0 * k * right[i] + g.pan.1 * k * up[i];
        }
    }

    fn floating(&mut self, cx: &mut Cx, w: Rect) {
        let d = cx.dp;
        let (size, gap, m) = (52.0 * d, 10.0 * d, 14.0 * d);
        let at = |i: f32| Rect::new(w.x + m + i * (size + gap), w.y + m, size, size);
        let (end, content) = (self.shell.end.open, self.end_content);
        if cx.pastille(OBJECTS, at(0.0), Icon::List, self.shell.start.open) {
            self.shell.start.toggle();
        }
        if cx.pastille(DISPLAY, at(1.0), Icon::Sliders, end && content == EndContent::Display) {
            self.toggle_end(EndContent::Display);
        }
        if cx.pastille(HELP, at(2.0), Icon::Help, end && content == EndContent::Help) {
            self.toggle_end(EndContent::Help);
        }

        // Le titre s'efface quand l'espace devient étroit.
        if w.w > 420.0 * d {
            cx.text_right(w.right() - m, w.y + m + 14.0 * d, "Rustine", 19.0, widgets::TEXT);
            cx.text_right(w.right() - m, w.y + m + 38.0 * d, "1 objet · 12 triangles", 14.0, widgets::DIM);
        }

        let h = 44.0 * d;
        let (w1, w2) = (cx.pill_width("Recadrer"), cx.pill_width("Rotation"));
        let x = w.cx() - (w1 + w2 + gap) / 2.0;
        let y = w.bottom() - h - m;
        if cx.pill(FIT, Rect::new(x, y, w1, h), "Recadrer", false) {
            self.cam = HOME;
        }
        if cx.pill(SPIN, Rect::new(x + w1 + gap, y, w2, h), "Rotation", self.spin) {
            self.spin = !self.spin;
        }
    }

    fn panel_frame(&self, cx: &mut Cx, id: Id, r: Rect, side: Side, landscape: bool) {
        cx.ui.region_add(id, r, crate::ui::Sense::Sink);
        cx.dl.rrect(r, 0.0, widgets::SURFACE);
        let t = cx.dp.max(1.0);
        let line = match (side, landscape) {
            (Side::Start, true) => Rect::new(r.right() - t, r.y, t, r.h),
            (Side::End, true) => Rect::new(r.x, r.y, t, r.h),
            (Side::Start, false) => Rect::new(r.x, r.bottom() - t, r.w, t),
            (Side::End, false) => Rect::new(r.x, r.y, r.w, t),
        };
        cx.dl.rrect(line, 0.0, widgets::LINE);
    }

    /// En-tête d'un panneau : renvoie le rectangle du corps, et vrai si ✕ a été touché.
    fn panel_header(&self, cx: &mut Cx, id_close: Id, r: Rect) -> (Rect, Rect, bool) {
        let d = cx.dp;
        let head = 60.0 * d;
        let close = Rect::new(r.right() - 50.0 * d, r.y + 8.0 * d, 44.0 * d, 44.0 * d);
        let clicked = cx.ui.click(id_close, close, None);
        cx.dl.rrect(close, 22.0 * d, widgets::CHIP);
        cx.icon(close.inset(11.0 * d), Icon::Close, widgets::TEXT);
        cx.dl.rrect(Rect::new(r.x, r.y + head - d, r.w, d.max(1.0)), 0.0, widgets::LINE);
        (Rect::new(r.x + 12.0 * d, r.y + 11.0 * d, r.w - 74.0 * d, 38.0 * d), Rect::new(r.x, r.y + head, r.w, r.h - head), clicked)
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
        cx.tabs(id("onglets-debut"), tabs, &["Couleurs"], 0);
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
                self.display_tab = cx.tabs(id("onglets-affichage"), tabs, &["Vue", "Cube"], self.display_tab);
                let scroll = sub_id(id("corps-affichage"), self.display_tab);
                let mut b = cx.body(scroll, body, &self.scrolls);
                if self.display_tab == 0 {
                    cx.section(&mut b, "CAMÉRA");
                    let shown = format!("{:.0}°", self.fov);
                    cx.slider(&mut b, id("champ"), "Champ de vision", &shown, &mut self.fov, 25.0, 90.0);
                    if cx.button(&mut b, id("recadrer-panneau"), "Recadrer") {
                        self.cam = HOME;
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
            EndContent::Help => {
                cx.tabs(id("onglets-aide"), tabs, &["Aide"], 0);
                let mut b = cx.body(id("corps-aide"), body, &self.scrolls);
                cx.section(&mut b, "GESTES");
                cx.paragraph(&mut b, "Un doigt sur l'espace de travail fait tourner la vue.");
                cx.paragraph(&mut b, "Deux doigts zooment et déplacent la vue.");
                cx.section(&mut b, "PANNEAUX");
                cx.paragraph(&mut b, "Les pastilles déploient les panneaux. L'espace de travail s'adapte à la place restante.");
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
            let color = if pressed { widgets::ACCENT } else { [0x6C, 0x6C, 0x80, 140] };
            let (long, thick) = (52.0 * d, 5.0 * d);
            let grip = if l.landscape {
                Rect::new(band.cx() - thick / 2.0, band.cy() - long / 2.0, thick, long)
            } else {
                Rect::new(band.cx() - long / 2.0, band.cy() - thick / 2.0, long, thick)
            };
            cx.dl.rrect(grip, thick / 2.0, color);
        }
    }

    fn scene(&self, w: Rect) -> Scene {
        let c = &self.cam;
        let (sy, cy, sp, cp) = (c.yaw.sin(), c.yaw.cos(), c.pitch.sin(), c.pitch.cos());
        let eye = [c.target[0] + c.dist * sy * cp, c.target[1] + c.dist * sp, c.target[2] + c.dist * cy * cp];
        let view = math::look_at(eye, c.target, [0.0, 1.0, 0.0]);
        // Le rapport suit l'espace de travail : c'est lui qui « adapte » la 3D aux panneaux.
        let proj = math::perspective(self.fov.to_radians(), w.w.max(1.0) / w.h.max(1.0), 0.1, 100.0);
        let model = math::scale(self.size);
        let [r, g, b, _] = COLORS[self.color].1;
        Scene {
            workspace: w,
            mvp: math::mul(&proj, &math::mul(&view, &model)),
            model,
            color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
            tint: if self.faces { 0.25 } else { 0.0 },
        }
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
            let mut cx = Cx { ui: &mut self.ui, text: &mut self.text, dl: &mut self.dl, dp: 1.5 };
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

    // Centre de la n-ième pastille quand aucun panneau n'est ouvert.
    fn pastille(n: f32) -> (f32, f32) {
        (1.5 * (14.0 + 26.0 + n * 62.0), 1.5 * (14.0 + 26.0))
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
        assert_eq!(r.frames(60).workspace, SCREEN);
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
        let yaw = r.app.cam.yaw;
        r.tap(200.0, 1.5 * (60.0 + 8.0 + 48.0 * 3.5));
        assert_eq!(r.app.color, 3);
        assert_eq!(r.app.cam.yaw, yaw);
    }

    #[test]
    fn glisser_sur_l_espace_de_travail_tourne_la_vue() {
        let mut r = Rig::new();
        let yaw = r.app.cam.yaw;
        r.ui.event(Event::Down { id: 0, x: 900., y: 600. }, 0.0);
        r.ui.event(Event::Move { id: 0, x: 1000., y: 600. }, 0.1);
        r.ui.event(Event::Move { id: 0, x: 1100., y: 600. }, 0.1);
        r.frames(1);
        assert!(r.app.cam.yaw < yaw);
    }

    #[test]
    fn rien_ne_redessine_au_repos() {
        let mut r = Rig::new();
        r.frames(3);
        assert!(!r.app.animating());
    }
}
