/// A filter for repositories based on tag matching with AND/OR logic.
///
/// Tag filtering works as follows:
/// - Each inner Vec represents tags that must ALL be present (AND logic)
/// - Different inner Vecs are ORed together
/// - Empty filter matches all repos
///
/// # Examples
///
/// ```
/// use gitopolis::tag_filter::TagFilter;
///
/// // Match all repos (no filter)
/// let filter = TagFilter::all();
///
/// // Match repos with BOTH "foo" AND "bar"
/// let filter = TagFilter::from_cli_args(&["foo,bar".to_string()]);
///
/// // Match repos with (foo AND bar) OR (baz AND boz)
/// let filter = TagFilter::from_cli_args(&["foo,bar".to_string(), "baz,boz".to_string()]);
///
/// // Match repos tagged foo, except repo1 and anything tagged "archived"
/// let filter = TagFilter::from_cli_args(&["foo".to_string()])
///     .with_exclusions(&["repo1".to_string()], &["archived".to_string()]);
/// ```
///
/// Exclusions are applied after the tag groups: a repo is excluded if its path is in the exclude list,
/// or it has ANY of the excluded tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFilter {
	tag_groups: Vec<Vec<String>>,
	exclude_repos: Vec<String>,
	exclude_tags: Vec<String>,
}

impl TagFilter {
	/// Create a filter that matches all repositories (no filtering)
	pub fn all() -> Self {
		Self {
			tag_groups: Vec::new(),
			exclude_repos: Vec::new(),
			exclude_tags: Vec::new(),
		}
	}

	/// Create a filter from CLI tag arguments.
	/// Each argument can contain comma-separated tags (AND logic).
	/// Multiple arguments are ORed together.
	pub fn from_cli_args(tag_args: &[String]) -> Self {
		if tag_args.is_empty() {
			return Self::all();
		}

		let tag_groups = tag_args
			.iter()
			.map(|tag_str| {
				tag_str
					.split(',')
					.map(|s| s.trim().to_string())
					.collect::<Vec<String>>()
			})
			.collect();

		Self {
			tag_groups,
			..Self::all()
		}
	}

	/// Add exclusions from CLI args. Each arg can be comma-separated and/or repeated.
	/// Repo paths have trailing slashes removed so tab-completed folder names work.
	pub fn with_exclusions(
		mut self,
		exclude_repo_args: &[String],
		exclude_tag_args: &[String],
	) -> Self {
		self.exclude_repos = split_args(exclude_repo_args)
			.into_iter()
			.map(|r| {
				r.trim_start_matches("./")
					.trim_start_matches(".\\")
					.trim_end_matches(['/', '\\'])
					.to_string()
			})
			.collect();
		self.exclude_tags = split_args(exclude_tag_args);
		self
	}

	/// Excluded repo paths that don't match any of the given known repo paths (probably typos).
	pub fn unknown_excluded_repos(&self, known_repo_paths: &[String]) -> Vec<String> {
		self.exclude_repos
			.iter()
			.filter(|r| !known_repo_paths.contains(r))
			.cloned()
			.collect()
	}

	/// Check if this filter matches a repository with the given path and tags, including exclusions.
	pub fn matches_repo(&self, repo_path: &str, repo_tags: &[String]) -> bool {
		!self.exclude_repos.iter().any(|r| r == repo_path) && self.matches(repo_tags)
	}

	/// Check if this filter matches a repository with the given tags.
	pub fn matches(&self, repo_tags: &[String]) -> bool {
		if self.exclude_tags.iter().any(|tag| repo_tags.contains(tag)) {
			return false;
		}
		if self.tag_groups.is_empty() {
			// No filter, match everything
			return true;
		}

		// Check if repo matches ANY of the tag groups (OR)
		self.tag_groups.iter().any(|tag_group| {
			// Check if repo has ALL tags in this group (AND)
			tag_group.iter().all(|tag| repo_tags.contains(tag))
		})
	}

	/// Returns true if this is an "all" filter (no filtering)
	pub fn is_all(&self) -> bool {
		self.tag_groups.is_empty() && self.exclude_repos.is_empty() && self.exclude_tags.is_empty()
	}
}

