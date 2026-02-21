use crate::core::currency::CurrencyCode;
use crate::core::obligation::Obligation;
use crate::core::party::PartyId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tracks the net position of each party in each currency.
///
/// A positive balance means the party is owed (net creditor).
/// A negative balance means the party owes (net debtor).
///
/// The ledger is the output of the netting process — it shows
/// what each party actually needs to pay or receive after optimization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// (PartyId, CurrencyCode) -> net balance
    /// Positive = net creditor, Negative = net debtor
    #[serde(with = "positions_serde")]
    positions: HashMap<(PartyId, CurrencyCode), Decimal>,
}

mod positions_serde {
    use super::*;
    use serde::ser::SerializeMap;
    use serde::de::{self, MapAccess, Visitor};

    pub fn serialize<S: serde::Serializer>(
        positions: &HashMap<(PartyId, CurrencyCode), Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(positions.len()))?;
        for ((party, currency), amount) in positions {
            map.serialize_entry(&format!("{}:{}", party, currency), amount)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<(PartyId, CurrencyCode), Decimal>, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = HashMap<(PartyId, CurrencyCode), Decimal>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map with \"party:currency\" keys")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
                let mut map = HashMap::new();
                while let Some((key, value)) = access.next_entry::<String, Decimal>()? {
                    let (party, currency) = key.split_once(':')
                        .ok_or_else(|| de::Error::custom(format!("invalid key: {key}")))?;
                    map.insert((PartyId::new(party), CurrencyCode::new(currency)), value);
                }
                Ok(map)
            }
        }
        deserializer.deserialize_map(V)
    }
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an obligation: debtor loses, creditor gains.
    pub fn apply_obligation(&mut self, obligation: &Obligation) {
        let debtor_key = (obligation.debtor().clone(), obligation.currency().clone());
        let creditor_key = (obligation.creditor().clone(), obligation.currency().clone());

        *self.positions.entry(debtor_key).or_insert(Decimal::ZERO) -= obligation.amount();
        *self.positions.entry(creditor_key).or_insert(Decimal::ZERO) += obligation.amount();
    }

    /// Get the net position of a party in a specific currency.
    pub fn position(&self, party: &PartyId, currency: &CurrencyCode) -> Decimal {
        self.positions
            .get(&(party.clone(), currency.clone()))
            .copied()
            .unwrap_or(Decimal::ZERO)
    }

    /// Get all positions for a given party across all currencies.
    pub fn positions_for_party(&self, party: &PartyId) -> HashMap<CurrencyCode, Decimal> {
        self.positions
            .iter()
            .filter(|((p, _), _)| p == party)
            .map(|((_, c), &v)| (c.clone(), v))
            .collect()
    }

    /// Get all non-zero positions.
    pub fn all_positions(&self) -> &HashMap<(PartyId, CurrencyCode), Decimal> {
        &self.positions
    }

    /// Verify that the ledger is balanced: sum of all positions per currency = 0.
    pub fn is_balanced(&self) -> bool {
        let mut currency_sums: HashMap<CurrencyCode, Decimal> = HashMap::new();
        for ((_, currency), amount) in &self.positions {
            *currency_sums.entry(currency.clone()).or_insert(Decimal::ZERO) += amount;
        }
        currency_sums.values().all(|sum| *sum == Decimal::ZERO)
    }

    /// Total absolute value of all net positions (sum of |position|).
    /// This represents the total amount that actually needs to settle.
    pub fn total_net_settlement(&self) -> Decimal {
        // Sum positive positions only (equivalent to sum of |negative| positions)
        self.positions
            .values()
            .filter(|v| **v > Decimal::ZERO)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_ledger_basic() {
        let mut ledger = Ledger::new();
        let ob = Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            CurrencyCode::new("USD"),
        );
        ledger.apply_obligation(&ob);

        assert_eq!(
            ledger.position(&PartyId::new("A"), &CurrencyCode::new("USD")),
            dec!(-100)
        );
        assert_eq!(
            ledger.position(&PartyId::new("B"), &CurrencyCode::new("USD")),
            dec!(100)
        );
    }

    #[test]
    fn test_ledger_balanced() {
        let mut ledger = Ledger::new();
        let usd = CurrencyCode::new("USD");
        ledger.apply_obligation(&Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            usd.clone(),
        ));
        ledger.apply_obligation(&Obligation::new(
            PartyId::new("B"),
            PartyId::new("A"),
            dec!(60),
            usd,
        ));
        assert!(ledger.is_balanced());
    }

    #[test]
    fn test_ledger_circular_cancels() {
        let mut ledger = Ledger::new();
        let usd = CurrencyCode::new("USD");
        ledger.apply_obligation(&Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            usd.clone(),
        ));
        ledger.apply_obligation(&Obligation::new(
            PartyId::new("B"),
            PartyId::new("C"),
            dec!(100),
            usd.clone(),
        ));
        ledger.apply_obligation(&Obligation::new(
            PartyId::new("C"),
            PartyId::new("A"),
            dec!(100),
            usd,
        ));

        // Perfect cycle: everyone's net position is zero
        assert_eq!(
            ledger.position(&PartyId::new("A"), &CurrencyCode::new("USD")),
            Decimal::ZERO
        );
        assert_eq!(ledger.total_net_settlement(), Decimal::ZERO);
    }
}
