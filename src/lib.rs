mod config;
pub mod file_handling;
pub mod plumbing;
mod utils;

pub use crate::config::{GRIT_DIRECTORY, WORKING_DIR};
use colored::*;
use plumbing::{GritMode, GritObjectType};
use std::fs::{self, File};
use std::io::prelude::*;

pub fn init() -> Option<()> {
    let sub_directories = vec!["refs/heads", "objects", "objects/info", "objects/pack"];
    let grit_files = vec!["HEAD", "index", "refs/heads/main"];

    // Create the grit directory
    match fs::create_dir_all(GRIT_DIRECTORY.clone()) {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
            return None;
        }
    }

    for dir in sub_directories {
        let dir_name = format!("{GRIT_DIRECTORY}/{dir}");
        match fs::create_dir_all(&dir_name) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to create directory {}: {}", &dir_name, e);
                return None;
            }
        }
    }

    for grit_file in grit_files {
        let file_name = format!("{GRIT_DIRECTORY}/{grit_file}");
        let mut file = match File::create(file_name) {
            Ok(file) => file,
            Err(e) => {
                println!("{}", e);
                return None;
            }
        };

        if grit_file == "HEAD" {
            match file.write_all("ref: refs/heads/main".as_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    println!("{}", e);
                    return None;
                }
            };
        }
    }

    println!("Initialized empty grit repository");

    Some(())
}

pub fn add(mut filenames: Vec<String>) -> Option<()> {
    let (_index_tree_map, index_tree_paths, _index_tree_entries, index_tree_entry_oids) =
        plumbing::get_index_entries()?;

    if filenames[0] == "." {
        filenames = index_tree_paths;

        let mut working_tree_entries = plumbing::get_working_tree_entries_recursively()?;

        filenames.append(&mut working_tree_entries);
        filenames.sort();
        filenames.dedup();
    }

    let mut oids: Vec<String> = Vec::new();

    let mut added_files = String::new();

    for filename in &filenames {
        let lines = match file_handling::read_source_file(filename) {
            Some(content) => content,
            None => {
                println!("here: {filename}");
                plumbing::remove_from_index(&filename);
                continue;
            }
        };

        let oid: String = plumbing::hash_object(&lines, GritObjectType::Blob, false)?;

        if match index_tree_entry_oids
            .iter()
            .find(|index_tree_entry_oid| **index_tree_entry_oid == oid)
        {
            Some(_index_tree_object_oid) => true,
            None => false,
        } {
            continue;
        }

        let oid: String = plumbing::hash_object(&lines, GritObjectType::Blob, true)?;

        plumbing::update_index(GritMode::Normal, GritObjectType::Blob, &oid, filename)?;

        let oid_and_file = format!("\n{} {}", oid[..7].to_string(), filename);
        added_files.push_str(oid_and_file.as_str());
        oids.push(oid.to_string());
    }

    let report = format!(
        "Added {} file{}:{}",
        oids.len(),
        if oids.len() == 1 { "" } else { "s" },
        added_files
    );

    println!("{}", report);

    Some(())
}

pub fn rm(mut filenames: Vec<String>) -> Option<()> {
    if filenames[0] == "." {
        filenames.pop();
        let (_index_tree_map, mut index_tree_paths, _index_tree_entries, _index_tree_oids) =
            plumbing::get_index_entries()?;

        filenames.append(&mut index_tree_paths);
    }

    let mut removed_files = String::new();

    for filename in &filenames {
        plumbing::remove_from_index(filename)?;

        let oid_and_file = format!("\n{}", filename);
        removed_files.push_str(oid_and_file.as_str());
    }

    let report = format!(
        "Removed {} file{}:{}",
        filenames.len(),
        if filenames.len() == 1 { "" } else { "s" },
        removed_files
    );

    println!("{}", report);

    Some(())
}

pub fn commit(message: &str) -> Option<()> {
    println!("Committing changes...");

    let tree_oid = plumbing::write_tree()?;

    let head_oid = plumbing::get_head_oid()?;
    let branch = plumbing::get_current_branch()?;

    let mut parent_oid = None;

    if !head_oid.is_empty() {
        parent_oid = Some(head_oid.as_str());
    }

    let commit_oid = plumbing::commit_tree(&tree_oid, message, parent_oid)?;

    plumbing::update_head(&commit_oid);

    let report = format!("[{branch} {}] {}", &commit_oid.to_string()[..7], message,);

    println!("{report}");

    Some(())
}

