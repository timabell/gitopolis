use std::process::Command;

/// hacky call to external git command to get url of origin
pub(crate) fn read_url(path: &str, remote_name: &str) -> String {
	repo_capture_exec(
		&path,
		"git",
		&["config".to_string(), format!("remote.{}.url", remote_name)].to_vec(),
	)
	.trim()
	.to_owned()
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
