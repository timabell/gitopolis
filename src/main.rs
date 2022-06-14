use clap::{Parser, Subcommand};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use toml;

const STATE_FILENAME: &str = ".gitopolis.toml";

/// gitopolis, a cli tool for managnig multiple git repositories - https://github.com/timabell/gitopolis
#[derive(Parser)]
#[clap(author, version, subcommand_required = true)]
struct Args {
	#[clap(subcommand)]
	command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
	/// add one or more git repos to manage
	Add {
		#[clap(required = true)]
		repo_folders: Vec<String>,
	},
	Remove {
		#[clap(required = true)]
		repo_folders: Vec<String>,
	},
	List,
	Exec {
		exec_args: Vec<String>,
	},
	Tag {
		/// Remove this tag from these repo_folders
		#[clap(short, long)]
		remove: bool,
		#[clap(required = true)]
		tag_name: String,
		#[clap(required = true)]
		repo_folders: Vec<String>,
	},
}

fn main() {
	let args = Args::parse();

	match &args.command {
		Some(Commands::Add { repo_folders }) => add_repos(repo_folders),
		Some(Commands::Remove { repo_folders }) => remove_repos(repo_folders),
		Some(Commands::List) => list(),
		Some(Commands::Exec { exec_args }) => exec(exec_args),
		Some(Commands::Tag {
			tag_name,
			repo_folders,
			remove,
		}) => tag_folders(tag_name, repo_folders, &remove),
		None => {
			println!("nada");
		}
	}
}

fn tag_folders(tag_name: &str, repo_folders: &Vec<String>, remove: &bool) {
	let mut repos = load();
	for repo_folder in repo_folders {
		let mut repo = repos[repo_folder];
		if *remove {
			if let Some(ix) = repo.tags.iter().position(|t| t == tag_name) {
				repo.tags.remove(ix);
			}
		} else {
			repo.tags.push(tag_name.to_string());
		}
	}
	save(&repos);
}

fn exec(exec_args: &Vec<String>) {
	let args_copy: &mut Vec<String> = &mut exec_args.to_owned();
	let args = args_copy.split_off(1);
	let cmd = &args_copy[0]; // only cmd remaining after split_off above
	let repos = load();
	for (_,repo) in repos {
		repo_exec(&repo.path, &cmd, &args);
	}
}

fn repo_exec(path: &str, cmd: &str, args: &Vec<String>) {
	println!("🌲 {}> {} {:?}", path, cmd, args);
	let output = Command::new(cmd)
		.args(args)
		.current_dir(path)
		.output()
		.expect(&format!("Error running exec {}", cmd));

	let stdout = String::from_utf8(output.stdout).expect("Error converting stdout to string");
	println!("{}", stdout);
	println!();
}

/// Run a command and capture the output for use internally
fn repo_capture_exec(path: &str, cmd: &str, args: &Vec<String>) -> String {
	let output = Command::new(cmd)
		.args(args)
		.current_dir(path)
		.output()
		.expect(&format!(
			"Error running external command {} {:?} in folder {}",
			cmd, args, path
		));

	String::from_utf8(output.stdout).expect("Error converting stdout to string")
}

fn list() {
	let repos = load();
	if repos.len() == 0 {
		println!("No repos");
		std::process::exit(2);
	}
	for (path,_) in repos {
		println!("{}", path);
	}
}

type Repos = BTreeMap<String, Repo>;

#[derive(Debug, Deserialize, Serialize)]
struct Repo {
	path: String,
	tags: Vec<String>,
	remotes: Remotes,
}

type Remotes = BTreeMap<String, Remote>;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Remote {
	name: String,
	url: String,
}

/// hacky call to external git command to get url of origin
fn read_url(path: &str, remote_name: &str) -> String {
	repo_capture_exec(
		&path,
		"git",
		&["config".to_string(), format!("remote.{}.url", remote_name)].to_vec(),
	)
	.trim()
	.to_owned()
}

fn add_repos(repo_folders: &Vec<String>) {
	let mut repos = load();
	for repo_folder in repo_folders {
		println!("Adding {} ...", repo_folder);
		if repos.contains_key(repo_folder) {
			println!("{} already added, ignoring.", repo_folder);
			continue;
		}

		// todo: read all remotes, not just origin https://github.com/timabell/gitopolis/issues/7
		let remote_name = "origin";

		let url = read_url(repo_folder, remote_name);

		let mut remotes: Remotes = Remotes::new();
		remotes.insert(
			remote_name.to_owned(),
			Remote {
				name: remote_name.to_owned(),
				url,
			},
		);

		let repo = Repo {
			path: repo_folder.to_owned(),
			tags: Vec::new(),
			remotes,
		};
		repos.insert(repo.path.to_owned(), repo);
	}
	save(&repos); // &* to pass as *immutable* (dereference+reference) https://stackoverflow.com/questions/41366896/how-to-make-a-rust-mutable-reference-immutable/41367094#41367094
	println!("Done.");
}

fn remove_repos(repo_folders: &Vec<String>) {
	let mut repos = load();
	for repo_folder in repo_folders {
		repos.remove(repo_folder);
	}
	save(&repos);
}

fn save(repos: &Repos) {
	let state_toml = toml::to_string(&repos).expect("Failed to generate toml for repo list");
	fs::write(STATE_FILENAME, state_toml).expect(&format!("Failed to write {}", STATE_FILENAME));
}

fn load() -> &mut Repos {
	if !std::path::Path::new(STATE_FILENAME).exists() {
		return Repos::new();
	}
	let state_toml = fs::read_to_string(STATE_FILENAME).expect("Failed to read state file {}");

	let mut repos: Repos = toml::from_str(&state_toml).expect(&format!("Failed to parse {}", STATE_FILENAME));

	// let repos = named_container
	// 	.remove("repos") // [re]move this rather than taking a ref so that ownership moves with it (borrow checker)
	// 	.expect(&format!("Corrupted state file {}", STATE_FILENAME));
	repos
}
