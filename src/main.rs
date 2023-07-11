use buf_list::BufList;
use bytes::Bytes;
use futures::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::mem;
use tokio::sync::mpsc;

async fn get_it(client: Client, url: String) {
    let response = client.get(&url).send().await.expect("GET failed");
    let mut stream = response.bytes_stream();
    let mut n = 0;
    let (tx, rx) = mpsc::channel(8);
    let hasher_task = tokio::spawn(hash_it(rx));
    while let Some(item) = stream.next().await {
        let chunk = item.expect("error on item");
        if !chunk.is_empty() {
            if chunk.iter().all(|&x| x == 0) {
                println!("[get-it] all zero chunk: offset {n}");
            }
            n += chunk.len();
            tx.send(chunk).await.unwrap();
        }
    }
    println!("success ({n} bytes downloaded)");
    mem::drop(tx);
    hasher_task.await.unwrap();
}

async fn hash_it(mut rx: mpsc::Receiver<Bytes>) {
    let mut accum = BufList::new();
    let mut n = 0;
    while let Some(chunk) = rx.recv().await {
        if chunk.iter().all(|&x| x == 0) {
            println!("[hash_it] rx all zero chunk: offset {n}");
        }
        n += chunk.len();
        accum.push_chunk(chunk);
    }
    println!("rx done: hashing {n} bytes");
    let mut hasher = Sha256::default();
    for chunk in accum.iter() {
        if chunk.iter().all(|&x| x == 0) {
            println!("[hash_it] hashing all zero chunk: offset {n}");
        }
        hasher.update(chunk);
    }
    println!("hashing done: {:x}", hasher.finalize());
}

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("need url as first arg");
    let client = Client::new();

    loop {
        get_it(client.clone(), url.clone()).await;
    }
}
