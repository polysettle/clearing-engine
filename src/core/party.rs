use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a party (counterparty) in the settlement network.
///
/// A party can represent a central bank, commercial bank, treasury,
/// clearing house, or any entity that participates in payment obligations.
///
/// # Examples
///
/// ```
/// use clearing_engine::core::party::PartyId;
///
/// let brazil = PartyId::new("BR-TREASURY");
/// let india = PartyId::new("IN-RBI");
/// assert_ne!(brazil, india);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PartyId(String);

impl PartyId {
    /// Create a new party identifier.
    ///
    /// Convention: use ISO 3166-1 alpha-2 country code prefix
    /// followed by institution identifier (e.g., "BR-TREASURY", "IN-RBI").
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the string representation of this party ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PartyId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_party_equality() {
        let a = PartyId::new("BR-TREASURY");
        let b = PartyId::new("BR-TREASURY");
        let c = PartyId::new("IN-RBI");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_party_display() {
        let p = PartyId::new("CN-PBOC");
        assert_eq!(format!("{}", p), "CN-PBOC");
    }

    #[test]
    fn test_party_ordering() {
        let a = PartyId::new("A-BANK");
        let b = PartyId::new("B-BANK");
        assert!(a < b);
    }
}
