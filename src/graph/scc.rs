use crate::core::currency::CurrencyCode;
use crate::core::party::PartyId;
use crate::graph::payment_graph::PaymentGraph;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// A strongly connected component in the payment graph.
///
/// All parties within an SCC can reach each other through payment chains,
/// meaning multilateral netting is possible within the component.
/// Parties in different SCCs can only settle bilaterally.
#[derive(Debug, Clone)]
pub struct StronglyConnectedComponent {
    pub parties: Vec<PartyId>,
    pub currency: CurrencyCode,
}

impl StronglyConnectedComponent {
    pub fn len(&self) -> usize {
        self.parties.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parties.is_empty()
    }

    /// Returns true if this SCC contains more than one party
    /// (meaning netting opportunities exist).
    pub fn is_nettable(&self) -> bool {
        self.parties.len() > 1
    }
}

/// Find all strongly connected components using Tarjan's algorithm.
///
/// This identifies clusters of parties where multilateral netting
/// is possible. Parties within an SCC all have paths to each other,
/// so circular flows can be compressed.
pub fn find_sccs(
    graph: &PaymentGraph,
    currency: &CurrencyCode,
) -> Vec<StronglyConnectedComponent> {
    let adj = graph.adjacency_list(currency);
    let parties: Vec<PartyId> = {
        let mut p: Vec<_> = graph.parties().iter().cloned().collect();
        p.sort();
        p
    };

    let mut state = TarjanState {
        index_counter: 0,
        stack: Vec::new(),
        on_stack: HashMap::new(),
        indices: HashMap::new(),
        lowlinks: HashMap::new(),
        result: Vec::new(),
    };

    for party in &parties {
        if !state.indices.contains_key(party) {
            strongconnect(party, &adj, &mut state);
        }
    }

    state
        .result
        .into_iter()
        .map(|parties| StronglyConnectedComponent {
            parties,
            currency: currency.clone(),
        })
        .collect()
}

struct TarjanState {
    index_counter: usize,
    stack: Vec<PartyId>,
    on_stack: HashMap<PartyId, bool>,
    indices: HashMap<PartyId, usize>,
    lowlinks: HashMap<PartyId, usize>,
    result: Vec<Vec<PartyId>>,
}

fn strongconnect(
    v: &PartyId,
    adj: &HashMap<PartyId, Vec<(PartyId, Decimal)>>,
    state: &mut TarjanState,
) {
    state.indices.insert(v.clone(), state.index_counter);
    state.lowlinks.insert(v.clone(), state.index_counter);
    state.index_counter += 1;
    state.stack.push(v.clone());
    state.on_stack.insert(v.clone(), true);

    if let Some(neighbors) = adj.get(v) {
        for (w, _) in neighbors {
            if !state.indices.contains_key(w) {
                strongconnect(w, adj, state);
                let low_w = state.lowlinks[w];
                let low_v = state.lowlinks[v];
                state.lowlinks.insert(v.clone(), low_v.min(low_w));
            } else if *state.on_stack.get(w).unwrap_or(&false) {
                let idx_w = state.indices[w];
                let low_v = state.lowlinks[v];
                state.lowlinks.insert(v.clone(), low_v.min(idx_w));
            }
        }
    }

    // If v is a root node, pop the stack and generate an SCC
    if state.lowlinks[v] == state.indices[v] {
        let mut component = Vec::new();
        loop {
            let w = state.stack.pop().unwrap();
            state.on_stack.insert(w.clone(), false);
            component.push(w.clone());
            if w == *v {
                break;
            }
        }
        component.sort();
        state.result.push(component);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::obligation::Obligation;
    use rust_decimal_macros::dec;

    #[test]
    fn test_single_scc() {
        let mut graph = PaymentGraph::new();
        let usd = CurrencyCode::new("USD");

        // A -> B -> C -> A (all connected)
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

        let sccs = find_sccs(&graph, &usd);
        let nettable: Vec<_> = sccs.iter().filter(|s| s.is_nettable()).collect();
        assert_eq!(nettable.len(), 1);
        assert_eq!(nettable[0].len(), 3);
    }

    #[test]
    fn test_disjoint_components() {
        let mut graph = PaymentGraph::new();
        let usd = CurrencyCode::new("USD");

        // Two separate cycles
        graph.add_obligation(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            usd.clone(),
        ));
        graph.add_obligation(Obligation::new(
            PartyId::new("B"),
            PartyId::new("A"),
            dec!(100),
            usd.clone(),
        ));
        graph.add_obligation(Obligation::new(
            PartyId::new("C"),
            PartyId::new("D"),
            dec!(50),
            usd.clone(),
        ));
        graph.add_obligation(Obligation::new(
            PartyId::new("D"),
            PartyId::new("C"),
            dec!(50),
            usd.clone(),
        ));

        let sccs = find_sccs(&graph, &usd);
        let nettable: Vec<_> = sccs.iter().filter(|s| s.is_nettable()).collect();
        assert_eq!(nettable.len(), 2);
    }

    #[test]
    fn test_no_cycles_all_singletons() {
        let mut graph = PaymentGraph::new();
        let usd = CurrencyCode::new("USD");

        // Linear chain: no cycles
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

        let sccs = find_sccs(&graph, &usd);
        let nettable: Vec<_> = sccs.iter().filter(|s| s.is_nettable()).collect();
        assert!(nettable.is_empty());
    }
}
