//! Dessine l'interface sur le vrai processeur graphique, hors écran, et écrit
//! des images PPM. Sert à valider nuanceurs et mise en page sans installer l'APK.
//!
//!   cargo run --example apercu -- <dossier>

use rustine::demo::App;
use rustine::draw::{DrawList, Rect};
use rustine::gles::{load_gl, Egl};
use rustine::renderer::Renderer;
use rustine::text::Text;
use rustine::ui::{Event, Ui};
use rustine::widgets::Cx;

struct Shot {
    name: &'static str,
    w: i32,
    h: i32,
    dp: f32,
    start: bool,
    end: bool,
    help: bool,
    fenetres: bool,
}

fn main() {
    let dir = std::env::args().nth(1).expect("dossier de sortie");
    let shots = [
        Shot { name: "tablette-ferme", w: 1920, h: 1200, dp: 1.5, start: false, end: false, help: false, fenetres: false },
        Shot { name: "tablette-deux", w: 1920, h: 1200, dp: 1.5, start: true, end: true, help: false, fenetres: false },
        Shot { name: "tablette-portrait", w: 1200, h: 1920, dp: 1.5, start: true, end: true, help: true, fenetres: false },
        Shot { name: "tablette-fenetres", w: 1920, h: 1200, dp: 1.5, start: false, end: false, help: false, fenetres: true },
        Shot { name: "telephone", w: 2400, h: 1080, dp: 2.5, start: false, end: true, help: false, fenetres: false },
    ];
    let mut egl = Egl::new(true).expect("EGL");
    egl.attach_pbuffer(2400, 2400).expect("pbuffer");
    let render = Renderer::new(load_gl(true).expect("GL")).expect("nuanceurs");
    println!("nuanceurs compilés et liés");
    let mut text = Text::new();

    for s in shots {
        let (w, h) = (s.w as f32, s.h as f32);
        let screen = Rect::new(0.0, 0.0, w, h);
        let mut app = App::new(s.dp);
        let mut ui = Ui::new(s.dp);
        let mut dl = DrawList::default();
        app.shell.start.open = s.start;
        app.shell.end.open = s.end;
        if s.fenetres {
            // Trois fenêtres : un cube à gauche, une liste et l'aide à droite
            use rustine::surface::Sens;
            app.surface.partager(1, Sens::Colonnes, 0.6);
            let liste = app.surface.partager(1, Sens::Lignes, 0.55).unwrap();
            app.surface.changer_vue(liste, rustine::demo::VUE_COULEURS);
            app.surface.changer_vue(1, rustine::demo::VUE_AIDE);
        }

        let mut scene = None;
        for i in 0..60 {
            if i == 30 && s.help {
                // Vraie séquence tactile : appui sur la pastille d'aide, dans la barre du bas.
                let c = 48.0 * s.dp;
                ui.event(Event::Down { id: 0, x: 3.5 * c, y: h - c / 2.0 }, 0.0);
                ui.event(Event::Up { id: 0 }, 0.05);
            }
            dl.clear();
            let mut cx = Cx { ui: &mut ui, text: &mut text, dl: &mut dl, dp: s.dp, theme: Default::default() };
            scene = Some(app.frame(&mut cx, screen, 0.05));
            ui.end_frame();
        }
        render.draw(w, h, &scene.unwrap(), &dl, &mut text);

        let gl = render.gl();
        let mut px = vec![0u8; (s.w * s.h * 4) as usize];
        unsafe {
            (gl.glReadPixels)(0, 0, s.w, s.h, 0x1908, 0x1401, px.as_mut_ptr() as *mut _);
            let e = (gl.glGetError)();
            println!("{} : {} sommets, erreur GL {e:#x}", s.name, dl.vertices.len());
        }
        let mut out = format!("P6\n{} {}\n255\n", s.w, s.h).into_bytes();
        for y in (0..s.h as usize).rev() {
            for x in 0..s.w as usize {
                out.extend_from_slice(&px[(y * s.w as usize + x) * 4..][..3]);
            }
        }
        std::fs::write(format!("{dir}/{}.ppm", s.name), out).unwrap();
    }
}