fn split_args(args: &[String]) -> Vec<String> {
	args.iter()
		.flat_map(|s| s.split(',').map(|t| t.trim().to_string()))
		.filter(|t| !t.is_empty())
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_filter_matches_everything() {
		let filter = TagFilter::all();
		assert!(filter.matches(&[]));
		assert!(filter.matches(&["foo".to_string()]));
		assert!(filter.matches(&["foo".to_string(), "bar".to_string()]));
		assert!(filter.is_all());
	}

	#[test]
	fn empty_cli_args_creates_all_filter() {
		let filter = TagFilter::from_cli_args(&[]);
		assert!(filter.is_all());
		assert!(filter.matches(&["anything".to_string()]));
	}

	#[test]
	fn single_tag_matches_repos_with_that_tag() {
		let filter = TagFilter::from_cli_args(&["foo".to_string()]);
		assert!(filter.matches(&["foo".to_string()]));
		assert!(filter.matches(&["foo".to_string(), "bar".to_string()]));
		assert!(!filter.matches(&["bar".to_string()]));
		assert!(!filter.matches(&[]));
	}

	#[test]
	fn comma_separated_tags_require_all_and_logic() {
		let filter = TagFilter::from_cli_args(&["foo,bar".to_string()]);
		assert!(filter.matches(&["foo".to_string(), "bar".to_string()]));
		assert!(filter.matches(&["foo".to_string(), "bar".to_string(), "baz".to_string()]));
		assert!(!filter.matches(&["foo".to_string()]));
		assert!(!filter.matches(&["bar".to_string()]));
		assert!(!filter.matches(&[]));
	}

	#[test]
	fn multiple_tag_groups_use_or_logic() {
		let filter = TagFilter::from_cli_args(&["foo,bar".to_string(), "baz,boz".to_string()]);
		// Matches (foo AND bar)
		assert!(filter.matches(&["foo".to_string(), "bar".to_string()]));
		// Matches (baz AND boz)
		assert!(filter.matches(&["baz".to_string(), "boz".to_string()]));
		// Matches both groups
		assert!(filter.matches(&[
			"foo".to_string(),
			"bar".to_string(),
			"baz".to_string(),
			"boz".to_string()
		]));
		// Doesn't match - only has foo (not foo AND bar)
		assert!(!filter.matches(&["foo".to_string()]));
		// Doesn't match - only has baz (not baz AND boz)
		assert!(!filter.matches(&["baz".to_string()]));
	}

	#[test]
	fn exclude_tags_removes_repos_with_any_excluded_tag() {
		let filter = TagFilter::all()
			.with_exclusions(&[], &["archived,old".to_string(), "slow".to_string()]);
		assert!(!filter.is_all());
		assert!(filter.matches(&[]));
		assert!(filter.matches(&["foo".to_string()]));
		assert!(!filter.matches(&["archived".to_string()]));
		assert!(!filter.matches(&["foo".to_string(), "old".to_string()]));
		assert!(!filter.matches(&["slow".to_string()]));
	}

	#[test]
	fn exclude_tags_applied_after_tag_groups() {
		let filter = TagFilter::from_cli_args(&["foo".to_string()])
			.with_exclusions(&[], &["archived".to_string()]);
		assert!(filter.matches(&["foo".to_string()]));
		assert!(!filter.matches(&["foo".to_string(), "archived".to_string()]));
		assert!(!filter.matches(&["bar".to_string()]));
	}

	#[test]
	fn exclude_repos_by_path() {
		let filter = TagFilter::all().with_exclusions(
			&["repo1,repo2/".to_string(), "nested/repo3".to_string()],
			&[],
		);
		assert!(filter.matches_repo("repo0", &[]));
		assert!(!filter.matches_repo("repo1", &[]));
		assert!(!filter.matches_repo("repo2", &[]));
		assert!(!filter.matches_repo("nested/repo3", &[]));
		assert!(filter.matches_repo("repo3", &[]));
	}

	#[test]
	fn exclude_repos_strips_dot_slash() {
		let filter = TagFilter::all().with_exclusions(&["./repo1".to_string()], &[]);
		assert!(!filter.matches_repo("repo1", &[]));
	}

	#[test]
	fn unknown_excluded_repos_reported() {
		let filter = TagFilter::all().with_exclusions(&["repo1,typo".to_string()], &[]);
		assert_eq!(
			filter.unknown_excluded_repos(&["repo1".to_string(), "repo2".to_string()]),
			vec!["typo".to_string()]
		);
	}

	#[test]
	fn whitespace_is_trimmed() {
		let filter = TagFilter::from_cli_args(&[" foo , bar ".to_string()]);
		assert!(filter.matches(&["foo".to_string(), "bar".to_string()]));
	}
}
