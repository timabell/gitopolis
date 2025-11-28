use crate::gitopolis::GitopolisError;
use crate::gitopolis::GitopolisError::{GitError, GitRemoteError};
use git2::Repository;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub trait Git {
	fn read_url(&self, path: String, remote_name: String) -> Result<String, GitopolisError>;
	fn read_all_remotes(&self, path: String) -> Result<BTreeMap<String, String>, GitopolisError>;
	fn add_remote(&self, path: &str, remote_name: &str, url: &str);
	/// Clone a repo. Returns Ok(true) if cloned, Ok(false) if already exists (skipped).
	fn clone(
		&self,
		path: &str,
		url: &str,
		remote_name: Option<&str>,
	) -> Result<bool, GitopolisError>;
}

pub struct GitImpl {}

impl Git for GitImpl {
	fn read_url(&self, path: String, remote_name: String) -> Result<String, GitopolisError> {
		let repository = Repository::open(path).map_err(|error| GitError {
			message: format!("Couldn't open git repo. {}", error.message()),
		})?;
		let remote = repository
			.find_remote(remote_name.as_str())
			.map_err(|error| GitRemoteError {
				message: format!("Remote not found. {}", error.message()),
				remote: remote_name,
			})?;
		let url: String = remote.url().unwrap_or("").to_string();
		Ok(url)
	}

	fn read_all_remotes(&self, path: String) -> Result<BTreeMap<String, String>, GitopolisError> {
		let repository = Repository::open(path).map_err(|error| GitError {
			message: format!("Couldn't open git repo. {}", error.message()),
		})?;
		let remote_names = repository.remotes().map_err(|error| GitError {
			message: format!("Failed to read remotes. {}", error.message()),
		})?;

		let mut remotes = BTreeMap::new();
		for remote_name in remote_names.iter().flatten() {
			if let Ok(remote) = repository.find_remote(remote_name) {
				if let Some(url) = remote.url() {
					remotes.insert(remote_name.to_string(), url.to_string());
				}
			}
		}
		Ok(remotes)
	}

	fn add_remote(&self, path: &str, remote_name: &str, url: &str) {
		let output = Command::new("git")
			.current_dir(path)
			.args(
				[
					"remote".to_string(),
					"add".to_string(),
					remote_name.to_string(),
					url.to_string(),
				]
				.to_vec(),
			)
			.output()
			.expect("Error running git remote add");
		if !output.status.success() {
			let stderr =
				String::from_utf8(output.stderr).expect("Error converting stderr to string");
			eprintln!("Warning: Failed to add remote {remote_name}: {stderr}");
		}
	}

	fn clone(
		&self,
		path: &str,
		url: &str,
		remote_name: Option<&str>,
	) -> Result<bool, GitopolisError> {
		if Path::new(path).exists() {
			println!("🏢 {path}> Already exists, skipped.");
			return Ok(false);
		}
		println!("🏢 {path}> Cloning {url} ...");
		let mut args = vec!["clone"];
		if let Some(name) = remote_name {
			args.push("--origin");
			args.push(name);
		}
		args.push(url);
		args.push(path);
		let output = Command::new("git")
			.args(args)
			.output()
			.expect("Error running git clone");
		let stdout = String::from_utf8(output.stdout).expect("Error converting stdout to string");
		let stderr = String::from_utf8(output.stderr).expect("Error converting stderr to string");
		println!("{stdout}");
		println!("{stderr}");

		if !output.status.success() {
			return Err(GitError {
				message: format!("Failed to clone {} to {}", url, path),
			});
		}

		Ok(true)
	}
}
