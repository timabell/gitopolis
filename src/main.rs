use clap::{CommandFactory, Parser, Subcommand};
use gitopolis::exec::exec;
use gitopolis::git::GitImpl;
use gitopolis::gitopolis::{Gitopolis, GitopolisError};
use gitopolis::repos::Repo;
use gitopolis::storage::StorageImpl;
use gitopolis::tag_filter::TagFilter;
use log::LevelFilter;
use std::collections::BTreeMap;
use std::io::Write;

/// A CLI tool for managing multiple git repositories
/// License: A-GPL v3.0
/// Repo: https://github.com/timabell/gitopolis
#[derive(Parser)]
#[clap(author, version, subcommand_required = true, verbatim_doc_comment)]
struct Args {
	#[clap(subcommand)]
	command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
	/// Add one or more git repos to manage.
	Add {
		#[clap(required = true)]
		repo_folders: Vec<String>,
		/// Tags to apply to all the added repos. Comma-separated and/or repeated (e.g. "--tag foo,bar --tag baz").
		#[arg(short, long)]
		tag: Vec<String>,
	},
	/// Remove one or more git repos from gitopolis's list. Leaves actual repo on filesystem alone.
	Remove {
		#[clap(required = true)]
		repo_folders: Vec<String>,
	},
	/// Show list of repos gitopolis knows about. Just names by default, "--long" for human-readable tags and remotes, "--tab" for machine-readable tab separated tags and remotes.
	List {
		#[command(flatten)]
		filter: FilterArgs,
		/// Multi-line human-readable output showing tags and remotes of each repo (like `show`)
		#[clap(short, long, conflicts_with = "tab")]
		long: bool,
		/// One line per repo: path, comma-separated tags, and comma-separated name=url remotes, separated by tabs. For scripts.
		#[clap(long)]
		tab: bool,
	},
	/// Run any shell command. E.g. `gitopolis exec -- git pull`. Double-dash separator indicates end of gitopolis's arguments and prevents arguments to your commands being interpreted by gitopolis.
	Exec {
		#[command(flatten)]
		filter: FilterArgs,
		#[arg(long)]
		oneline: bool,
		/// Don't output repo name when no stdout/stderr output produced
		#[arg(long)]
		skip_blank: bool,
		/// Kill the command if it runs for longer than this many seconds in any one repo, and move on to the next repo. 0 means no limit.
		#[arg(long, default_value_t = 300, value_name = "SECONDS")]
		timeout: u64,
		exec_args: Vec<String>,
	},
	/// Add/remove repo tags. Use tags to organise repos and allow running commands against subsets of the repo list. Supports comma-separated tag lists (e.g., "tag1,tag2,tag3").
	Tag {
		/// Remove this tag from these repo_folders. If no repos specified, removes from ALL repos.
		#[clap(short, long)]
		remove: bool,
		#[clap(required = true)]
		tag: String,
		repo_folders: Vec<String>,
	},
	/// List known tags. Use "long" to list repos per tag.
	Tags {
		#[clap(short, long)]
		long: bool,
	},
	/// Clone repository from URL and add to gitopolis, or clone all configured repos from .gitopolis.toml.
	/// This command behaves in two very different ways depending on whether a remote url was provided:
	/// If URL is provided: clones from that URL, extracts repo name, adds to gitopolis (optionally with tags).
	/// If URL is omitted: clones all repos from .gitopolis.toml (filtered by --tag if specified) skipping existing folders. (Useful for setting up new machines/developers from an existing team configuration)
	Clone {
		/// Optional git URL to clone from (e.g., git@github.com:user/repo.git or https://github.com/user/repo).
		/// If omitted, clones repos defined in .gitopolis.toml configuration
		url: Option<String>,
		/// Optional addition to URL - target directory name to clone this url into (like git clone). If omitted, extracts name from URL
		target_dir: Option<String>,
		/// When cloning from URL, all specified tags are applied to the new repo.
		/// When cloning without URL from existing config,
		/// filters repos to clone from configuration by tags; comma-separated tags use AND logic (e.g., "foo,bar" = foo AND bar),
		/// multiple --tag flags use OR logic (e.g., "--tag foo,bar --tag baz" = (foo AND bar) OR baz).
		#[arg(short, long)]
		tag: Vec<String>,
		/// When cloning from existing config, don't clone these repos (comma-separated and/or repeated)
		#[arg(long)]
		exclude: Vec<String>,
		/// When cloning from existing config, don't clone repos with any of these tags (comma-separated and/or repeated)
		#[arg(long)]
		exclude_tag: Vec<String>,
	},
	/// Sync remotes between git repositories and .gitopolis.toml configuration
	Sync {
		/// Update .gitopolis.toml from remotes in git repositories
		#[arg(long, conflicts_with = "write_remotes")]
		read_remotes: bool,
		/// Update git repositories with remotes from .gitopolis.toml (adds missing remotes and updates URLs, but will not remove any remotes)
		#[arg(long, conflicts_with = "read_remotes")]
		write_remotes: bool,
		/// Show what would be changed without making any modifications
		#[arg(long)]
		dry_run: bool,
		#[command(flatten)]
		filter: FilterArgs,
	},
	/// Show detailed information about a repository including tags and remotes
	Show {
		#[clap(required = true)]
		repo_folder: String,
	},
	/// Move a repository to a new location, updating gitopolis configuration
	Move {
		#[clap(subcommand)]
		entity: MoveEntity,
	},
	/// Rename a tag or other entity
	Rename {
		#[clap(subcommand)]
		entity: RenameEntity,
	},
	/// Generate man pages. Prints the main man page to stdout, or writes pages for gitopolis and all
	/// its subcommands (gitopolis.1, gitopolis-exec.1 etc.) to OUT_DIR if given.
	/// Mostly for packagers, e.g. `gitopolis man /usr/share/man/man1/`
	#[command(hide = true)]
	Man { out_dir: Option<std::path::PathBuf> },
}

