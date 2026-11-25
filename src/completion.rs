//! Shell completion helpers. The actual hooking up to clap is in main.rs, these are the
//! pure functions that work out what to suggest so they can be unit tested.

/// Suggest tags for the value currently being typed.
///
/// Tag arguments support comma-separated lists, so only the segment after the last comma is
/// completed, keeping the earlier segments as a prefix. Tags already in the list are not suggested again.
///
/// ```
/// use gitopolis::completion::complete_tag_list;
/// let tags = vec!["backend".to_string(), "frontend".to_string(), "rust".to_string()];
/// assert_eq!(complete_tag_list(&tags, "ba"), vec!["backend"]);
/// assert_eq!(complete_tag_list(&tags, "rust,"), vec!["rust,backend", "rust,frontend"]);
/// ```
pub fn complete_tag_list(all_tags: &[String], current: &str) -> Vec<String> {
	let (prefix, partial) = match current.rfind(',') {
		Some(i) => (&current[..=i], current[i + 1..].trim_start()),
		None => ("", current),
	};
	let already: Vec<&str> = prefix.split(',').map(|t| t.trim()).collect();
	all_tags
		.iter()
		.filter(|tag| tag.starts_with(partial) && !already.contains(&tag.as_str()))
		.map(|tag| format!("{prefix}{tag}"))
		.collect()
}

/// Suggest known repo paths that start with what has been typed so far.
pub fn complete_repo(all_repos: &[String], current: &str) -> Vec<String> {
	all_repos
		.iter()
		.filter(|repo| repo.starts_with(current))
		.cloned()
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tags() -> Vec<String> {
		vec!["backend", "bar", "frontend", "rust"]
			.into_iter()
			.map(String::from)
			.collect()
	}

	#[test]
	fn empty_suggests_all() {
		assert_eq!(
			complete_tag_list(&tags(), ""),
			vec!["backend", "bar", "frontend", "rust"]
		);
	}

	#[test]
	fn prefix_filters() {
		assert_eq!(complete_tag_list(&tags(), "ba"), vec!["backend", "bar"]);
		assert_eq!(complete_tag_list(&tags(), "nope"), Vec::<String>::new());
	}

	#[test]
	fn completes_after_comma_and_skips_already_used() {
		assert_eq!(
			complete_tag_list(&tags(), "bar,"),
			vec!["bar,backend", "bar,frontend", "bar,rust"]
		);
		assert_eq!(
			complete_tag_list(&tags(), "rust,bar,f"),
			vec!["rust,bar,frontend"]
		);
	}

	#[test]
	fn space_after_comma_is_ignored() {
		assert_eq!(
			complete_tag_list(&tags(), "rust, fr"),
			vec!["rust,frontend"]
		);
	}

	#[test]
	fn repo_prefix() {
		let repos = vec!["api".to_string(), "app".to_string(), "web".to_string()];
		assert_eq!(complete_repo(&repos, "a"), vec!["api", "app"]);
		assert_eq!(complete_repo(&repos, ""), vec!["api", "app", "web"]);
	}
}
