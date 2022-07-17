mod exec;
mod git;
mod list;
mod repos;
mod storage;

use clap::{Parser, Subcommand};
use exec::exec;
use list::list;
use log::{info, LevelFilter};
use repos::*;
use std::collections::BTreeMap;
use std::io::Write;
use storage::{load, save};

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
	env_logger::builder()
		.format(|buf, record| writeln!(buf, "{}", record.args())) // turn off log decorations https://docs.rs/env_logger/0.9.0/env_logger/#using-a-custom-format
		.filter(None, LevelFilter::Info) // turn on log output
		.init();

	let args = Args::parse();
	let mut repos = load();

	match &args.command {
		Some(Commands::Add { repo_folders }) => {
			add_repos(&mut repos, repo_folders);
			save(repos)
		}
		Some(Commands::Remove { repo_folders }) => {
			repos.remove(repo_folders);
			save(repos)
		}
		Some(Commands::List) => list(&repos),
		Some(Commands::Exec { exec_args }) => {
			exec(exec_args.to_owned(), &repos);
			save(repos)
		}
		Some(Commands::Tag {
			tag_name,
			repo_folders,
			remove,
		}) => {
			if *remove {
				repos.remove_tag(tag_name, repo_folders);
			} else {
				repos.add_tag(tag_name, repo_folders);
			}
			save(repos)
		}
		None => {
			panic!("no command") // this doesn't happen because help shows instead
		}
	}
}

fn add_repos(repos: &mut Repos, repo_folders: &Vec<String>) {
	for repo_folder in repo_folders {
		if repos.exists(&repo_folder) {
			info!("{} already added, ignoring.", repo_folder);
			continue;
		}
		// todo: read all remotes, not just origin https://github.com/timabell/gitopolis/issues/7
		let remote_name = "origin";
		let url = git::read_url(&repo_folder, remote_name);
		let mut remotes: BTreeMap<String, Remote> = BTreeMap::new();
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
		repos.add(repo);
	}
}
