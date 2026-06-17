//! Architecture checks for the Lab workspace.
//!
//! `cargo xtask check` holds the dependency-cone and module-fence rules that
//! Cargo cannot. Each check arrives with the crate it guards.
//!
//! Current checks (both guard `lab-core`, the ecosystem root):
//! - it has zero ecosystem (workspace) dependencies;
//! - its crate root never references `crate::report`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
  match std::env::args().nth(1).as_deref() {
    Some("check") => run_check(),
    Some(other) => {
      eprintln!("xtask: unknown command `{other}` (try `cargo xtask check`)");
      std::process::exit(2);
    }
    None => {
      eprintln!("xtask: usage: cargo xtask check");
      std::process::exit(2);
    }
  }
}

/// The workspace root — the parent of this `xtask` crate's directory.
fn workspace_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .expect("xtask manifest dir has a parent")
    .to_path_buf()
}

type Check = fn(&Path) -> Result<(), String>;

fn run_check() {
  let checks: [(&str, Check); 2] = [
    (
      "lab-core has zero ecosystem dependencies",
      check_lab_core_zero_ecosystem_deps,
    ),
    (
      "lab-core root does not reference crate::report",
      check_root_does_not_reference_report,
    ),
  ];

  let root = workspace_root();
  let mut failed = false;
  for (name, check) in checks {
    match check(&root) {
      Ok(()) => println!("ok: {name}"),
      Err(reason) => {
        eprintln!("FAIL: {name}\n      {reason}");
        failed = true;
      }
    }
  }

  if failed {
    std::process::exit(1);
  }
  println!("xtask check: all checks passed");
}

/// `lab-core` is the ecosystem root: it must depend on no other workspace
/// crate. Workspace members resolve as path dependencies under the workspace
/// root; external crates carry no such path and are out of scope for this rule.
fn check_lab_core_zero_ecosystem_deps(root: &Path) -> Result<(), String> {
  let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
  let output = Command::new(cargo)
    .current_dir(root)
    .args([
      "tree",
      "--package",
      "lab-core",
      "--edges",
      "normal",
      "--prefix",
      "none",
    ])
    .output()
    .map_err(|e| format!("could not run `cargo tree`: {e}"))?;

  if !output.status.success() {
    return Err(format!(
      "`cargo tree` failed: {}",
      String::from_utf8_lossy(&output.stderr).trim()
    ));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let root_path_marker = format!("({}", root.to_string_lossy());

  let offenders: Vec<&str> = stdout
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    // The crate itself heads the tree; skip its own line.
    .filter(|line| !line.starts_with("lab-core "))
    // A remaining line annotated with a workspace path is a workspace dep.
    .filter(|line| line.contains(&root_path_marker))
    .collect();

  if offenders.is_empty() {
    Ok(())
  } else {
    Err(format!(
      "lab-core must depend on no workspace crate; found: {}",
      offenders.join(", ")
    ))
  }
}

/// The crate root is the most general position in `lab-core`; it may declare the
/// `report` child (`pub mod report;`) but must never reference it
/// (`crate::report::…`). A parent referencing its child inverts the dependency
/// direction the architecture is built on.
fn check_root_does_not_reference_report(root: &Path) -> Result<(), String> {
  let lib = root.join("crates/lab-core/src/lib.rs");
  let src =
    std::fs::read_to_string(&lib).map_err(|e| format!("could not read {}: {e}", lib.display()))?;

  let hits: Vec<String> = src
    .lines()
    .enumerate()
    .filter(|(_, line)| line.contains("crate::report"))
    .map(|(i, line)| format!("line {}: {}", i + 1, line.trim()))
    .collect();

  if hits.is_empty() {
    Ok(())
  } else {
    Err(format!(
      "the crate root must not reference crate::report (declaring `pub mod report;` is allowed): {}",
      hits.join("; ")
    ))
  }
}