/// Options for choosing which repos a command applies to
#[derive(clap::Args)]
struct FilterArgs {
	/// Filter by tags. Comma-separated tags use AND logic (e.g., "foo,bar" = foo AND bar).
	/// Multiple --tag flags use OR logic (e.g., "--tag foo,bar --tag baz" = (foo AND bar) OR baz).
	#[arg(short, long)]
	tag: Vec<String>,
	/// Exclude these repos (comma-separated and/or repeated, e.g. "--exclude repo1,repo2 --exclude repo3")
	#[arg(long)]
	exclude: Vec<String>,
	/// Exclude repos that have ANY of these tags (comma-separated and/or repeated, e.g. "--exclude-tag archived,slow")
	#[arg(long)]
	exclude_tag: Vec<String>,
}

impl FilterArgs {
	fn to_filter(&self) -> TagFilter {
		TagFilter::from_cli_args(&self.tag).with_exclusions(&self.exclude, &self.exclude_tag)
	}
}

#[derive(Subcommand)]
enum MoveEntity {
	/// Move a repository to a new location. Works like unix `mv`:
	/// if new_path is an existing folder or ends with a slash the repo is moved into it (keeping its name),
	/// otherwise the repo is moved/renamed to new_path.
	#[clap(verbatim_doc_comment)]
	Repo {
		/// Current path of the repository
		old_path: String,
		/// New path for the repository, or a folder to move it into (existing, or ending with a slash)
		new_path: String,
	},
}

#[derive(Subcommand)]
enum RenameEntity {
	/// Rename a tag across all repos. Use --merge if some repos already have the new tag.
	Tag {
		/// Current tag name
		old_name: String,
		/// New tag name
		new_name: String,
		/// Allow renaming even if some repos already have the new tag
		#[clap(long)]
		merge: bool,
	},
}

