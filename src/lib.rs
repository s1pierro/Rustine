//! Rustine : socle d'applications Android en Rust, interface tactile et 3D.
//!
//! - `gles`, `renderer`, `android` : ce qui touche à OpenGL ES et à Android ;
//! - `draw`, `text`, `ui`, `widgets`, `shell` : l'interface, sans aucun appel
//!   OpenGL, donc testable sur n'importe quelle machine ;
//! - `demo` : l'application de démonstration.

pub mod android;
pub mod demo;
pub mod draw;
pub mod gles;
pub mod math;
pub mod renderer;
pub mod shell;
pub mod text;
pub mod ui;
pub mod widgets;
