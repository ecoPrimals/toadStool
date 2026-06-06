#![allow(clippy::default_trait_access)]
// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC handler throughput benchmarks (parse → dispatch → serialize).

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use criterion::black_box;
use criterion::{Criterion, criterion_group, criterion_main};
use toadstool_server::pure_jsonrpc::JsonRpcHandler;
use toadstool_server::pure_jsonrpc::process_request;
use toadstool_server::tarpc_server::{StandaloneExecutor, WorkloadExecutorDispatch};
use tokio::runtime::Runtime;

fn jsonrpc_handler() -> JsonRpcHandler {
    JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        Arc::<str>::from("bench-1.0.0"),
        None,
        Arc::new(AtomicBool::new(true)),
        None,
    )
}

/// `capabilities.list` — semantic capabilities enumeration.
fn bench_jsonrpc_capabilities_list(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = jsonrpc_handler();
    let body: Vec<u8> = br#"{"jsonrpc":"2.0","method":"capabilities.list","id":1}"#.to_vec();

    let mut group = c.benchmark_group("jsonrpc_capabilities_list");
    group.bench_function("bench_jsonrpc_capabilities_list", |b| {
        b.iter(|| {
            let out = rt
                .block_on(process_request(
                    &handler,
                    black_box(body.as_slice()),
                    Default::default(),
                ))
                .unwrap();
            black_box(out);
        });
    });
    group.finish();
}

/// `health.liveness` — lightweight health probe.
fn bench_jsonrpc_health_liveness(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = jsonrpc_handler();
    let body: Vec<u8> = br#"{"jsonrpc":"2.0","method":"health.liveness","id":2}"#.to_vec();

    let mut group = c.benchmark_group("jsonrpc_health_liveness");
    group.bench_function("bench_jsonrpc_health_liveness", |b| {
        b.iter(|| {
            let out = rt
                .block_on(process_request(
                    &handler,
                    black_box(body.as_slice()),
                    Default::default(),
                ))
                .unwrap();
            black_box(out);
        });
    });
    group.finish();
}

/// `identity.get` — service identity and semantic registry snapshot.
fn bench_jsonrpc_identity_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let handler = jsonrpc_handler();
    let body: Vec<u8> = br#"{"jsonrpc":"2.0","method":"identity.get","id":3}"#.to_vec();

    let mut group = c.benchmark_group("jsonrpc_identity_get");
    group.bench_function("bench_jsonrpc_identity_get", |b| {
        b.iter(|| {
            let out = rt
                .block_on(process_request(
                    &handler,
                    black_box(body.as_slice()),
                    Default::default(),
                ))
                .unwrap();
            black_box(out);
        });
    });
    group.finish();
}

criterion_group!(
    name = jsonrpc_throughput;
    config = Criterion::default();
    targets =
        bench_jsonrpc_capabilities_list,
        bench_jsonrpc_health_liveness,
        bench_jsonrpc_identity_get
);
criterion_main!(jsonrpc_throughput);