pub fn status() -> Option<()> {
    let branch = plumbing::get_current_branch()?;

    let mut report_header = format!("On branch {branch}");
    let mut staging_report = format!("");
    let mut working_tree_report = format!("");

    let (index_tree_map, index_tree_paths, _index_tree_entries, _index_tree_oids) =
        plumbing::get_index_entries()?;

    let working_tree_paths = plumbing::get_working_tree_entries_recursively()?;

    let mut to_be_staged: Vec<String> = vec![];
    let mut untracked: Vec<String> = vec![];
    let mut to_be_committed: Vec<String> = vec![];
    let mut to_be_removed: Vec<String> = vec![];

    // Get files to ve staged
    working_tree_paths.iter().for_each(|working_tree_path| {
        match index_tree_map.get(working_tree_path) {
            Some(index_tree_object_oid) => {
                let contents = match file_handling::read_source_file(working_tree_path) {
                    Some(lines) => lines,
                    None => "".to_string(),
                };
                let working_tree_object_oid =
                    match plumbing::hash_object(&contents, GritObjectType::Blob, false) {
                        Some(oid) => oid,
                        None => "".to_string(),
                    };

                if working_tree_object_oid != *index_tree_object_oid {
                    to_be_staged.push(format!("modified:    {working_tree_path}"));
                }
            }
            None => {
                untracked.push(working_tree_path.to_string());
            }
        }
    });

    // Get files to be removed
    index_tree_paths.iter().for_each(|index_tree_path| {
        match working_tree_paths
            .iter()
            .find(|working_tree_path| working_tree_path == &index_tree_path)
        {
            Some(_index_tree_object_oid) => (),
            None => {
                to_be_removed.push(format!("deleted:    {index_tree_path}"));
            }
        }
    });

    let head_commit_oid = plumbing::get_head_oid()?;
    let (head_tree_map, _paths, _head_tree_entries, _oids) =
        plumbing::get_head_tree_entries(&head_commit_oid)?;

    for (index_tree_entry, index_tree_entry_oid) in index_tree_map.iter() {
        match head_tree_map.get(index_tree_entry) {
            Some(head_object_oid) => {
                if *head_object_oid != *index_tree_entry_oid {
                    to_be_committed.push(format!("modified:   {index_tree_entry}"));
                }
            }
            None => {
                to_be_committed.push(format!("new file:    {index_tree_entry}"));
            }
        }
    }

    for (head_tree_entry, _head_tree_entry_oid) in head_tree_map.iter() {
        match index_tree_map.get(head_tree_entry) {
            Some(_index_object_oid) => (),
            None => {
                to_be_committed.push(format!("deleted:    {head_tree_entry}"));
            }
        }
    }

    if head_commit_oid.is_empty() {
        report_header = format!("{report_header}\n\nNo commits yet");
    }

    if !to_be_committed.is_empty() {
        staging_report = "\n\nChanges to be committed:".to_string()
            // + "\n  (use `grit rm --cached <file>...` to unstage)"
            + "\n  (use `grit restore --staged <file>...` to unstage)"
            + format!("\n\t{}", &to_be_committed.join("\n\t").green()).as_str();
    }

    if !to_be_staged.is_empty() {
        working_tree_report = format!("\n\nChanges not staged for commit:")
            + "\n  (use `grit add <file>...` to include in what will be committed)"
            + "\n  (use `grit restore <file>...` to discard changes in working directory)"
            + format!("\n\t{}", &to_be_staged.join("\n\t").red()).as_str();
        // + "\n\nnothing added to commit but untracked files present (use `grit add` to track)";
        if !to_be_removed.is_empty() {
            working_tree_report = format!(
                "{working_tree_report}\n\t{}",
                &to_be_removed.join("\n\t").red()
            );
            // + "\n\nnothing added to commit but untracked files present (use `grit add` to track)";
        }
    } else if !to_be_removed.is_empty() {
        working_tree_report = format!("\n\nChanges not staged for commit:")
            + "\n  (use `grit add <file>...` to include in what will be committed)"
            + "\n  (use `grit restore <file>...` to discard changes in working directory)"
            + format!("\n\t{}", &to_be_removed.join("\n\t").red()).as_str();
    }

    if !untracked.is_empty() {
        working_tree_report = format!("{working_tree_report}\n\nUntracked files:")
            + "\n  (use `grit add <file>...` to include in what will be committed)"
            + format!("\n\t{}", &untracked.join("\n\t").red()).as_str()
        // + "\n\nnothing added to commit but untracked files present (use `grit add` to track)";
    }

    println!("{report_header}{staging_report}{working_tree_report}");
    return None;
}