fn main() {
	env_logger::builder()
		.format(|buf, record| writeln!(buf, "{}", record.args())) // turn off log decorations https://docs.rs/env_logger/0.9.0/env_logger/#using-a-custom-format
		.filter(None, LevelFilter::Info) // turn on log output
		.init();

	match &Args::parse_from(wild::args()).command {
		Some(Commands::Add { repo_folders, tag }) => add(repo_folders.to_owned(), tag),
		Some(Commands::Remove { repo_folders }) => {
			init_gitopolis()
				.remove(repo_folders)
				.expect("Failed to remove repository");
		}
		Some(Commands::List { filter, long, tab }) => {
			let filter = filter.to_filter();
			let format = if *long {
				ListFormat::Long
			} else if *tab {
				ListFormat::Tab
			} else {
				ListFormat::Names
			};
			list(
				init_gitopolis()
					.list(&filter)
					.expect("Failed to list repositories"),
				format,
			)
		}
		Some(Commands::Clone {
			url,
			target_dir,
			tag: tag_args,
			exclude,
			exclude_tag,
		}) => {
			if url.is_some() && !(exclude.is_empty() && exclude_tag.is_empty()) {
				eprintln!("Error: --exclude and --exclude-tag can only be used when cloning from existing config (no url)");
				std::process::exit(1);
			}
			clone(
				url,
				target_dir,
				tag_args,
				TagFilter::from_cli_args(tag_args).with_exclusions(exclude, exclude_tag),
			)
		}
		Some(Commands::Exec {
			filter,
			oneline,
			skip_blank,
			timeout,
			exec_args,
		}) => {
			let filter = filter.to_filter();
			exec(
				exec_args.to_owned(),
				init_gitopolis()
					.list(&filter)
					.expect("Failed to list repositories for exec"),
				*oneline,
				*skip_blank,
				match timeout {
					0 => None,
					secs => Some(std::time::Duration::from_secs(*secs)),
				},
			);
		}
		Some(Commands::Tag {
			tag: tag_name,
			repo_folders,
			remove,
		}) => {
			let tags: Vec<&str> = tag_name.split(',').map(|s| s.trim()).collect();
			if *remove && repo_folders.is_empty() {
				let mut repo_tags: BTreeMap<String, Vec<&str>> = BTreeMap::new();
				for tag in &tags {
					match init_gitopolis().remove_tag_from_all(tag) {
						Ok(affected) => {
							for repo in affected {
								repo_tags.entry(repo).or_default().push(tag);
							}
						}
						Err(error) => {
							eprintln!("Error: {}", error.message());
							std::process::exit(1);
						}
					}
				}
				if repo_tags.is_empty() {
					println!("Tags {} not found on any repos", tags.join(","));
				} else {
					println!("Removed tags:");
					for (repo, removed_tags) in &repo_tags {
						println!("{} {}", repo, removed_tags.join(","));
					}
				}
			} else if !*remove && repo_folders.is_empty() {
				eprintln!("Error: repo_folders are required when adding tags");
				std::process::exit(1);
			} else {
				for tag in tags {
					let result = if *remove {
						init_gitopolis().remove_tag(tag, repo_folders)
					} else {
						init_gitopolis().add_tag(tag, repo_folders)
					};
					if let Err(error) = result {
						eprintln!("Error: {}", error.message());
						std::process::exit(1);
					}
				}
			}
		}
		Some(Commands::Tags { long }) => list_tags(*long),
		Some(Commands::Sync {
			read_remotes,
			write_remotes,
			dry_run,
			filter,
		}) => {
			let filter = filter.to_filter();
			if *read_remotes {
				init_gitopolis()
					.sync_read_remotes(&filter, *dry_run)
					.expect("Sync read failed");
			} else if *write_remotes {
				init_gitopolis()
					.sync_write_remotes(&filter, *dry_run)
					.expect("Sync write failed");
			} else {
				eprintln!("Error: Must specify either --read-remotes or --write-remotes");
				std::process::exit(1);
			}
		}
		Some(Commands::Show { repo_folder }) => {
			show(repo_folder);
		}
		Some(Commands::Move { entity }) => match entity {
			MoveEntity::Repo { old_path, new_path } => {
				match init_gitopolis().move_repo(old_path, new_path) {
					Ok(resolved_path) => {
						eprintln!("Moved {} to {}", old_path, resolved_path);
					}
					Err(error) => {
						eprintln!("Error: {}", error.message());
						std::process::exit(1);
					}
				}
			}
		},
		Some(Commands::Rename { entity }) => match entity {
			RenameEntity::Tag {
				old_name,
				new_name,
				merge,
			} => match init_gitopolis().rename_tag(old_name, new_name, *merge) {
				Ok(affected) => {
					if affected.is_empty() {
						println!("Tag '{}' not found on any repos", old_name);
					} else {
						println!("Renamed tag '{}' to '{}' on:", old_name, new_name);
						for repo in &affected {
							println!("{}", repo);
						}
					}
				}
				Err(error) => {
					eprintln!("Error: {}", error.message());
					std::process::exit(1);
				}
			},
		},
		Some(Commands::Man { out_dir }) => man(out_dir),
		None => {
			panic!("no command") // this doesn't happen because help shows instead
		}
	}
}

