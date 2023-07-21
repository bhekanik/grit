extern crate chrono;
use chrono::offset::Utc;
use chrono::DateTime;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::prelude::*;
use std::time::SystemTime;

pub fn get_current_time() -> String {
    let system_time = SystemTime::now();
    let datetime: DateTime<Utc> = system_time.into();
    datetime.format("%d/%m/%Y %T").to_string()
}

pub fn compress_object_content(content: &str) -> Option<Vec<u8>> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
    e.write_all(content.as_bytes()).ok()?;
    let compressed_bytes = e.finish();

    match compressed_bytes {
        Ok(bytes) => Some(bytes),
        Err(e) => {
            println!("Failed to get compressed bytes: {}", e);
            return None;
        }
    }
}

pub fn decompress_object_content(content: &[u8]) -> Option<String> {
    let mut d = DeflateDecoder::new(content);
    let mut s = String::new();
    match d.read_to_string(&mut s) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to read uncompressed object content: {}", e);
            return None;
        }
    }

    Some(s)
}
