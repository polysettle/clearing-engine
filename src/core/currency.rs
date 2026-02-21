use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// ISO 4217-style currency code.
///
/// Supports standard fiat currencies (USD, BRL, INR, CNY, etc.)
/// as well as arbitrary currency identifiers for digital currencies
/// or experimental settlement units.
///
/// # Examples
///
/// ```
/// use clearing_engine::core::currency::CurrencyCode;
///
/// let usd = CurrencyCode::new("USD");
/// let brl = CurrencyCode::new("BRL");
/// assert_ne!(usd, brl);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CurrencyCode {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Errors arising from FX rate operations.
#[derive(Debug, Error)]
pub enum FxError {
    #[error("no FX rate available for {from} -> {to}")]
    RateNotFound {
        from: CurrencyCode,
        to: CurrencyCode,
    },
    #[error("FX rate must be positive, got {rate} for {from} -> {to}")]
    InvalidRate {
        from: CurrencyCode,
        to: CurrencyCode,
        rate: Decimal,
    },
}

/// A pair of currencies representing an exchange rate direction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencyPair {
    pub base: CurrencyCode,
    pub quote: CurrencyCode,
}

impl CurrencyPair {
    pub fn new(base: CurrencyCode, quote: CurrencyCode) -> Self {
        Self { base, quote }
    }
}

impl fmt::Display for CurrencyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

/// FX rate table for converting between currencies.
///
/// Stores direct rates and can compute inverse rates.
/// Used by the netting engine to normalize obligations into
/// a common settlement currency when performing multi-currency optimization.
///
/// # Examples
///
/// ```
/// use clearing_engine::core::currency::{CurrencyCode, FxRateTable};
/// use rust_decimal_macros::dec;
///
/// let mut rates = FxRateTable::new(CurrencyCode::new("USD"));
/// rates.set_rate(
///     CurrencyCode::new("BRL"),
///     CurrencyCode::new("USD"),
///     dec!(0.20),
/// ).unwrap();
///
/// let converted = rates.convert(
///     dec!(1000),
///     &CurrencyCode::new("BRL"),
///     &CurrencyCode::new("USD"),
/// ).unwrap();
/// assert_eq!(converted, dec!(200));
/// ```
#[derive(Debug, Clone)]
pub struct FxRateTable {
    /// The base currency for normalization.
    pub base_currency: CurrencyCode,
    /// Direct rates: (from, to) -> rate.
    rates: HashMap<(CurrencyCode, CurrencyCode), Decimal>,
}

impl FxRateTable {
    /// Create a new FX rate table with the given base currency.
    pub fn new(base_currency: CurrencyCode) -> Self {
        Self {
            base_currency,
            rates: HashMap::new(),
        }
    }

    /// Set a direct exchange rate: 1 unit of `from` = `rate` units of `to`.
    pub fn set_rate(
        &mut self,
        from: CurrencyCode,
        to: CurrencyCode,
        rate: Decimal,
    ) -> Result<(), FxError> {
        if rate <= Decimal::ZERO {
            return Err(FxError::InvalidRate {
                from,
                to,
                rate,
            });
        }
        // Store direct rate
        self.rates.insert((from.clone(), to.clone()), rate);
        // Store inverse
        self.rates
            .insert((to, from), Decimal::ONE / rate);
        Ok(())
    }

    /// Get the exchange rate from one currency to another.
    pub fn get_rate(&self, from: &CurrencyCode, to: &CurrencyCode) -> Result<Decimal, FxError> {
        if from == to {
            return Ok(Decimal::ONE);
        }
        self.rates
            .get(&(from.clone(), to.clone()))
            .copied()
            .ok_or_else(|| FxError::RateNotFound {
                from: from.clone(),
                to: to.clone(),
            })
    }

    /// Convert an amount from one currency to another.
    pub fn convert(
        &self,
        amount: Decimal,
        from: &CurrencyCode,
        to: &CurrencyCode,
    ) -> Result<Decimal, FxError> {
        let rate = self.get_rate(from, to)?;
        Ok(amount * rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_currency_code_equality() {
        let a = CurrencyCode::new("USD");
        let b = CurrencyCode::new("USD");
        assert_eq!(a, b);
    }

    #[test]
    fn test_fx_rate_table_direct() {
        let mut table = FxRateTable::new(CurrencyCode::new("USD"));
        table
            .set_rate(
                CurrencyCode::new("BRL"),
                CurrencyCode::new("USD"),
                dec!(0.20),
            )
            .unwrap();

        let rate = table
            .get_rate(&CurrencyCode::new("BRL"), &CurrencyCode::new("USD"))
            .unwrap();
        assert_eq!(rate, dec!(0.20));
    }

    #[test]
    fn test_fx_rate_table_inverse() {
        let mut table = FxRateTable::new(CurrencyCode::new("USD"));
        table
            .set_rate(
                CurrencyCode::new("BRL"),
                CurrencyCode::new("USD"),
                dec!(0.20),
            )
            .unwrap();

        let rate = table
            .get_rate(&CurrencyCode::new("USD"), &CurrencyCode::new("BRL"))
            .unwrap();
        assert_eq!(rate, dec!(5)); // 1 / 0.20
    }

    #[test]
    fn test_fx_convert() {
        let mut table = FxRateTable::new(CurrencyCode::new("USD"));
        table
            .set_rate(
                CurrencyCode::new("INR"),
                CurrencyCode::new("USD"),
                dec!(0.012),
            )
            .unwrap();

        let result = table
            .convert(
                dec!(1000),
                &CurrencyCode::new("INR"),
                &CurrencyCode::new("USD"),
            )
            .unwrap();
        assert_eq!(result, dec!(12));
    }

    #[test]
    fn test_same_currency_rate() {
        let table = FxRateTable::new(CurrencyCode::new("USD"));
        let rate = table
            .get_rate(&CurrencyCode::new("USD"), &CurrencyCode::new("USD"))
            .unwrap();
        assert_eq!(rate, Decimal::ONE);
    }

    #[test]
    fn test_invalid_rate() {
        let mut table = FxRateTable::new(CurrencyCode::new("USD"));
        let result = table.set_rate(
            CurrencyCode::new("BRL"),
            CurrencyCode::new("USD"),
            dec!(-0.5),
        );
        assert!(result.is_err());
    }
}
