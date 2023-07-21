use crate::config::{GRIT_DIRECTORY, WORKING_DIR};
use crate::file_handling;
use crate::utils;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::prelude::*;
use std::path;
use walkdir::WalkDir;

pub enum GritObjectType {
    Blob,
    Tree,
    Commit,
}

pub enum GritCatType {
    Type,
    Pretty,
    Size,
}

pub enum GritMode {
    Normal,
    Executable,
    Symlink,
}

struct GritAuthor {
    pub name: String,
}

pub fn hash_object(content: &str, object_type: GritObjectType, write: bool) -> Option<String> {
    let bytes_size = content.len();

    let header = match object_type {
        GritObjectType::Blob => format!("blob {}\0", bytes_size),
        GritObjectType::Tree => format!("tree {}\0", bytes_size),
        GritObjectType::Commit => format!("commit {}\0", bytes_size),
    };

    let store = format!("{header}{content}");

    let oid = file_handling::hash_file(&store);

    if write {
        file_handling::save_object(&oid, store)?;
    }

    Some(oid)
}

pub fn index_is_empty() -> Option<bool> {
    let index_file_content = file_handling::read_grit_file("index")?;
    Some(index_file_content.is_empty())
}

pub fn get_index_entries() -> Option<(
    HashMap<String, String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
)> {
    let index_file_content = file_handling::read_grit_file("index")?;
    let mut index_objects_map = HashMap::new();
    let mut paths = vec![];
    let mut oids = vec![];

    if index_file_content.is_empty() {
        return Some((index_objects_map, paths, vec![], oids));
    }

    let index_entries = index_file_content
        .trim()
        .split("\n")
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<String>>();

    index_entries.iter().for_each(|entry| {
        let entry = entry.split(" ").collect::<Vec<&str>>();
        index_objects_map.insert(entry[3].to_string(), entry[2].to_string());
        paths.push(entry[3].to_string());
        oids.push(entry[2].to_string());
    });

    Some((index_objects_map, paths, index_entries, oids))
}

pub fn get_head_tree_entries(
    commit_oid: &str,
) -> Option<(
    HashMap<String, String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
)> {
    let mut head_objects_map = HashMap::new();
    let mut paths = vec![];
    let mut oids = vec![];

    if commit_oid == "" {
        return Some((head_objects_map, paths, vec![], oids));
    }

    let commit_content = generate_cat_content(&commit_oid, GritCatType::Pretty)?;

    let tree_line = commit_content
        .split("\n")
        .find(|line| line.starts_with("tree"))?;

    let tree_oid = tree_line.split(" ").collect::<Vec<&str>>()[1];

    let tree = generate_cat_content(tree_oid, GritCatType::Pretty)?;

    let tree_entries = tree
        .trim()
        .split("\n")
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<String>>();

    tree_entries.iter().for_each(|entry| {
        let entry = entry.split(" ").collect::<Vec<&str>>();
        head_objects_map.insert(entry[3].to_string(), entry[2].to_string());
        paths.push(entry[3].to_string());
        oids.push(entry[2].to_string());
    });

    Some((head_objects_map, paths, tree_entries, oids))
}

pub fn exists_in_index(filename: &str) -> Option<bool> {
    let (index_objects_map, _paths, _index_entries, _oids) = get_index_entries()?;

    if index_objects_map.is_empty() {
        return Some(false);
    }

    Some(match index_objects_map.get(filename) {
        Some(_) => true,
        None => false,
    })
}

