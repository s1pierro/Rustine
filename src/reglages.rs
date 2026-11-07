//! Réglages de l'interface, dans un fichier texte qu'on peut ouvrir et modifier à la
//! main : une valeur par ligne, `clé = valeur`, des commentaires après `#`.
//!
//! Seule l'interface y figure : l'apparence (`Theme`) et la disposition des fenêtres.
//! Une clé inconnue, absente ou mal écrite garde sa valeur par défaut, et les nombres
//! sont ramenés dans leurs bornes : un fichier abîmé ne casse rien. Le fichier est
//! réécrit en entier, avec ses commentaires, quand un réglage change dans l'application.

use crate::widgets::{Theme, ACCENTS};
use std::path::PathBuf;

/// Le nom du fichier, dans le dossier de l'application.
pub const NOM: &str = "interface.txt";

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Reglages {
    pub theme: Theme,
    /// La disposition des fenêtres, telle que `Surface::ecrire` l'écrit.
    pub fenetres: Option<String>,
}

/// Les paires `clé = valeur` d'un texte, sans les commentaires ni les lignes vides.
pub fn cles(texte: &str) -> Vec<(String, String)> {
    texte
        .lines()
        .filter_map(|l| {
            let l = l.split('#').next().unwrap_or("").trim();
            let (k, v) = l.split_once('=')?;
            Some((k.trim().to_lowercase(), v.trim().to_string()))
        })
        .collect()
}

fn nombre(v: &str) -> Option<f32> {
    v.trim_end_matches('%').trim().replace(',', ".").parse().ok().filter(|x: &f32| x.is_finite())
}

fn oui(v: &str) -> Option<bool> {
    match v.to_lowercase().as_str() {
        "oui" | "vrai" | "1" | "yes" | "true" => Some(true),
        "non" | "faux" | "0" | "no" | "false" => Some(false),
        _ => None,
    }
}

impl Reglages {
    pub fn lire(texte: &str) -> Reglages {
        let mut r = Reglages::default();
        let t = &mut r.theme;
        for (k, v) in cles(texte) {
            match k.as_str() {
                "commandes" => t.commande = nombre(&v).map_or(t.commande, |x| x.round().clamp(36.0, 64.0)),
                "lignes" => t.ligne = nombre(&v).map_or(t.ligne, |x| x.round().clamp(40.0, 64.0)),
                "texte" => t.texte = nombre(&v).map_or(t.texte, |x| (x / 100.0).clamp(0.85, 1.3)),
                "arrondi" => t.arrondi = nombre(&v).map_or(t.arrondi, |x| (x / 100.0).clamp(0.0, 1.0)),
                "sombre" => t.sombre = oui(&v).unwrap_or(t.sombre),
                "accent" => t.accent = ACCENTS.iter().position(|a| a.0.eq_ignore_ascii_case(&v)).unwrap_or(t.accent),
                "fenetres" if !v.is_empty() => r.fenetres = Some(v),
                _ => {}
            }
        }
        r
    }

    pub fn ecrire(&self) -> String {
        let t = &self.theme;
        let accents: Vec<String> = ACCENTS.iter().map(|a| a.0.to_lowercase()).collect();
        let mut s = String::new();
        s += "# Réglages de l'interface de Rustine\n";
        s += "#\n";
        s += "# Ce fichier se modifie à la main : une valeur par ligne, « clé = valeur ».\n";
        s += "# Tout ce qui suit # est ignoré. Une valeur absente ou mal écrite reprend sa\n";
        s += "# valeur par défaut. L'application réécrit ce fichier quand un réglage change.\n\n";
        s += "# Taille des pastilles, pilules, boutons et onglets, en dp (36 à 64 ; 48)\n";
        s += &format!("commandes = {:.0}\n\n", t.commande);
        s += "# Hauteur des lignes des listes, en dp (40 à 64 ; 48)\n";
        s += &format!("lignes = {:.0}\n\n", t.ligne);
        s += "# Taille du texte, en % (85 à 130 ; 100)\n";
        s += &format!("texte = {:.0}\n\n", t.texte * 100.0);
        s += "# Arrondi des commandes, en % (0 : coins presque droits ; 100 : bouts ronds)\n";
        s += &format!("arrondi = {:.0}\n\n", t.arrondi * 100.0);
        s += "# Thème sombre : oui ou non\n";
        s += &format!("sombre = {}\n\n", if t.sombre { "oui" } else { "non" });
        s += &format!("# Couleur d'accent : {}\n", accents.join(", "));
        s += &format!("accent = {}\n\n", accents[t.accent.min(accents.len() - 1)]);
        s += "# Disposition des fenêtres : F<numéro>:<vue> pour une fenêtre ; C<part>(…,…)\n";
        s += "# pour deux fenêtres côte à côte, L<part>(…,…) pour l'une sur l'autre. La part\n";
        s += "# est celle de la première, de 0.1 à 0.9.\n";
        if let Some(f) = &self.fenetres {
            s += &format!("fenetres = {f}\n");
        }
        s
    }
}

