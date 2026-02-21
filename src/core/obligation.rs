use crate::core::currency::CurrencyCode;
use crate::core::party::PartyId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A directed payment obligation between two parties.
///
/// Represents the fact that `debtor` owes `creditor` a specific `amount`
/// denominated in `currency`. This is the atomic unit of the clearing graph.
///
/// Obligations are immutable once created. The clearing engine operates
/// on collections of obligations to compute net positions.
///
/// # Examples
///
/// ```
/// use clearing_engine::core::obligation::Obligation;
/// use clearing_engine::core::party::PartyId;
/// use clearing_engine::core::currency::CurrencyCode;
/// use rust_decimal_macros::dec;
///
/// let obligation = Obligation::new(
///     PartyId::new("BR-TREASURY"),
///     PartyId::new("IN-RBI"),
///     dec!(100_000_000),
///     CurrencyCode::new("USD"),
/// );
///
/// assert_eq!(obligation.amount(), dec!(100_000_000));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    /// Unique identifier for this obligation.
    id: Uuid,
    /// The party that owes the amount.
    debtor: PartyId,
    /// The party that is owed the amount.
    creditor: PartyId,
    /// The amount owed. Must be positive.
    amount: Decimal,
    /// The currency of denomination.
    currency: CurrencyCode,
    /// When this obligation was created.
    created_at: DateTime<Utc>,
    /// Optional settlement deadline.
    settlement_date: Option<DateTime<Utc>>,
    /// Optional reference or memo.
    reference: Option<String>,
}

impl Obligation {
    /// Create a new obligation.
    ///
    /// # Panics
    ///
    /// Panics if `amount` is not positive.
    pub fn new(
        debtor: PartyId,
        creditor: PartyId,
        amount: Decimal,
        currency: CurrencyCode,
    ) -> Self {
        assert!(
            amount > Decimal::ZERO,
            "Obligation amount must be positive, got {}",
            amount
        );
        Self {
            id: Uuid::new_v4(),
            debtor,
            creditor,
            amount,
            currency,
            created_at: Utc::now(),
            settlement_date: None,
            reference: None,
        }
    }

    /// Create an obligation with a specific ID (useful for testing / determinism).
    pub fn with_id(
        id: Uuid,
        debtor: PartyId,
        creditor: PartyId,
        amount: Decimal,
        currency: CurrencyCode,
    ) -> Self {
        assert!(amount > Decimal::ZERO);
        Self {
            id,
            debtor,
            creditor,
            amount,
            currency,
            created_at: Utc::now(),
            settlement_date: None,
            reference: None,
        }
    }

    /// Set the settlement date.
    pub fn with_settlement_date(mut self, date: DateTime<Utc>) -> Self {
        self.settlement_date = Some(date);
        self
    }

    /// Set a reference string.
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    // --- Accessors ---

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn debtor(&self) -> &PartyId {
        &self.debtor
    }

    pub fn creditor(&self) -> &PartyId {
        &self.creditor
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn settlement_date(&self) -> Option<DateTime<Utc>> {
        self.settlement_date
    }

    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
}

/// A collection of obligations that can be submitted to the clearing engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObligationSet {
    obligations: Vec<Obligation>,
}

impl ObligationSet {
    pub fn new() -> Self {
        Self {
            obligations: Vec::new(),
        }
    }

    pub fn add(&mut self, obligation: Obligation) {
        self.obligations.push(obligation);
    }

    pub fn obligations(&self) -> &[Obligation] {
        &self.obligations
    }

    pub fn len(&self) -> usize {
        self.obligations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.obligations.is_empty()
    }

    /// Total gross value of all obligations.
    pub fn gross_total(&self) -> Decimal {
        self.obligations.iter().map(|o| o.amount()).sum()
    }

    /// All unique parties referenced in this set.
    pub fn parties(&self) -> Vec<PartyId> {
        let mut parties: Vec<PartyId> = self
            .obligations
            .iter()
            .flat_map(|o| vec![o.debtor().clone(), o.creditor().clone()])
            .collect();
        parties.sort();
        parties.dedup();
        parties
    }

    /// All unique currencies referenced in this set.
    pub fn currencies(&self) -> Vec<CurrencyCode> {
        let mut currencies: Vec<CurrencyCode> = self
            .obligations
            .iter()
            .map(|o| o.currency().clone())
            .collect();
        currencies.sort();
        currencies.dedup();
        currencies
    }
}

impl FromIterator<Obligation> for ObligationSet {
    fn from_iter<T: IntoIterator<Item = Obligation>>(iter: T) -> Self {
        Self {
            obligations: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_obligation() -> Obligation {
        Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(1000),
            CurrencyCode::new("USD"),
        )
    }

    #[test]
    fn test_obligation_creation() {
        let ob = sample_obligation();
        assert_eq!(ob.debtor().as_str(), "A");
        assert_eq!(ob.creditor().as_str(), "B");
        assert_eq!(ob.amount(), dec!(1000));
        assert_eq!(ob.currency().as_str(), "USD");
    }

    #[test]
    #[should_panic(expected = "must be positive")]
    fn test_obligation_zero_amount() {
        Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            Decimal::ZERO,
            CurrencyCode::new("USD"),
        );
    }

    #[test]
    #[should_panic(expected = "must be positive")]
    fn test_obligation_negative_amount() {
        Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(-100),
            CurrencyCode::new("USD"),
        );
    }

    #[test]
    fn test_obligation_set_gross() {
        let mut set = ObligationSet::new();
        set.add(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            CurrencyCode::new("USD"),
        ));
        set.add(Obligation::new(
            PartyId::new("B"),
            PartyId::new("C"),
            dec!(200),
            CurrencyCode::new("USD"),
        ));
        assert_eq!(set.gross_total(), dec!(300));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_obligation_set_parties() {
        let mut set = ObligationSet::new();
        set.add(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            CurrencyCode::new("USD"),
        ));
        set.add(Obligation::new(
            PartyId::new("B"),
            PartyId::new("C"),
            dec!(200),
            CurrencyCode::new("USD"),
        ));
        let parties = set.parties();
        assert_eq!(parties.len(), 3);
    }
}
