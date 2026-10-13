//! Identifiant IMDb validé (`tt` + chiffres), utilisé comme clé par Stremio.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImdbId(String);

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidImdbId;

impl fmt::Display for InvalidImdbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("identifiant IMDb invalide (attendu : tt + 7 à 10 chiffres)")
    }
}

impl std::error::Error for InvalidImdbId {}

impl ImdbId {
    pub fn parse(raw: &str) -> Result<Self, InvalidImdbId> {
        let digits = raw.strip_prefix("tt").ok_or(InvalidImdbId)?;
        let valid = (7..=10).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit());
        if valid {
            Ok(Self(raw.to_owned()))
        } else {
            Err(InvalidImdbId)
        }
    }

    /// Tous les identifiants trouvés dans un texte ou un lien, sans doublon, dans l'ordre.
    pub fn find_all(text: &str) -> Vec<Self> {
        let mut ids: Vec<Self> = Vec::new();
        for id in text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter_map(|w| Self::parse(w).ok())
        {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImdbId {
    type Error = InvalidImdbId;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}

impl From<ImdbId> for String {
    fn from(id: ImdbId) -> Self {
        id.0
    }
}

impl fmt::Display for ImdbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_ids() {
        assert_eq!(
            ImdbId::parse("tt0111161").map(|i| i.to_string()),
            Ok("tt0111161".into())
        );
        assert!(ImdbId::parse("tt10872600").is_ok());
    }

    #[test]
    fn rejects_invalid_ids() {
        for raw in [
            "",
            "tt",
            "tt12345",
            "0111161",
            "TT0111161",
            "tt0111161x",
            "tt01111611234",
        ] {
            assert_eq!(ImdbId::parse(raw), Err(InvalidImdbId), "{raw}");
        }
    }

    #[test]
    fn finds_id_in_imdb_link() {
        let ids = ImdbId::find_all("https://www.imdb.com/fr/title/tt0111161/?ref_=chttp_t_1");
        assert_eq!(ids, vec![ImdbId("tt0111161".into())]);
    }

    #[test]
    fn finds_several_ids_without_duplicates() {
        let ids = ImdbId::find_all("tt0068646 puis tt0111161, et encore tt0068646");
        assert_eq!(
            ids,
            vec![ImdbId("tt0068646".into()), ImdbId("tt0111161".into())]
        );
    }

    #[test]
    fn ignores_ids_glued_to_other_words() {
        assert_eq!(ImdbId::find_all("att0111161 tt0111161abc"), []);
    }

    #[test]
    fn serde_rejects_invalid_id() {
        assert!(serde_json::from_str::<ImdbId>("\"nope\"").is_err());
        assert!(serde_json::from_str::<ImdbId>("\"tt0111161\"").is_ok());
    }
}
