use criterion::{black_box, criterion_group, criterion_main, Criterion};
use clearing_engine::core::currency::CurrencyCode;
use clearing_engine::optimization::netting::NettingEngine;
use clearing_engine::simulation::stress_test::{generate_random_network, NetworkConfig};

fn bench_netting_10_parties(c: &mut Criterion) {
    let config = NetworkConfig {
        party_count: 10,
        avg_obligations_per_party: 5,
        ..Default::default()
    };
    let set = generate_random_network(&config);

    c.bench_function("netting_10_parties", |b| {
        b.iter(|| NettingEngine::multilateral_net(black_box(&set)))
    });
}

fn bench_netting_100_parties(c: &mut Criterion) {
    let config = NetworkConfig {
        party_count: 100,
        avg_obligations_per_party: 10,
        ..Default::default()
    };
    let set = generate_random_network(&config);

    c.bench_function("netting_100_parties", |b| {
        b.iter(|| NettingEngine::multilateral_net(black_box(&set)))
    });
}

fn bench_netting_1000_parties(c: &mut Criterion) {
    let config = NetworkConfig {
        party_count: 1000,
        avg_obligations_per_party: 10,
        ..Default::default()
    };
    let set = generate_random_network(&config);

    c.bench_function("netting_1000_parties", |b| {
        b.iter(|| NettingEngine::multilateral_net(black_box(&set)))
    });
}

criterion_group!(
    benches,
    bench_netting_10_parties,
    bench_netting_100_parties,
    bench_netting_1000_parties
);
criterion_main!(benches);
