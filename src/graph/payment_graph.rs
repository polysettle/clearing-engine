use crate::core::currency::CurrencyCode;
use crate::core::obligation::{Obligation, ObligationSet};
use crate::core::party::PartyId;
use crate::optimization::netting::{NettingEngine, NettingResult};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// A directed graph of payment obligations between parties.
///
/// Each edge represents an aggregate obligation from one party to another
/// in a specific currency. The graph supports multiple currencies simultaneously.
///
/// This is the primary input to the netting and optimization algorithms.
///
/// # Examples
///
/// ```
/// use clearing_engine::prelude::*;
/// use rust_decimal_macros::dec;
///
/// let mut graph = PaymentGraph::new();
/// let usd = CurrencyCode::new("USD");
///
/// graph.add_obligation(Obligation::new(
///     PartyId::new("A"), PartyId::new("B"), dec!(100), usd.clone(),
/// ));
/// graph.add_obligation(Obligation::new(
///     PartyId::new("B"), PartyId::new("A"), dec!(60), usd,
/// ));
///
/// assert_eq!(graph.party_count(), 2);
/// assert_eq!(graph.obligation_count(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct PaymentGraph {
    obligations: ObligationSet,
    /// Aggregated edges: (debtor, creditor, currency) -> total amount
    edges: HashMap<(PartyId, PartyId, CurrencyCode), Decimal>,
    /// All known parties
    parties: HashSet<PartyId>,
    /// All known currencies
    currencies: HashSet<CurrencyCode>,
}

impl PaymentGraph {
    pub fn new() -> Self {
        Self {
            obligations: ObligationSet::new(),
            edges: HashMap::new(),
            parties: HashSet::new(),
            currencies: HashSet::new(),
        }
    }

    /// Add a single obligation to the graph.
    pub fn add_obligation(&mut self, obligation: Obligation) {
        let key = (
            obligation.debtor().clone(),
            obligation.creditor().clone(),
            obligation.currency().clone(),
        );
        *self.edges.entry(key).or_insert(Decimal::ZERO) += obligation.amount();

        self.parties.insert(obligation.debtor().clone());
        self.parties.insert(obligation.creditor().clone());
        self.currencies.insert(obligation.currency().clone());
        self.obligations.add(obligation);
    }

    /// Load obligations from a set.
    pub fn from_obligations(obligations: Vec<Obligation>) -> Self {
        let mut graph = Self::new();
        for ob in obligations {
            graph.add_obligation(ob);
        }
        graph
    }

    /// Number of unique parties in the graph.
    pub fn party_count(&self) -> usize {
        self.parties.len()
    }

    /// Number of individual obligations loaded.
    pub fn obligation_count(&self) -> usize {
        self.obligations.len()
    }

    /// Number of unique currencies.
    pub fn currency_count(&self) -> usize {
        self.currencies.len()
    }

    /// All parties in the graph.
    pub fn parties(&self) -> &HashSet<PartyId> {
        &self.parties
    }

    /// All currencies in the graph.
    pub fn currencies(&self) -> &HashSet<CurrencyCode> {
        &self.currencies
    }

    /// Gross total of all obligations.
    pub fn gross_total(&self) -> Decimal {
        self.obligations.gross_total()
    }

    /// Get the aggregated amount owed from debtor to creditor in a given currency.
    pub fn edge_amount(
        &self,
        debtor: &PartyId,
        creditor: &PartyId,
        currency: &CurrencyCode,
    ) -> Decimal {
        self.edges
            .get(&(debtor.clone(), creditor.clone(), currency.clone()))
            .copied()
            .unwrap_or(Decimal::ZERO)
    }

    /// Get all edges as (debtor, creditor, currency, amount).
    pub fn edges(&self) -> Vec<(&PartyId, &PartyId, &CurrencyCode, Decimal)> {
        self.edges
            .iter()
            .map(|((d, c, cur), &amt)| (d, c, cur, amt))
            .collect()
    }

    /// Get outgoing edges from a party in a given currency.
    pub fn outgoing(
        &self,
        party: &PartyId,
        currency: &CurrencyCode,
    ) -> Vec<(&PartyId, Decimal)> {
        self.edges
            .iter()
            .filter(|((d, _, c), _)| d == party && c == currency)
            .map(|((_, creditor, _), &amt)| (creditor, amt))
            .collect()
    }

    /// Get incoming edges to a party in a given currency.
    pub fn incoming(
        &self,
        party: &PartyId,
        currency: &CurrencyCode,
    ) -> Vec<(&PartyId, Decimal)> {
        self.edges
            .iter()
            .filter(|((_, cr, c), _)| cr == party && c == currency)
            .map(|((debtor, _, _), &amt)| (debtor, amt))
            .collect()
    }

    /// Compute net positions using the netting engine.
    pub fn compute_net_positions(&self) -> NettingResult {
        NettingEngine::multilateral_net(&self.obligations)
    }

    /// Get the underlying obligation set.
    pub fn obligations(&self) -> &ObligationSet {
        &self.obligations
    }

    /// Build an adjacency list for a specific currency.
    /// Returns: party -> [(counterparty, amount)]
    pub fn adjacency_list(
        &self,
        currency: &CurrencyCode,
    ) -> HashMap<PartyId, Vec<(PartyId, Decimal)>> {
        let mut adj: HashMap<PartyId, Vec<(PartyId, Decimal)>> = HashMap::new();
        for party in &self.parties {
            adj.entry(party.clone()).or_default();
        }
        for ((debtor, creditor, cur), &amount) in &self.edges {
            if cur == currency {
                adj.entry(debtor.clone())
                    .or_default()
                    .push((creditor.clone(), amount));
            }
        }
        adj
    }
}

impl Default for PaymentGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_graph_basic() {
        let mut graph = PaymentGraph::new();
        let usd = CurrencyCode::new("USD");
        graph.add_obligation(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            usd.clone(),
        ));
        graph.add_obligation(Obligation::new(
            PartyId::new("B"),
            PartyId::new("C"),
            dec!(200),
            usd,
        ));

        assert_eq!(graph.party_count(), 3);
        assert_eq!(graph.obligation_count(), 2);
        assert_eq!(graph.gross_total(), dec!(300));
    }

    #[test]
    fn test_edge_aggregation() {
        let mut graph = PaymentGraph::new();
        let usd = CurrencyCode::new("USD");
        let a = PartyId::new("A");
        let b = PartyId::new("B");

        graph.add_obligation(Obligation::new(a.clone(), b.clone(), dec!(100), usd.clone()));
        graph.add_obligation(Obligation::new(a.clone(), b.clone(), dec!(50), usd.clone()));

        assert_eq!(graph.edge_amount(&a, &b, &usd), dec!(150));
    }

    #[test]
    fn test_multi_currency() {
        let mut graph = PaymentGraph::new();
        let a = PartyId::new("A");
        let b = PartyId::new("B");

        graph.add_obligation(Obligation::new(
            a.clone(),
            b.clone(),
            dec!(100),
            CurrencyCode::new("USD"),
        ));
        graph.add_obligation(Obligation::new(
            a.clone(),
            b.clone(),
            dec!(500),
            CurrencyCode::new("BRL"),
        ));

        assert_eq!(graph.currency_count(), 2);
        assert_eq!(
            graph.edge_amount(&a, &b, &CurrencyCode::new("USD")),
            dec!(100)
        );
        assert_eq!(
            graph.edge_amount(&a, &b, &CurrencyCode::new("BRL")),
            dec!(500)
        );
    }
}