/// Clone repository/repositories with dual behavior depending on URL presence.
///
/// # Behavior
///
/// ## When URL is provided
/// Clones a single repository from the given URL and adds it to gitopolis.
/// All tags from tag_args are flattened and applied to the cloned repository.
/// - Example: `--tag foo,bar --tag baz` results in repo having tags: [foo, bar, baz]
///
/// ## When URL is omitted
/// Clones all repositories from .gitopolis.toml configuration that match the tag filter.
/// Uses AND/OR logic for filtering:
/// - Comma-separated tags within one --tag flag use AND logic
/// - Multiple --tag flags use OR logic
/// - Example: `--tag foo,bar --tag baz,boz` clones repos matching (foo AND bar) OR (baz AND boz)
///
/// # Arguments
///
/// * `url` - Optional git URL to clone from
/// * `target_dir` - Optional target directory name (only used when URL is provided)
/// * `tag_args` - Tags to apply (with URL)
/// * `filter` - Tag/exclusion filter (without URL)
fn clone(
	url: &Option<String>,
	target_dir: &Option<String>,
	tag_args: &[String],
	filter: TagFilter,
) {
	match url {
		Some(git_url) => clone_from_url(git_url, target_dir, tag_args),
		None => {
			// Clone from .gitopolis.toml with tag filtering
			let gitopolis = init_gitopolis();
			gitopolis.clone(
				gitopolis
					.list(&filter)
					.expect("Failed to list repositories for cloning"),
			);
		}
	}
}

/// Clone a single repository from a URL and add it to gitopolis.
///
/// All tags from tag_args are flattened (comma-separated tags are split)
/// and applied to the cloned repository. No AND/OR filtering logic is used
/// since we're cloning a single repo.
///
/// # Arguments
///
/// * `git_url` - Git URL to clone from
/// * `target_dir` - Optional target directory name. If None, extracts from URL
/// * `tag_args` - Tags to apply to the cloned repo (all tags are flattened)
///
/// # Example
///
/// `--tag foo,bar --tag baz` results in repo having tags: [foo, bar, baz]
fn clone_from_url(git_url: &str, target_dir: &Option<String>, tag_args: &[String]) {
	let mut gitopolis = init_gitopolis();
	// Flatten all tags - when cloning a single repo, all tags are applied (no AND/OR logic)
	let tags = flatten_tags(tag_args);
	match gitopolis.clone_and_add(git_url, target_dir.as_deref(), &tags) {
		Ok(folder_name) => {
			println!("Successfully cloned and added {}", folder_name);
		}
		Err(error) => {
			eprintln!("Error: {}", error.message());
			std::process::exit(1);
		}
	}
}

const STATE_FILE: &str = ".gitopolis.toml";

/// https://github.com/timabell/gitopolis/issues/239
fn man(out_dir: &Option<std::path::PathBuf>) {
	// The short about text also has license/repo lines which look odd in the man page NAME section
	let cmd = Args::command();
	let full_about = cmd.get_about().cloned();
	let cmd = cmd
		// generate_to() does this itself; do it for stdout too so SUBCOMMANDS doesn't list a non-existent gitopolis-help(1)
		.disable_help_subcommand(true)
		.about("A CLI tool for managing multiple git repositories")
		.long_about(full_about.map(|a| a.to_string()).unwrap_or_default());
	let result = match out_dir {
		Some(dir) => std::fs::create_dir_all(dir).and_then(|_| clap_mangen::generate_to(cmd, dir)),
		None => clap_mangen::Man::new(cmd).render(&mut std::io::stdout()),
	};
	if let Err(error) = result {
		eprintln!("Error generating man pages: {error}");
		std::process::exit(1);
	}
}

