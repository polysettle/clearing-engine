use crate::core::currency::CurrencyCode;
use crate::core::party::PartyId;
use crate::graph::payment_graph::PaymentGraph;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// A cycle in the payment graph — a circular flow of obligations
/// that can potentially be compressed to reduce gross settlement.
#[derive(Debug, Clone)]
pub struct PaymentCycle {
    /// Ordered list of parties forming the cycle.
    /// The last party has an obligation back to the first.
    pub parties: Vec<PartyId>,
    /// The currency in which this cycle exists.
    pub currency: CurrencyCode,
    /// The minimum edge weight along the cycle (bottleneck).
    /// This is the maximum amount that can be netted from this cycle.
    pub bottleneck: Decimal,
}

impl PaymentCycle {
    /// The number of parties (and edges) in this cycle.
    pub fn len(&self) -> usize {
        self.parties.len()
    }

    /// Total gross value that would be saved by compressing this cycle.
    /// Equal to bottleneck * number_of_edges.
    pub fn potential_savings(&self) -> Decimal {
        self.bottleneck * Decimal::from(self.parties.len())
    }
}

/// Detect all simple cycles in the payment graph for a given currency.
///
/// Uses Johnson's algorithm adapted for weighted directed graphs.
/// Returns cycles ordered by potential savings (largest first).
///
/// # Algorithm
///
/// For each node, performs a DFS to find all cycles passing through
/// that node. The bottleneck (minimum edge weight) determines
/// how much liquidity can be saved by compressing each cycle.
pub fn find_cycles(graph: &PaymentGraph, currency: &CurrencyCode) -> Vec<PaymentCycle> {
    let adj = graph.adjacency_list(currency);
    let parties: Vec<PartyId> = {
        let mut p: Vec<_> = graph.parties().iter().cloned().collect();
        p.sort();
        p
    };

    let mut all_cycles = Vec::new();

    // For each starting node, find cycles using DFS
    for start in &parties {
        let mut visited: HashSet<PartyId> = HashSet::new();
        let mut path: Vec<PartyId> = Vec::new();
        let mut path_set: HashSet<PartyId> = HashSet::new();

        dfs_find_cycles(
            start,
            start,
            &adj,
            &mut visited,
            &mut path,
            &mut path_set,
            currency,
            &mut all_cycles,
            graph,
        );
    }

    // Deduplicate cycles (same set of nodes in same order = same cycle)
    deduplicate_cycles(&mut all_cycles);

    // Sort by potential savings descending
    all_cycles.sort_by(|a, b| b.potential_savings().cmp(&a.potential_savings()));
    all_cycles
}

fn dfs_find_cycles(
    current: &PartyId,
    start: &PartyId,
    adj: &HashMap<PartyId, Vec<(PartyId, Decimal)>>,
    visited: &mut HashSet<PartyId>,
    path: &mut Vec<PartyId>,
    path_set: &mut HashSet<PartyId>,
    currency: &CurrencyCode,
    cycles: &mut Vec<PaymentCycle>,
    graph: &PaymentGraph,
) {
    path.push(current.clone());
    path_set.insert(current.clone());

    if let Some(neighbors) = adj.get(current) {
        for (next, _amount) in neighbors {
            if next == start && path.len() >= 2 {
                // Found a cycle back to start
                let cycle_parties = path.clone();
                let bottleneck = compute_bottleneck(&cycle_parties, currency, graph);
                if bottleneck > Decimal::ZERO {
                    cycles.push(PaymentCycle {
                        parties: cycle_parties,
                        currency: currency.clone(),
                        bottleneck,
                    });
                }
            } else if !path_set.contains(next) && !visited.contains(next) && next > start {
                // Only explore nodes "greater than" start to avoid duplicate cycles
                dfs_find_cycles(
                    next, start, adj, visited, path, path_set, currency, cycles, graph,
                );
            }
        }
    }

    path.pop();
    path_set.remove(current);
    // Mark as visited only after exploring all paths from start through current
    if current == start {
        visited.insert(current.clone());
    }
}

/// Compute the bottleneck (minimum edge weight) along a cycle.
fn compute_bottleneck(
    parties: &[PartyId],
    currency: &CurrencyCode,
    graph: &PaymentGraph,
) -> Decimal {
    let mut min = Decimal::MAX;
    for i in 0..parties.len() {
        let from = &parties[i];
        let to = &parties[(i + 1) % parties.len()];
        let amount = graph.edge_amount(from, to, currency);
        if amount < min {
            min = amount;
        }
    }
    min
}

/// Remove duplicate cycles (same nodes in rotated order).
fn deduplicate_cycles(cycles: &mut Vec<PaymentCycle>) {
    let mut seen: HashSet<Vec<PartyId>> = HashSet::new();
    cycles.retain(|cycle| {
        let canonical = canonical_form(&cycle.parties);
        seen.insert(canonical)
    });
}

/// Normalize a cycle to its canonical (smallest rotation) form.
fn canonical_form(parties: &[PartyId]) -> Vec<PartyId> {
    if parties.is_empty() {
        return Vec::new();
    }
    let n = parties.len();
    let mut best = parties.to_vec();
    for i in 1..n {
        let rotated: Vec<PartyId> = parties[i..]
            .iter()
            .chain(parties[..i].iter())
            .cloned()
            .collect();
        if rotated < best {
            best = rotated;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::obligation::Obligation;
    use rust_decimal_macros::dec;

    #[test]
    fn test_simple_cycle() {
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
            dec!(100),
            usd.clone(),
        ));
        graph.add_obligation(Obligation::new(
            PartyId::new("C"),
            PartyId::new("A"),
            dec!(100),
            usd.clone(),
        ));

        let cycles = find_cycles(&graph, &usd);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
        assert_eq!(cycles[0].bottleneck, dec!(100));
        assert_eq!(cycles[0].potential_savings(), dec!(300));
    }

    #[test]
    fn test_no_cycle() {
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
            dec!(100),
            usd.clone(),
        ));

        let cycles = find_cycles(&graph, &usd);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_asymmetric_cycle() {
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
            PartyId::new("A"),
            dec!(60),
            usd.clone(),
        ));

        let cycles = find_cycles(&graph, &usd);
        assert_eq!(cycles.len(), 1);
        // Bottleneck is the smaller edge
        assert_eq!(cycles[0].bottleneck, dec!(60));
    }
}