pub fn update_in_index(filename: &str, entry: &str) -> Option<()> {
    let (index_objects_map, _paths, index_entries, _oids) = get_index_entries()?;

    if index_objects_map.is_empty() {
        return None;
    }

    let updated_lines = index_entries
        .iter()
        .map(|path| {
            if path.split(" ").collect::<Vec<&str>>()[3] == filename {
                entry.trim().to_string()
            } else {
                path.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    let index_file_path = format!("{GRIT_DIRECTORY}/index");

    let mut index_file = match File::create(index_file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create index: {}", e);
            return None;
        }
    };

    match index_file.write_all(updated_lines.as_bytes()) {
        Ok(_) => Some(()),
        Err(e) => {
            println!("Failed to update index: {}", e);
            return None;
        }
    }
}

pub fn remove_from_index(filename: &str) -> Option<()> {
    let (index_objects_map, _paths, index_entries, _oids) = get_index_entries()?;

    if index_objects_map.is_empty() {
        return None;
    }

    let updated_lines = index_entries
        .iter()
        .filter(|path| path.split(" ").collect::<Vec<&str>>()[3] != filename)
        .map(|path| path.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    let index_file_path = format!("{GRIT_DIRECTORY}/index");

    let mut index_file = match File::create(index_file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create index: {}", e);
            return None;
        }
    };

    match index_file.write_all(updated_lines.as_bytes()) {
        Ok(_) => Some(()),
        Err(e) => {
            println!("Failed to update index: {}", e);
            return None;
        }
    }
}

pub fn append_to_index(entry: &str) -> Option<()> {
    let index_file_path = format!("{GRIT_DIRECTORY}/index");

    let mut index_file = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(index_file_path)
    {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create index: {}", e);
            return None;
        }
    };

    let content_to_write = if !index_is_empty()? {
        format!("\n{}", &entry)
    } else {
        entry.to_string()
    };

    match index_file.write_all(content_to_write.as_bytes()) {
        Ok(_) => Some(()),
        Err(e) => {
            println!("Failed to update index: {}", e);
            return None;
        }
    }
}

pub fn update_index(
    mode: GritMode,
    object_type: GritObjectType,
    oid: &str,
    filename: &str,
) -> Option<()> {
    let mode_value = match mode {
        GritMode::Normal => 100644,
        GritMode::Executable => 100755,
        GritMode::Symlink => 120000,
    };

    let object_type = match object_type {
        GritObjectType::Blob => "blob",
        GritObjectType::Tree => "tree",
        GritObjectType::Commit => "commit",
    };

    let entry = format!("{mode_value} {object_type} {oid} {filename}");

    let exists = exists_in_index(filename)?;

    if exists {
        update_in_index(&filename, &entry)
    } else {
        append_to_index(&entry)
    }
}

pub fn clear_index() -> Option<()> {
    let index_file_path = format!("{GRIT_DIRECTORY}/index");

    let mut file = match File::create(&index_file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to open index file: {}", e);
            return None;
        }
    };

    match file.write_all("".as_bytes()) {
        Ok(_) => Some(()),
        Err(e) => {
            println!("Failed to update index file: {}", e);
            return None;
        }
    }
}

pub fn write_tree() -> Option<String> {
    let index_file_path = "index";

    let content = file_handling::read_grit_file(&index_file_path)?;

    if content.is_empty() {
        println!("Nothing to write");
        return None;
    }

    let tree_oid = hash_object(&content, GritObjectType::Tree, true)?;

    Some(tree_oid)
}

pub fn read_tree(directory_name: &str, oid: &str) {
    update_index(GritMode::Normal, GritObjectType::Tree, oid, directory_name);
}

pub fn unhash_object(oid: &str) -> Option<String> {
    let sub_directory = &oid[..2].to_string();
    let filename = &oid[2..];
    let object_path = format!("{GRIT_DIRECTORY}/objects/{sub_directory}");
    let read_dirs = match fs::read_dir(&object_path) {
        Ok(result) => result,
        Err(e) => {
            println!("Failed to read directory {}: {}", &object_path, e);
            return None;
        }
    };

    let paths = read_dirs
        .map(
            |object| match object.expect("Failed to read file").file_name().to_str() {
                Some(path) => path.to_string(),
                None => "".to_string(),
            },
        )
        .collect::<Vec<String>>();

    let full_oid = format!(
        "{}{}",
        sub_directory,
        paths.iter().find(|path| path.starts_with(&filename))?
    );

    let file_path = file_handling::get_object_path_from_oid(&full_oid, false);

    let compressed_contents = file_handling::read_file_as_bytes(&file_path)?;

    let content = utils::decompress_object_content(&compressed_contents)?;

    Some(content)
}

fn get_object_contents(content: &str) -> String {
    content.split("\0").collect::<Vec<&str>>()[1].to_string()
}

