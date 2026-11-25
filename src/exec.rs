use crate::repos::Repo;
use std::env;
use std::io::{BufRead, BufReader, Error, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Result of running a command in one repo
enum Outcome {
	Exited(ExitStatus),
	TimedOut,
}

impl Outcome {
	fn success(&self) -> bool {
		match self {
			Outcome::Exited(status) => status.success(),
			Outcome::TimedOut => false,
		}
	}
}

/// Run a command in each repo.
/// `timeout`: kill the command (and anything it started) if it runs longer than this. None = no limit.
pub fn exec(
	exec_args: Vec<String>,
	repos: Vec<Repo>,
	oneline: bool,
	skip_blank: bool,
	timeout: Option<Duration>,
) {
	let mut failed_repos: Vec<&str> = vec![];
	let mut skipped_repos: Vec<&str> = vec![];
	for repo in &repos {
		if !exists(&repo.path) {
			if oneline {
				println!("{}\tRepo folder missing, skipped.", repo.path);
			} else {
				println!();
				println!("🏢 {}> Repo folder missing, skipped.", repo.path);
			}
			skipped_repos.push(&repo.path);
			continue;
		}
		if oneline {
			let (output, success) = repo_exec_oneline(&repo.path, &exec_args, timeout)
				.expect("Failed to execute command.");
			if skip_blank && success && output.is_none() {
				continue;
			}
			match output {
				Some(output_text) => println!("{}\t{}", repo.path, output_text),
				None => println!("{}\t", repo.path),
			}
			if !success {
				failed_repos.push(&repo.path);
			}
		} else {
			let (exit_status, had_output) = repo_exec(&repo.path, &exec_args, skip_blank, timeout)
				.expect("Failed to execute command.");
			if skip_blank && exit_status.success() && !had_output {
				continue;
			}
			if !exit_status.success() {
				failed_repos.push(&repo.path);
			}
			println!();
		}
	}
	if !failed_repos.is_empty() || !skipped_repos.is_empty() {
		eprint!("{}", summary(&failed_repos, &skipped_repos));
		std::process::exit(1);
	}
}

/// Summary of which repos failed / were skipped, so you don't have to scroll back through a long
/// list of output to find them. https://github.com/timabell/gitopolis/issues/280
fn summary(failed_repos: &[&str], skipped_repos: &[&str]) -> String {
	let mut out = String::new();
	if !failed_repos.is_empty() {
		out.push_str(&format!(
			"{} commands exited with non-zero status code:\n",
			failed_repos.len()
		));
		for repo in failed_repos {
			out.push_str(&format!("  {repo}\n"));
		}
	}
	if !skipped_repos.is_empty() {
		out.push_str(&format!("{} repos skipped:\n", skipped_repos.len()));
		for repo in skipped_repos {
			out.push_str(&format!("  {repo}\n"));
		}
	}
	out
}

fn exists(repo_path: &String) -> bool {
	let mut path = env::current_dir().expect("failed to get current working directory");
	path.push(repo_path);
	path.exists() && path.is_dir()
}

fn needs_quoting(arg: &str) -> bool {
	arg.chars().any(|c| {
		c.is_whitespace()
			|| matches!(
				c,
				'|' | '&'
					| ';'
					| '<'
					| '>'
					| '('
					| ')'
					| '$'
					| '`'
					| '\\'
					| '"'
					| '\''
					| '*'
					| '?'
					| '['
					| ']'
					| '{'
					| '}'
					| '!'
					| '#'
			)
	})
}

/// Formats command arguments for display with intelligent quoting.
///
/// This function reconstructs a shell command string from parsed arguments,
/// adding quotes where needed to make the output readable and copy-pasteable.
///
/// # Why We Need Heuristics
///
/// By the time we receive the arguments here, the shell has already parsed
/// the user's input and consumed the original quotes. For example:
/// - User types: `gitopolis exec -- git log --since="One Week"`
/// - Shell parses this into separate args: `["git", "log", "--since=One Week"]`
/// - We receive: The value "One Week" with no information about original quoting
///
/// Since we've lost the original quoting style, we use heuristics to reconstruct
/// a readable command that would execute correctly if copy-pasted.
///
/// # Strategy
///
/// - Arguments without special chars: display as-is
/// - Arguments with spaces/special chars: wrap in single quotes
/// - Arguments matching `--flag=value` pattern: quote only the value portion
///   for better readability (e.g., `--since='One Week'` vs `'--since=One Week'`)
/// - Values containing single quotes: use double quotes with escaping
fn format_args_for_display(args: &[String]) -> String {
	args.iter()
		.map(|arg| {
			if needs_quoting(arg) {
				// Check if this is a --flag=value or -flag=value pattern
				if arg.starts_with('-') && arg.contains('=') {
					format_key_value_for_display(arg)
				} else {
					// Not a flag=value pattern, quote the whole thing
					quote_arg(arg)
				}
			} else {
				arg.clone()
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

/// Formats a --key=value argument, quoting only the value portion if needed.
///
/// For example: `--since=One Week` becomes `--since='One Week'`
fn format_key_value_for_display(arg: &str) -> String {
	let eq_pos = arg.find('=').expect("arg must contain '='");
	let (key, value) = arg.split_at(eq_pos);
	let value = &value[1..]; // Skip the '=' character

	// Quote only the value portion if it needs quoting
	if needs_quoting(value) {
		if value.contains('\'') {
			// For values containing single quotes, use double quotes and escape
			format!(
				"{}=\"{}\"",
				key,
				value.replace('\\', "\\\\").replace('"', "\\\"")
			)
		} else {
			format!("{}='{}'", key, value)
		}
	} else {
		// Value doesn't need quoting, return as-is
		arg.to_string()
	}
}

fn quote_arg(arg: &str) -> String {
	// Use single quotes for simplicity, escape any single quotes in the string
	if arg.contains('\'') {
		// For strings containing single quotes, use double quotes and escape
		format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
	} else {
		format!("'{}'", arg)
	}
}

fn repo_exec(
	path: &str,
	exec_args: &[String],
	skip_blank: bool,
	timeout: Option<Duration>,
) -> Result<(Outcome, bool), Error> {
	let header = format!("\n🏢 {}> {}", path, format_args_for_display(exec_args));

	if !skip_blank {
		print!("{}", header);
		println!();
	}

	// If single argument, pass directly to shell for interpretation (supports pipes, etc.)
	// If multiple arguments, pass via positional parameters to avoid quoting issues
	#[cfg(unix)]
	let mut child_process: Child = if exec_args.len() == 1 {
		Command::new("sh")
			.arg("-c")
			.arg(&exec_args[0]) // Single arg passed directly for shell interpretation
			.current_dir(path)
			.stdin(Stdio::null()) // Prevent interactive prompts/pagers
			.stdout(Stdio::piped()) // Prevent TTY detection for pagers
			.stderr(Stdio::piped())
			.spawn()?
	} else {
		Command::new("sh")
			.arg("-c")
			.arg(r#""$@""#) // Execute all positional parameters
			.arg("--") // $0 placeholder (ignored)
			.args(exec_args) // These become $1, $2, $3, etc.
			.current_dir(path)
			.stdin(Stdio::null()) // Prevent interactive prompts/pagers
			.stdout(Stdio::piped()) // Prevent TTY detection for pagers
			.stderr(Stdio::piped())
			.spawn()?
	};

	#[cfg(windows)]
	let mut child_process: Child = if exec_args.len() == 1 {
		Command::new("cmd")
			.arg("/C")
			.arg(&exec_args[0]) // Single arg passed directly for shell interpretation
			.current_dir(path)
			.stdin(Stdio::null()) // Prevent interactive prompts/pagers
			.stdout(Stdio::piped()) // Prevent TTY detection for pagers
			.stderr(Stdio::piped())
			.spawn()?
	} else {
		// Windows cmd doesn't have an equivalent to sh -c "$@"
		// We need to join args with proper quoting
		let command_string = exec_args
			.iter()
			.map(|arg| {
				// Quote if contains spaces or special chars
				if arg.contains(' ') || arg.contains('"') || arg.contains('&') || arg.contains('|')
				{
					format!("\"{}\"", arg.replace('"', "\"\""))
				} else {
					arg.clone()
				}
			})
			.collect::<Vec<_>>()
			.join(" ");
		Command::new("cmd")
			.arg("/C")
			.arg(command_string)
			.current_dir(path)
			.stdin(Stdio::null()) // Prevent interactive prompts/pagers
			.stdout(Stdio::piped()) // Prevent TTY detection for pagers
			.stderr(Stdio::piped())
			.spawn()?
	};

	// Stream stdout and stderr in real-time using threads
	let stdout = child_process
		.stdout
		.take()
		.expect("Failed to capture stdout");
	let stderr = child_process
		.stderr
		.take()
		.expect("Failed to capture stderr");

	// When skip_blank, defer header until first output byte arrives.
	// AtomicBool ensures only the first thread (stdout or stderr) prints the header.
	let header_printed = Arc::new(AtomicBool::new(!skip_blank));
	let header_for_stdout = header.clone();
	let header_printed_stdout = Arc::clone(&header_printed);
	let header_printed_stderr = Arc::clone(&header_printed);
	// Set if we give up waiting for output after a timeout, so stray output from anything that survived
	// the kill doesn't get printed under the next repo's header.
	let abandoned = Arc::new(AtomicBool::new(false));
	let abandoned_stdout = Arc::clone(&abandoned);
	let abandoned_stderr = Arc::clone(&abandoned);

	let stdout_thread = thread::spawn(move || {
		let reader = BufReader::new(stdout);
		for line in reader.lines().map_while(Result::ok) {
			if abandoned_stdout.load(Ordering::SeqCst) {
				break;
			}
			if header_printed_stdout
				.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
				.is_ok()
			{
				println!("{}", header_for_stdout);
			}
			println!("{}", line);
		}
	});

	let header_for_stderr = header;
	let stderr_thread = thread::spawn(move || {
		let reader = BufReader::new(stderr);
		for line in reader.lines().map_while(Result::ok) {
			if abandoned_stderr.load(Ordering::SeqCst) {
				break;
			}
			if header_printed_stderr
				.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
				.is_ok()
			{
				println!("{}", header_for_stderr);
			}
			eprintln!("{}", line);
		}
	});

	let outcome = wait_with_timeout(child_process, timeout)?;

	// Wait for output threads to finish
	let deadline = output_deadline(&outcome);
	let stdout_done = join_output_thread(stdout_thread, deadline).is_some();
	let stderr_done = join_output_thread(stderr_thread, deadline).is_some();
	if !(stdout_done && stderr_done) {
		abandoned.store(true, Ordering::SeqCst);
	}

	let had_output = header_printed.load(Ordering::SeqCst);

	// For failed commands, print header retroactively if nothing was output
	if !outcome.success() {
		if !had_output {
			println!("\n🏢 {}> {}", path, format_args_for_display(exec_args));
		}
		eprintln!("{}", failure_message(&outcome, timeout));
	}
	let success = outcome.success();
	Ok((outcome, had_output || !success))
}

fn failure_message(outcome: &Outcome, timeout: Option<Duration>) -> String {
	match outcome {
		Outcome::Exited(status) => match status.code() {
			Some(code) => format!("Command exited with code {code}"),
			None => "Command was terminated by a signal".to_string(),
		},
		Outcome::TimedOut => format!(
			"Command timed out after {} seconds and was killed (use --timeout to change, 0 for no limit)",
			timeout.map(|t| t.as_secs()).unwrap_or_default()
		),
	}
}

/// Wait for the child to exit, killing it (and its descendants) if it takes longer than `timeout`.
/// https://github.com/timabell/gitopolis/issues/276
fn wait_with_timeout(mut child: Child, timeout: Option<Duration>) -> Result<Outcome, Error> {
	let Some(timeout) = timeout else {
		return Ok(Outcome::Exited(child.wait()?));
	};
	// Wait on a separate thread so we can stop waiting at the deadline without polling (no added latency).
	// The child isn't reaped until wait() returns, so its pid can't be reused before we kill it.
	let pid = child.id();
	let (sender, receiver) = mpsc::channel();
	thread::spawn(move || {
		let _ = sender.send(child.wait());
	});
	match receiver.recv_timeout(timeout) {
		Ok(status) => Ok(Outcome::Exited(status?)),
		Err(_) => {
			kill_tree(pid);
			let _ = receiver.recv_timeout(Duration::from_secs(5));
			Ok(Outcome::TimedOut)
		}
	}
}

/// Kill a process and everything it started (e.g. `sh` -> `git` -> `ssh`), otherwise
/// grandchildren keep running and hold our output pipes open.
fn kill_tree(pid: u32) {
	#[cfg(unix)]
	{
		let mut pids = vec![pid];
		let mut i = 0;
		while i < pids.len() {
			match child_pids(pids[i]) {
				Some(children) => pids.extend(children),
				None => {
					eprintln!("Warning: pgrep not found, so only the shell was killed; processes it started may still be running");
					break;
				}
			}
			i += 1;
		}
		// `kill` isn't always available as a binary (e.g. NixOS), so use the shell builtin.
		let _ = Command::new("sh")
			.arg("-c")
			.arg(r#"kill -9 "$@" 2>/dev/null"#)
			.arg("--")
			.args(pids.iter().map(|p| p.to_string()))
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status();
	}
	#[cfg(windows)]
	{
		let _ = Command::new("taskkill")
			.args(["/T", "/F", "/PID", &pid.to_string()])
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status();
	}
}

/// Direct children of a process, using pgrep (available on linux and macOS).
/// None if pgrep isn't available.
#[cfg(unix)]
fn child_pids(pid: u32) -> Option<Vec<u32>> {
	Command::new("pgrep")
		.arg("-P")
		.arg(pid.to_string())
		.stdin(Stdio::null())
		.stderr(Stdio::null())
		.output()
		.ok()
		.map(|output| {
			String::from_utf8_lossy(&output.stdout)
				.lines()
				.filter_map(|line| line.trim().parse().ok())
				.collect()
		})
}

/// After a timeout, only wait a short while for remaining output, in case something we couldn't kill
/// is still holding the pipe open. Shared between stdout and stderr.
fn output_deadline(outcome: &Outcome) -> Option<Instant> {
	match outcome {
		Outcome::TimedOut => Some(Instant::now() + Duration::from_secs(2)),
		Outcome::Exited(_) => None,
	}
}

/// Wait for an output-reading thread, giving up at `deadline` if there is one.
fn join_output_thread<T>(handle: JoinHandle<T>, deadline: Option<Instant>) -> Option<T> {
	if let Some(deadline) = deadline {
		while !handle.is_finished() {
			if Instant::now() > deadline {
				return None;
			}
			thread::sleep(Duration::from_millis(10));
		}
	}
	handle.join().ok()
}

fn repo_exec_oneline(
	path: &str,
	exec_args: &[String],
	timeout: Option<Duration>,
) -> Result<(Option<String>, bool), Error> {
	// If single argument, pass directly to shell for interpretation (supports pipes, etc.)
	// If multiple arguments, pass via positional parameters to avoid quoting issues
	#[cfg(unix)]
	let mut child_process: Child = if exec_args.len() == 1 {
		Command::new("sh")
			.arg("-c")
			.arg(&exec_args[0]) // Single arg passed directly for shell interpretation
			.current_dir(path)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?
	} else {
		Command::new("sh")
			.arg("-c")
			.arg(r#""$@""#) // Execute all positional parameters
			.arg("--") // $0 placeholder (ignored)
			.args(exec_args) // These become $1, $2, $3, etc.
			.current_dir(path)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?
	};

	#[cfg(windows)]
	let mut child_process: Child = if exec_args.len() == 1 {
		Command::new("cmd")
			.arg("/C")
			.arg(&exec_args[0]) // Single arg passed directly for shell interpretation
			.current_dir(path)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?
	} else {
		// Windows cmd doesn't have an equivalent to sh -c "$@"
		// We need to join args with proper quoting
		let command_string = exec_args
			.iter()
			.map(|arg| {
				// Quote if contains spaces or special chars
				if arg.contains(' ') || arg.contains('"') || arg.contains('&') || arg.contains('|')
				{
					format!("\"{}\"", arg.replace('"', "\"\""))
				} else {
					arg.clone()
				}
			})
			.collect::<Vec<_>>()
			.join(" ");
		Command::new("cmd")
			.arg("/C")
			.arg(command_string)
			.current_dir(path)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?
	};

	// Read in threads so that we can enforce the timeout while the command is running
	let stdout_thread = read_to_string_thread(child_process.stdout.take());
	let stderr_thread = read_to_string_thread(child_process.stderr.take());

	let outcome = wait_with_timeout(child_process, timeout)?;
	let deadline = output_deadline(&outcome);
	let stdout = join_output_thread(stdout_thread, deadline).unwrap_or_default();
	let mut stderr = join_output_thread(stderr_thread, deadline).unwrap_or_default();
	let success = outcome.success();
	if let Outcome::TimedOut = outcome {
		if !stderr.trim().is_empty() {
			stderr.push(' ');
		}
		stderr.push_str(&failure_message(&outcome, timeout));
	}

	// Flatten multi-line output to single line by replacing newlines with spaces
	let stdout_clean = stdout.trim().replace('\n', " ");
	let stderr_clean = stderr.trim().replace('\n', " ");

	// Combine stdout and stderr, with stderr included when command fails
	let output = if !success && !stderr_clean.is_empty() {
		if stdout_clean.is_empty() {
			stderr_clean
		} else {
			format!("{} {}", stdout_clean, stderr_clean)
		}
	} else if !stdout_clean.is_empty() {
		stdout_clean
	} else {
		String::new()
	};

	if output.is_empty() {
		Ok((None, success))
	} else {
		Ok((Some(output), success))
	}
}

fn read_to_string_thread<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<String> {
	thread::spawn(move || {
		let mut buf = String::new();
		if let Some(mut pipe) = pipe {
			let _ = pipe.read_to_string(&mut buf);
		}
		buf
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_summary_failed_and_skipped() {
		assert_eq!(
			summary(&["repo_a", "repo_b"], &["missing"]),
			"2 commands exited with non-zero status code:\n  repo_a\n  repo_b\n1 repos skipped:\n  missing\n"
		);
	}

	#[test]
	fn test_summary_failed_only() {
		assert_eq!(
			summary(&["repo_a"], &[]),
			"1 commands exited with non-zero status code:\n  repo_a\n"
		);
	}

	#[test]
	fn test_format_args_no_quoting_needed() {
		let args = vec!["git".to_string(), "status".to_string()];
		assert_eq!(format_args_for_display(&args), "git status");
	}

	#[test]
	fn test_format_args_simple_quoting() {
		let args = vec!["echo".to_string(), "hello world".to_string()];
		assert_eq!(format_args_for_display(&args), "echo 'hello world'");
	}

	#[test]
	fn test_format_args_flag_with_value() {
		let args = vec![
			"git".to_string(),
			"log".to_string(),
			"-n".to_string(),
			"5".to_string(),
			"--since=One Week".to_string(),
		];
		assert_eq!(
			format_args_for_display(&args),
			"git log -n 5 --since='One Week'"
		);
	}

	#[test]
	fn test_format_args_flag_with_value_containing_single_quote() {
		let args = vec![
			"git".to_string(),
			"commit".to_string(),
			"-m=Don't panic".to_string(),
		];
		assert_eq!(
			format_args_for_display(&args),
			"git commit -m=\"Don't panic\""
		);
	}

	#[test]
	fn test_format_args_non_flag_key_value() {
		// If it doesn't start with -, quote the whole thing
		let args = vec!["foo=bar baz".to_string()];
		assert_eq!(format_args_for_display(&args), "'foo=bar baz'");
	}

	#[test]
	fn test_format_args_flag_with_no_spaces() {
		let args = vec!["--since=yesterday".to_string()];
		assert_eq!(format_args_for_display(&args), "--since=yesterday");
	}

	#[test]
	fn test_format_args_mixed() {
		let args = vec![
			"git".to_string(),
			"log".to_string(),
			"--author=John Doe".to_string(),
			"--format=%H".to_string(),
			"-n".to_string(),
			"10".to_string(),
		];
		assert_eq!(
			format_args_for_display(&args),
			"git log --author='John Doe' --format=%H -n 10"
		);
	}

	#[test]
	fn test_format_args_with_special_chars() {
		let args = vec!["echo".to_string(), "hello|world".to_string()];
		assert_eq!(format_args_for_display(&args), "echo 'hello|world'");
	}

	#[test]
	fn test_format_args_double_dash_flag() {
		let args = vec!["grep".to_string(), "--exclude=*.tmp files".to_string()];
		assert_eq!(
			format_args_for_display(&args),
			"grep --exclude='*.tmp files'"
		);
	}
}
