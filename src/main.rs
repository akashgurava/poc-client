use std::collections::HashMap;
use std::time::Instant;

use futures::future::join_all;
use tokio::sync::mpsc;

use poc_client::client::Client;

pub fn mean(numbers: &[u128]) -> f32 {
    numbers.iter().sum::<u128>() as f32 / numbers.len() as f32
}

pub fn median(numbers: &mut [u128]) -> u128 {
    numbers.sort();
    let mid = numbers.len() / 2;
    numbers[mid]
}

pub fn mode(numbers: &[u128]) -> u128 {
    let mut occurrences = HashMap::new();

    for &value in numbers {
        *occurrences.entry(value).or_insert(0) += 1;
    }

    occurrences
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(val, _)| val)
        .expect("Cannot compute the mode of zero numbers")
}

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
                    Ok(data) => {
                        if data.len() == 1 {
                            s_send.send(1).unwrap();
                        }
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
        "No Limits: Request Count - {} > Success - {}({:.2}%) Total - {}ms > Mean - {}ms > Median - {}ms > Mode - {}ms > Min - {}ms > Max - {}ms",
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

#[tokio::main]
async fn main() {
    parallel_quote_plain(500).await;
}