fn get_object_size(content: &str) -> String {
    content.split("\0").collect::<Vec<&str>>()[0]
        .split(" ")
        .collect::<Vec<&str>>()[1]
        .to_string()
}

fn get_object_type(content: &str) -> String {
    content.split("\0").collect::<Vec<&str>>()[0]
        .split(" ")
        .collect::<Vec<&str>>()[0]
        .to_string()
}

pub fn generate_cat_content(oid: &str, cat_type: GritCatType) -> Option<String> {
    let content = unhash_object(oid)?;

    match cat_type {
        GritCatType::Type => {
            let report = get_object_type(&content);
            Some(report)
        }
        GritCatType::Pretty => {
            let report = get_object_contents(&content);
            Some(report)
        }
        GritCatType::Size => {
            let report = get_object_size(&content);
            Some(report)
        }
    }
}

pub fn cat_file(oid: &str, cat_type: GritCatType) -> Option<()> {
    let report = generate_cat_content(oid, cat_type)?;

    println!("{report}");

    Some(())
}

pub fn get_head_ref() -> Option<String> {
    let head_path = "HEAD";

    let head = file_handling::read_grit_file(&head_path)?;

    let head_ref = head.split(" ").collect::<Vec<&str>>()[1].to_string();

    Some(head_ref)
}

pub fn get_current_branch() -> Option<String> {
    let head_ref = get_head_ref()?;

    let branch = head_ref.replace("refs/heads/", "");

    Some(branch)
}

pub fn get_head_oid() -> Option<String> {
    let head_ref = get_head_ref()?;

    file_handling::read_grit_file(&head_ref)
}

pub fn update_head(commit_oid: &str) -> Option<()> {
    let head_ref = get_head_ref()?;

    let head_ref_path = format!("{GRIT_DIRECTORY}/{head_ref}");

    let mut head_ref_file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&head_ref_path)
    {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to open head ref file: {}", e);
            return None;
        }
    };

    match head_ref_file.write_all(commit_oid.as_bytes()) {
        Ok(_) => Some(()),
        Err(e) => {
            println!("Failed to update head ref file: {}", e);
            return None;
        }
    }
}

pub fn commit_tree(
    tree_oid: &str,
    message: &str,
    parent_commit_oid: Option<&str>,
) -> Option<String> {
    let author = GritAuthor {
        name: "Bhekani Khumalo".to_string(),
    };

    let mut formatted_commit = format!("tree {tree_oid}")
        + format!("\nauthor {}", &author.name).as_str()
        + format!("\ncommitter {}", &author.name).as_str()
        + format!("\ndate {}", utils::get_current_time()).as_str()
        + format!("\n\n{}", message).as_str();

    match parent_commit_oid {
        Some(parent_oid) => {
            formatted_commit = format!("parent {parent_oid}\n{formatted_commit}");
        }
        None => (),
    };

    let oid = hash_object(&formatted_commit, GritObjectType::Commit, true)?;

    Some(oid)
}

pub fn get_working_tree_entries() -> Option<Vec<String>> {
    let working_dir_path = path::Path::new(WORKING_DIR);
    let read_dirs = match fs::read_dir(&working_dir_path) {
        Ok(result) => result,
        Err(e) => {
            println!("Failed to read directory {:?}: {}", &working_dir_path, e);
            return None;
        }
    };

    let paths = read_dirs
        .map(|object| match object {
            Ok(item) => item.path().display().to_string(),
            Err(e) => {
                println!("Failed to read item: {}", e);
                return "".to_string();
            }
        })
        .collect::<Vec<String>>();

    Some(paths)
}

pub fn get_working_tree_entries_recursively() -> Option<Vec<String>> {
    let source_dir_path = path::Path::new(WORKING_DIR);
    let items = WalkDir::new(source_dir_path);

    let paths: Vec<String> = items
        .into_iter()
        .filter_map(Result::ok) // Unwrap the entry, ignoring any errors
        .filter(|entry| entry.file_type().is_file()) // Filter out directories, leaving only file paths
        .map(|entry| {
            entry
                .into_path()
                .display()
                .to_string()
                .replace("source/", "")
        }) // Convert entries to paths
        .collect();

    Some(paths)
}
