#[macro_use]
extern crate log;

extern crate bitcoin;
extern crate itertools;
extern crate reqwest;
extern crate serde_json;
extern crate simple_logger;

use bitcoin::blockdata::block::{Block, BlockHeader};
use bitcoin::network::encodable::ConsensusDecodable;
use bitcoin::network::serialize::BitcoinHash;
use bitcoin::network::serialize::{deserialize, RawDecoder};
use bitcoin::util::hash::Sha256dHash;
use itertools::enumerate;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;

const HEADER_SIZE: usize = 80;

type HeaderMap = HashMap<String, BlockHeader>;

fn get(resource: &str) -> reqwest::Response {
    let url = format!("http://localhost:8332/rest/{}", resource);
    reqwest::get(&url).unwrap()
}

fn get_bin(resource: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = vec![];
    let mut resp = get(resource);
    resp.copy_to(&mut buf).unwrap();
    buf
}

fn get_headers() -> (HeaderMap, String) {
    let mut headers = HashMap::new();
    let mut blockhash =
        String::from("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"); // genesis
    info!("loading headers from {}", blockhash);
    loop {
        let data = get_bin(&format!("headers/2000/{}.bin", blockhash));
        let num_of_headers = data.len() / HEADER_SIZE;
        let mut decoder = RawDecoder::new(Cursor::new(data));
        for _ in 0..num_of_headers {
            let header: BlockHeader = ConsensusDecodable::consensus_decode(&mut decoder).unwrap();
            blockhash = header.bitcoin_hash().be_hex_string();
            headers.insert(blockhash.to_string(), header);
        }
        if num_of_headers == 1 {
            break;
        }
    }
    info!("loaded {} headers till {}", headers.len(), blockhash);
    (headers, blockhash)
}

fn enumerate_headers(headers: &HeaderMap, bestblockhash: &str) -> Vec<(usize, String)> {
    let null_hash = Sha256dHash::default().be_hex_string();
    let mut hashes = VecDeque::<String>::new();
    let mut blockhash = bestblockhash.to_string();
    while blockhash != null_hash {
        let header: &BlockHeader = headers.get(&blockhash).unwrap();
        hashes.push_front(blockhash);
        blockhash = header.prev_blockhash.be_hex_string();
    }
    enumerate(hashes).collect()
}

fn get_bestblockhash() -> String {
    let data = get("chaininfo.json").text().unwrap();
    let val: Value = serde_json::from_str(&data).unwrap();
    val["bestblockhash"].as_str().unwrap().to_string()
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let (headers, blockhash) = get_headers();
    let hashes = enumerate_headers(&headers, &blockhash);
    info!("loading {} blocks", hashes.len());

    for &(height, ref blockhash) in &hashes {
        let buf = get_bin(&format!("block/{}.bin", &blockhash));
        let block: Block = deserialize(buf.as_slice()).unwrap();
        assert_eq!(&block.bitcoin_hash().be_hex_string(), blockhash);
        if height % 100 == 0 {
            info!("{} @ {}", blockhash, height);
        }
    }
}