/// Le fichier des réglages sur le disque. Il est relu au démarrage et au retour dans
/// l'application (il a pu être modifié entre-temps), et réécrit quand les réglages
/// changent.
pub struct Fichier {
    pub chemin: PathBuf,
    /// Ce qu'on a lu ou écrit en dernier : rien à faire tant que les réglages s'écrivent pareil.
    connu: String,
}

impl Fichier {
    pub fn new(chemin: PathBuf) -> Fichier {
        Fichier { chemin, connu: String::new() }
    }

    /// Le contenu du fichier, s'il a changé depuis la dernière lecture ou écriture.
    pub fn relire(&mut self) -> Option<Reglages> {
        let texte = std::fs::read_to_string(&self.chemin).ok()?;
        if texte == self.connu {
            return None;
        }
        let r = Reglages::lire(&texte);
        // Ce qu'on réécrirait à partir de ces réglages : un fichier simplement
        // reformaté n'est pas réécrit pour autant
        self.connu = r.ecrire();
        Some(r)
    }

    /// Écrit les réglages s'ils ont changé. Renvoie l'erreur éventuelle.
    pub fn suivre(&mut self, r: &Reglages) -> Result<(), String> {
        let texte = r.ecrire();
        if texte == self.connu {
            return Ok(());
        }
        if let Some(d) = self.chemin.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        // Écrit à côté puis renomme : un fichier n'est jamais à moitié écrit
        let tmp = self.chemin.with_extension("txt~");
        std::fs::write(&tmp, &texte).and_then(|_| std::fs::rename(&tmp, &self.chemin)).map_err(|e| e.to_string())?;
        self.connu = texte;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_reglages_se_relisent() {
        let r = Reglages {
            theme: Theme { commande: 56.0, ligne: 44.0, texte: 1.15, arrondi: 0.3, accent: 4, sombre: true },
            fenetres: Some("C0.6000(F2:0,F1:3)".into()),
        };
        let lu = Reglages::lire(&r.ecrire());
        assert_eq!(lu.fenetres, r.fenetres);
        let t = lu.theme;
        assert_eq!((t.commande, t.ligne, t.accent, t.sombre), (56.0, 44.0, 4, true));
        assert!((t.texte - 1.15).abs() < 1e-4 && (t.arrondi - 0.3).abs() < 1e-4);
    }

    #[test]
    fn un_fichier_ecrit_a_la_main_est_tolere() {
        let r = Reglages::lire(
            "# essai\n  COMMANDES=200   # trop grand\ntexte = 110 %\narrondi = abc\naccent = Vert\nsombre = oui\ninconnue = 3\nligne sans égal\n",
        );
        let t = r.theme;
        assert_eq!(t.commande, 64.0);
        assert!((t.texte - 1.1).abs() < 1e-4);
        assert_eq!(t.arrondi, Theme::default().arrondi);
        assert_eq!(t.accent, 3);
        assert!(t.sombre);
        assert_eq!(r.fenetres, None);
        assert_eq!(Reglages::lire(""), Reglages::default());
    }

    #[test]
    fn le_fichier_ne_s_ecrit_que_s_il_change() {
        let dir = std::env::temp_dir().join(format!("rustine-reglages-{}", std::process::id()));
        let mut f = Fichier::new(dir.join(NOM));
        assert!(f.relire().is_none());
        let mut r = Reglages::default();
        f.suivre(&r).unwrap();
        assert!(f.relire().is_none(), "on relit ce qu'on vient d'écrire : rien de neuf");
        // Une retouche à la main est relue
        std::fs::write(&f.chemin, "accent = rose\n").unwrap();
        assert_eq!(f.relire().unwrap().theme.accent, 5);
        r.theme.accent = 5;
        // …et ne provoque pas de réécriture tant que rien ne change dans l'application
        f.suivre(&r).unwrap();
        assert_eq!(std::fs::read_to_string(&f.chemin).unwrap(), "accent = rose\n");
        let _ = std::fs::remove_dir_all(dir);
    }
}
