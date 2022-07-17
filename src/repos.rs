use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Repos {
	// todo: make inner repos private if possible
	pub repos: Vec<Repo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repo {
	pub path: String,
	pub tags: Vec<String>,
	pub remotes: BTreeMap<String, Remote>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Remote {
	pub name: String,
	pub url: String,
}

impl Repos {
	pub fn new() -> Repos {
		Repos { repos: Vec::new() }
	}

	pub fn find_repo(&mut self, folder_name: &str) -> Option<&mut Repo> {
		if let Some(ix) = self.repo_index(folder_name) {
			return Some(&mut self.repos[ix]);
		}
		None
	}

	pub fn exists(&self, folder_name: &str) -> bool {
		match self.repo_index(folder_name) {
			Some(_) => true,
			_ => false,
		}
	}

	pub fn repo_index(&self, folder_name: &str) -> Option<usize> {
		self.repos.iter().position(|r| r.path == *folder_name)
	}

	pub fn add(&mut self, repo: Repo) {
		self.repos.push(repo);
	}

	pub fn remove(&mut self, repo_folders: &Vec<String>) {
		for repo_folder in repo_folders {
			let ix = self
				.repo_index(repo_folder)
				.expect(&format!("Repo '{}' not found", repo_folder));
			self.repos.remove(ix);
		}
	}

	pub fn add_tag(&mut self, tag_name: &str, repo_folders: &Vec<String>) {
		self.tag(tag_name, repo_folders, false)
	}
	pub fn remove_tag(&mut self, tag_name: &str, repo_folders: &Vec<String>) {
		self.tag(tag_name, repo_folders, true)
	}
	fn tag(&mut self, tag_name: &str, repo_folders: &Vec<String>, remove: bool) {
		for repo_folder in repo_folders {
			let repo = self
				.find_repo(repo_folder)
				.expect(&format!("Repo '{}' not found", repo_folder));
			if remove {
				if let Some(ix) = repo.tags.iter().position(|t| t == tag_name) {
					repo.tags.remove(ix);
				}
			} else {
				repo.tags.push(tag_name.to_string());
			}
		}
	}
}
