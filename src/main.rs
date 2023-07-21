use grit::plumbing::{GritCatType, GritObjectType};
use grit::{self, plumbing, WORKING_DIR};
use grit::{add, commit, init, rm, status};
use std::env;
use std::io::{self, BufRead};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: grit <command>");
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "init" => {
            init();
        }
        "status" => {
            status();
        }
        "write-tree" => {
            let oid = match plumbing::write_tree() {
                Some(oid) => oid,
                None => return,
            };
            println!("{oid}");
        }
        "hash-object" => {
            if args.len() < 3 {
                println!("Please provide the content to be processed: grit hash-object [-w] [--stdin] <filename>");
                return;
            }

            let parameters = Vec::from(&args[2..]);

            let mut write = false;
            let mut content = "".to_string();

            if !parameters.contains(&"--stdin".to_string()) && parameters.len() < 2 {
                println!("Please provide either a filename or use --stdin to read the content from stdin: grit hash-object [--stdin] <filename>");
                return;
            } else if !parameters.contains(&"--stdin".to_string()) {
                let filename = match parameters.iter().find(|&x| x != "-w") {
                    Some(x) => x.clone(),
                    None => {
                        println!("Please provide either a filename or use --stdin to read the content from stdin: grit hash-object [--stdin] <filename>");
                        return;
                    }
                };

                let filepath = format!("{WORKING_DIR}/{filename}");

                content = match grit::file_handling::read_file(&filepath) {
                    Some(content) => content,
                    None => {
                        println!("Failed to read file");
                        return;
                    }
                };
            }

            for parameter in parameters {
                match parameter.as_str() {
                    "--stdin" => {
                        let mut buffer = String::new();
                        let stdin = io::stdin();
                        let mut handle = stdin.lock();

                        handle.read_line(&mut buffer).expect("Failed to read line");
                        content = buffer.trim().to_string();
                    }
                    "-w" => write = true,
                    _ => (),
                }
            }

            let oid = match plumbing::hash_object(&content, GritObjectType::Blob, write) {
                Some(oid) => oid,
                None => {
                    println!("Failed to hash object");
                    return;
                }
            };
            println!("{oid}");
        }
        "add" => {
            if args.len() < 3 {
                println!("Please provide the files to add: grit add <filenames>");
                return;
            }

            let filenames = Vec::from(&args[2..]);

            add(filenames);
        }
        "rm" => {
            if args.len() < 3 {
                println!("Please provide the files to remove: grit rm <filenames>");
                return;
            }

            let filenames = Vec::from(&args[2..]);

            rm(filenames);
        }
        "read-tree" => {
            if args.len() < 4 {
                println!(
                    "Please provide a prefix and tree object id: grit read-tree --prefix=docs <object id>"
                );
                return;
            }

            let directory_name = args[2].split("=").collect::<Vec<&str>>()[1];
            let oid = &args[3];

            plumbing::read_tree(directory_name, oid);
        }
        "cat-file" => {
            if args.len() < 4 {
                println!("Please provide the type argument and file to print: grit cat-file -t <filenames>");
                return;
            }

            let oid = &args[3];
            let type_or_pretty = args[2].as_str();
            let cat_type: GritCatType = match type_or_pretty {
                "-t" => GritCatType::Type,
                "-p" => GritCatType::Pretty,
                "-s" => GritCatType::Size,
                _ => GritCatType::Type,
            };

            plumbing::cat_file(oid, cat_type);
        }
        "commit" => {
            if args.len() < 3 {
                println!("Please provide the message: grit commit <message>");
                return;
            }

            let message = &args[2];

            commit(message);
        }
        _ => println!("Unknown command"),
    }
}
