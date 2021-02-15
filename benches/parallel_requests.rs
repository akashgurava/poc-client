use std::time::{Duration, Instant};

mod common;

use futures::future::join_all;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use common::*;
use poc_client::client::Client;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

pub async fn parallel_quote_plain(request_count: usize) {
    let start = Instant::now();
    let (stx, mut srx) = mpsc::unbounded_channel::<u8>();
    let (dtx, mut drx) = mpsc::unbounded_channel::<u128>();

    let client = Client::default();
    let requests = (1..=request_count)
        .map(|_| {
            let client = client.clone();
            let s_send = stx.clone();
            let d_send = dtx.clone();
            tokio::spawn(async move {
                let t_start = Instant::now();
                let data = client.chart_quote("913354090", "d1").await;
                match data {
                    Ok(_) => {
                        s_send.send(1).unwrap();
                    }
                    Err(_) => {}
                }
                d_send.send(t_start.elapsed().as_millis()).unwrap();
            })
        })
        .collect::<Vec<_>>();

    drop(stx);
    drop(dtx);

    let mut req_dur = Vec::new();
    let mut success = 0usize;
    while let Some(_) = srx.recv().await {
        success += 1;
    }
    while let Some(dur) = drx.recv().await {
        req_dur.push(dur);
    }
    join_all(requests).await;

    println!(
        "No Limits: Request Count - {} > Success{}({:.2}) Total - {}ms > Mean - {}ms > Median - {}ms > Mode - {}ms > Min - {}ms > Max - {}ms",
        request_count,
        success,
        (success / request_count) * 100,
        start.elapsed().as_millis(),
        mean(&req_dur),
        median(&mut req_dur.clone()),
        mode(&req_dur),
        req_dur.iter().min().unwrap(),
        req_dur.iter().max().unwrap(),
    );
}

fn parallel_quote(c: &mut Criterion) {
    let request_count = 100;

    c.bench_with_input(
        BenchmarkId::new("parallel_quote_plain", request_count),
        &request_count,
        |b, &request_count| {
            let runner = Runtime::new().unwrap();
            // Insert a call to `to_async` to convert the bencher to async mode.
            // The timing loops are the same as with the normal bencher.
            b.to_async(runner)
                .iter(|| parallel_quote_plain(request_count));
        },
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(30));
    targets = parallel_quote
}
criterion_main!(benches);

// #![feature(test)]

// #[tokio::main]
// async fn main() {}