fn init_gitopolis() -> Gitopolis {
	Gitopolis::new(
		Box::new(StorageImpl { path: STATE_FILE }),
		Box::new(GitImpl {}),
	)
}

fn add(repo_folders: Vec<String>, tag_args: &[String]) {
	let mut added_folders: Vec<String> = vec![];
	let mut skipped_count = 0;
	for repo_folder in repo_folders {
		match init_gitopolis().add(repo_folder.to_owned()) {
			Ok(()) => added_folders.push(repo_folder),
			// Couldn't read it as a git repo (e.g. a plain folder or file matched by `add *`) - warn and carry on with the rest
			Err(GitopolisError::GitError { message }) => {
				eprintln!("Skipped {repo_folder}: {message}");
				skipped_count += 1;
			}
			Err(error) => {
				eprintln!("Error: {}", error.message());
				std::process::exit(1);
			}
		}
	}
	// Only tag the folders that were actually added, skipped ones aren't in the config
	if !added_folders.is_empty() {
		for tag in flatten_tags(tag_args) {
			if let Err(error) = init_gitopolis().add_tag(&tag, &added_folders) {
				eprintln!("Error: {}", error.message());
				std::process::exit(1);
			}
		}
	}
	if skipped_count > 0 {
		eprintln!("{skipped_count} folder(s) skipped (see above for why)");
		std::process::exit(1);
	}
}

/// Flatten repeated, comma-separated tag args into a single list of tags (no AND/OR logic).
/// `--tag foo,bar --tag baz` becomes [foo, bar, baz]
fn flatten_tags(tag_args: &[String]) -> Vec<String> {
	tag_args
		.iter()
		.flat_map(|s| s.split(',').map(|t| t.trim().to_string()))
		.filter(|t| !t.is_empty())
		.collect()
}

enum ListFormat {
	/// Just repo paths, one per line
	Names,
	/// Tab separated, one line per repo, for machines
	Tab,
	/// Multi-line, for humans
	Long,
}

fn list(repos: Vec<Repo>, format: ListFormat) {
	if repos.is_empty() {
		println!("No repos");
		std::process::exit(2);
	}
	for repo in &repos {
		match format {
			ListFormat::Names => println!("{}", repo.path),
			ListFormat::Tab => {
				let remotes_str = repo
					.remotes
					.iter()
					.map(|(name, remote)| format!("{}={}", name, remote.url))
					.collect::<Vec<_>>()
					.join(",");
				println!("{}\t{}\t{}", repo.path, repo.tags.join(","), remotes_str);
			}
			ListFormat::Long => {
				println!("{}", repo.path);
				println!("  Tags:");
				if repo.tags.is_empty() {
					println!("    (none)");
				}
				for tag in &repo.tags {
					println!("    {}", tag);
				}
				println!("  Remotes:");
				if repo.remotes.is_empty() {
					println!("    (none)");
				}
				for (name, remote) in &repo.remotes {
					println!("    {}: {}", name, remote.url);
				}
				println!();
			}
		}
	}
}

fn list_tags(long: bool) {
	let gitopolis = &init_gitopolis();
	if long {
		for tag in gitopolis.tags().expect("Failed to get tags") {
			println!("{tag}");
			let filter = TagFilter::from_cli_args(std::slice::from_ref(&tag));
			for r in gitopolis
				.list(&filter)
				.expect("Failed to list repositories for tag")
			{
				println!("\t{}", r.path);
			}
			println!();
		}
	} else {
		for tag in gitopolis.tags().expect("Failed to get tags") {
			println!("{tag}");
		}
	}
}

fn show(repo_folder: &str) {
	let gitopolis = init_gitopolis();
	match gitopolis.show(repo_folder) {
		Ok(repo_info) => {
			println!("Tags:");
			if repo_info.tags.is_empty() {
				println!("  (none)");
			} else {
				for tag in &repo_info.tags {
					println!("  {}", tag);
				}
			}
			println!();

			println!("Remotes:");
			if repo_info.remotes.is_empty() {
				println!("  (none)");
			} else {
				for (name, remote) in &repo_info.remotes {
					println!("  {}: {}", name, remote.url);
				}
			}
		}
		Err(error) => {
			eprintln!("Error: {}", error.message());
			std::process::exit(1);
		}
	}
}
