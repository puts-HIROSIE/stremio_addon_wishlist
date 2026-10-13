//! Analyse d'un classement IMDb copié-collé depuis le navigateur.

/// Un film repéré dans le texte : titre affiché + année de sortie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartEntry {
    pub title: String,
    pub year: u16,
}

/// Extrait les films d'un classement IMDb collé en texte brut.
///
/// Chaque film suit le motif `#rang` / titre / ligne commençant par l'année.
/// Les blocs incomplets sont ignorés.
pub fn parse_chart(text: &str) -> Vec<ChartEntry> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    lines
        .windows(3)
        .filter_map(|window| match window {
            [rank, title, year_line] if is_rank(rank) => Some(ChartEntry {
                title: (*title).to_owned(),
                year: leading_year(year_line)?,
            }),
            _ => None,
        })
        .collect()
}

fn is_rank(line: &str) -> bool {
    line.strip_prefix('#')
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Année en tête de ligne (`19942h 22m…` → 1994).
pub(crate) fn leading_year(line: &str) -> Option<u16> {
    let year = line.get(..4)?;
    if year.bytes().all(|b| b.is_ascii_digit()) {
        year.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, year: u16) -> ChartEntry {
        ChartEntry {
            title: title.to_owned(),
            year,
        }
    }

    #[test]
    fn parses_raw_imdb_copy_paste() {
        let text = "\
Tim Robbins in Les Évadés (1994)
#1
Les Évadés
19942h 22mTous publics
9,3
 (3,3 M)

Marlon Brando in Le Parrain (1972)
#2
Le Parrain
19722h 55m12
9,2
 (2,3 M)
";
        assert_eq!(
            parse_chart(text),
            vec![entry("Les Évadés", 1994), entry("Le Parrain", 1972)]
        );
    }

    #[test]
    fn year_glued_to_odd_durations() {
        let text = "#85\nOld Boy\n20032h16\n#213\nSherlock Junior\n192445mTous publics\n";
        assert_eq!(
            parse_chart(text),
            vec![entry("Old Boy", 2003), entry("Sherlock Junior", 1924)]
        );
    }

    #[test]
    fn keeps_same_title_with_different_years() {
        let text = "#21\nLa vie est belle\n1946\n#27\nLa vie est belle\n1997\n";
        assert_eq!(
            parse_chart(text),
            vec![
                entry("La vie est belle", 1946),
                entry("La vie est belle", 1997)
            ]
        );
    }

    #[test]
    fn skips_block_without_year() {
        let text = "#1\nSans année\n2h 10m\n#2\nMatrix\n1999\n";
        assert_eq!(parse_chart(text), vec![entry("Matrix", 1999)]);
    }

    #[test]
    fn ignores_text_without_rank_markers() {
        assert_eq!(parse_chart("Matrix (1999)\nun film culte\n"), []);
        assert_eq!(parse_chart(""), []);
    }

    #[test]
    fn parses_full_top250_fixture() {
        let entries = parse_chart(include_str!("../data/imdb-top250.txt"));
        assert_eq!(entries.len(), 250);
        assert_eq!(entries.first(), Some(&entry("Les Évadés", 1994)));
        assert_eq!(entries.last(), Some(&entry("Gangs of Wasseypur", 2012)));
    }
}
