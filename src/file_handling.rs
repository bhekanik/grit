use crate::config::{GRIT_DIRECTORY, WORKING_DIR};
use crate::utils;
use sha1::{Digest, Sha1};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{BufReader, Read};

pub fn read_file_as_bytes(filepath: &str) -> Option<Vec<u8>> {
    let file = match File::open(&filepath) {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to open file {}: {}", filepath, e);
            return None;
        }
    };

    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();

    // Read file into vector.
    match reader.read_to_end(&mut buffer) {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to read file {}: {}", filepath, e);
            return None;
        }
    };

    // Read.
    Some(buffer)
}

pub fn read_file(filename: &str) -> Option<String> {
    match std::fs::read_to_string(&filename) {
        Ok(contents) => Some(contents),
        Err(e) => {
            println!("Failed to read file {}: {}", filename, e);
            None
        }
    }
}

pub fn read_grit_file(filename: &str) -> Option<String> {
    let file_to_read = format!("{GRIT_DIRECTORY}/{filename}");

    match std::fs::read_to_string(&file_to_read) {
        Ok(contents) => Some(contents),
        Err(e) => {
            println!("Failed to read file {}: {}", file_to_read, e);
            None
        }
    }
}

pub fn read_source_file(filename: &str) -> Option<String> {
    let file_to_read = format!("{WORKING_DIR}/{filename}");

    match std::fs::read_to_string(&file_to_read) {
        Ok(contents) => Some(contents),
        Err(e) => {
            println!("Failed to read file {}: {}", filename, e);
            return None;
        }
    }
}

pub fn hash_file(input_string: &str) -> String {
    let mut hasher = Sha1::new();

    hasher.update(&input_string);

    let result = hasher.finalize();

    let hex_string = hex::encode(&result);
    hex_string
}

pub fn get_object_path_from_oid(oid: &str, create_dir: bool) -> String {
    let sub_directory = &oid[..2].to_string();
    let filename = &oid[2..];
    let sub_directory_path = format!("{GRIT_DIRECTORY}/objects/{sub_directory}");

    if create_dir {
        match fs::create_dir_all(&sub_directory_path) {
            Ok(_) => (),
            Err(e) => {
                println!("{}", e);
                return "".to_string();
            }
        };
    }

    format!("{sub_directory_path}/{filename}")
}

pub fn save_object(oid: &String, object_content: String) -> Option<()> {
    let filepath = get_object_path_from_oid(oid, true);

    let mut object_file = match File::create(&filepath) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create file {}: {}", filepath, e);
            return None;
        }
    };

    let compressed_object_content = utils::compress_object_content(&object_content)?;

    match object_file.write_all(&compressed_object_content) {
        Ok(_) => Some(()),
        Err(e) => {
            println!("Failed to save object: {}", e);
            return None;
        }
    }
}

pub fn object_exists(oid: &str) -> bool {
    let file_path = get_object_path_from_oid(&oid, false);

    match File::open(file_path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn get_all_objects() -> Vec<String> {
    let objects_dir_path = format!("{GRIT_DIRECTORY}/objects");

    let objects = match fs::read_dir(&objects_dir_path) {
        Ok(result) => result,
        Err(e) => {
            println!("Failed to read directory {}: {}", &objects_dir_path, e);
            return vec![];
        }
    };

    objects
        .map(
            |object| match object.expect("Failed to read file").path().to_str() {
                Some(filename) => filename
                    .split("/")
                    .collect::<Vec<&str>>()
                    .join("")
                    .to_string(),
                None => "".to_string(),
            },
        )
        .collect()
}
