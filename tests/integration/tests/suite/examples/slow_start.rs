// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Praxis Contributors

//! Slow start example: installing the pipeline seeds the baseline.

#[cfg(feature = "slow-start")]
use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "slow-start")]
use praxis_test_utils::{free_port, http_get, start_backend_with_shutdown, start_proxy};

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(feature = "slow-start")]
#[test]
fn slow_start_config_serves_the_baseline_endpoints() {
    let port_a_guard = start_backend_with_shutdown("a");
    let port_a = port_a_guard.port();
    let port_b_guard = start_backend_with_shutdown("b");
    let port_b = port_b_guard.port();
    let proxy_port = free_port();
    let config = super::load_example_config(
        "traffic-management/slow-start.yaml",
        proxy_port,
        HashMap::from([("127.0.0.1:3001", port_a), ("127.0.0.1:3002", port_b)]),
    );
    let load_balancer = config
        .filter_chains
        .iter()
        .flat_map(|chain| chain.filters.iter())
        .find(|entry| entry.filter_type == "load_balancer")
        .expect("the example configures a load balancer");
    let slow_start = load_balancer
        .config
        .get("clusters")
        .and_then(|clusters| clusters.as_sequence())
        .and_then(|clusters| clusters.first())
        .and_then(|cluster| cluster.get("slow_start"))
        .expect("the example must set slow_start; deleting it must fail this test");
    assert_eq!(
        slow_start.get("window_ms").and_then(serde_yaml::Value::as_u64),
        Some(30_000),
        "the example window is 30 seconds"
    );

    let registry = Arc::new(praxis_filter::SlowStartRegistry::new());
    let filter = praxis_filter::LoadBalancerFilter::from_config(&load_balancer.config)
        .expect("the example load balancer parses");
    filter.bind_slow_start(&registry);
    assert!(
        registry.baseline_seeded("backend"),
        "installing the load balancer seeds the listed endpoints at full weight"
    );

    let proxy = start_proxy(&config);

    let mut seen: HashMap<String, u32> = HashMap::new();
    for _ in 0..4 {
        let (status, body) = http_get(proxy.addr(), "/", None);
        assert_eq!(status, 200, "slow start config should proxy the request");
        *seen.entry(body).or_default() += 1;
    }
    assert_eq!(seen.len(), 2, "baseline endpoints stay in rotation");
}
