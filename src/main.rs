use buf_list::BufList;
use bytes::Bytes;
use futures::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::{env, mem, sync::Arc};
use tokio::sync::mpsc;

async fn get_it(correct: Arc<[u8]>, client: Client, url: String) {
    let response = client.get(&url).send().await.expect("GET failed");
    let mut stream = response.bytes_stream();
    let mut n = 0;
    let (tx, rx) = mpsc::channel(8);
    let hasher_task = tokio::spawn(hash_it(rx));
    while let Some(item) = stream.next().await {
        let chunk = item.expect("error on item");
        if !chunk.is_empty() {
            if chunk != correct[n..][..chunk.len()] {
                println!(
                    "[get-it] incorrect data at offset {n:#x} (addr={:?})",
                    chunk.as_ref().as_ptr()
                );
                for i in 0..chunk.len() {
                    println!(
                        "offset={:#10x} expected={:02x} got={:02x}",
                        n + i,
                        correct[n + i],
                        chunk[i]
                    );
                }
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
    let mut args = env::args();
    _ = args.next(); // skip argv[0]
    let url = args.next().expect("need url as arg 1");
    let path = args.next().expect("need path to correct file as arg 2");
    let data: Arc<[u8]> = std::fs::read(&path)
        .expect("failed to read correct file")
        .into();

    let balloon_size = args
        .next()
        .expect("need size of balloon as arg 3")
        .parse::<usize>()
        .expect("failed to parse balloon size arg");

    let v = vec![1u8; balloon_size];
    println!("big v = {}", v[balloon_size - 1]);

    let client = Client::new();

    loop {
        println!("big v = {}", v[balloon_size - 1]);
        get_it(Arc::clone(&data), client.clone(), url.clone()).await;
    }
}
