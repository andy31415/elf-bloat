use eyre::{Result, WrapErr, eyre};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use which::which;

/// Columns shown by default in csvlens. Omits `Size1`/`Size2` which are rarely
/// useful and waste horizontal space; `Function`, `Type`, and `Size` cover most
/// review workflows.
pub const CSVLENS_DEFAULT_COLUMNS: &str = "Function|Size$|Type";

/// The viewer tool to pipe CSV output to.
#[derive(Debug, PartialEq)]
pub enum ViewerTool {
    /// Auto-detect: prefer `vd`, then `csvlens`, then plain table output.
    Default,
    Visidata,
    Csvlens,
    /// Pipe to an arbitrary program (with optional args) that reads CSV from stdin.
    Custom(Vec<String>),
}

impl std::str::FromStr for ViewerTool {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "default" => Ok(Self::Default),
            "vd" | "visidata" => Ok(Self::Visidata),
            "csvlens" => Ok(Self::Csvlens),
            s if s.starts_with("custom:") => {
                let rest = s.trim_start_matches("custom:");
                let parts: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
                if parts.is_empty() {
                    Err(eyre!(
                        r#"custom: viewer requires a program name, e.g. custom:myviewer or custom:"grep chip""#
                    ))
                } else {
                    Ok(Self::Custom(parts))
                }
            }
            other => Err(eyre!(
                "Unknown viewer '{}'. Valid options: default, vd, visidata, csvlens, custom:<name>",
                other
            )),
        }
    }
}

impl ViewerTool {
    /// Resolve `Default` to a concrete choice based on what is installed.
    pub fn resolve(&self) -> ResolvedViewer {
        match self {
            Self::Default => {
                if which("vd").is_ok() {
                    ResolvedViewer::Visidata
                } else if which("csvlens").is_ok() {
                    ResolvedViewer::Csvlens
                } else {
                    ResolvedViewer::Table
                }
            }
            Self::Visidata => ResolvedViewer::Visidata,
            Self::Csvlens => ResolvedViewer::Csvlens,
            Self::Custom(parts) => ResolvedViewer::Custom(parts.clone()),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ResolvedViewer {
    Table,
    Visidata,
    Csvlens,
    Custom(Vec<String>),
}

pub fn pipe_to_viewer(input: &[u8], workdir: &Path, viewer: &ViewerTool) -> Result<()> {
    let resolved = viewer.resolve();
    let command = match resolved {
        ResolvedViewer::Visidata => {
            let mut cmd = Command::new("vd");
            cmd.current_dir(workdir).arg("-");
            Some(cmd)
        }
        ResolvedViewer::Csvlens => {
            let mut cmd = Command::new("csvlens");
            cmd.current_dir(workdir)
                .args(["--columns", CSVLENS_DEFAULT_COLUMNS]);
            Some(cmd)
        }
        ResolvedViewer::Custom(parts) => {
            let mut cmd = Command::new(&parts[0]);
            cmd.args(&parts[1..]).current_dir(workdir);
            Some(cmd)
        }
        ResolvedViewer::Table => {
            println!("{}", String::from_utf8_lossy(input));
            return Ok(());
        }
    };

    if let Some(mut command) = command {
        use std::io::Write;
        let mut child = command.stdin(std::process::Stdio::piped()).spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input)?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(eyre!(
                "Viewer command failed with status {}:
STDOUT: {}
STDERR: {}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    } else {
        Ok(())
    }
}

/// A chain of commands piped together: stdout of each feeds stdin of the next.
#[derive(Debug)]
pub struct CommandChain {
    commands: Vec<Command>,
}

impl CommandChain {
    pub fn new(initial_command: Command) -> Self {
        CommandChain {
            commands: vec![initial_command],
        }
    }

    pub fn pipe(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    pub fn execute(mut self) -> Result<()> {
        let n = self.commands.len();
        if n == 0 {
            return Ok(());
        }

        let mut previous_child: Option<Child> = None;
        let mut intermediate_children: Vec<Child> = Vec::new();

        for (i, command) in self.commands.iter_mut().enumerate() {
            if let Some(mut child) = previous_child.take() {
                command.stdin(Stdio::from(child.stdout.take().unwrap()));
                intermediate_children.push(child);
            }

            if i == n - 1 {
                command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
                let status = command.status().wrap_err("Failed to execute command")?;
                if !status.success() {
                    return Err(eyre!("Command failed with status: {}", status));
                }
            } else {
                command.stdout(Stdio::piped()).stderr(Stdio::inherit());
                previous_child = Some(command.spawn().wrap_err("Failed to start command")?);
            }
        }

        for mut child in intermediate_children {
            let status = child
                .wait()
                .wrap_err("Failed to wait on intermediate command")?;
            if !status.success() {
                return Err(eyre!("Intermediate command failed with status: {}", status));
            }
        }

        Ok(())
    }
}

pub fn build_viewer_chain(
    mut diff_cmd: Command,
    from_path: &Path,
    to_path: &Path,
    workdir: &Path,
    viewer: &ViewerTool,
) -> CommandChain {
    let resolved = viewer.resolve();
    let output_format = if matches!(resolved, ResolvedViewer::Table) {
        "table"
    } else {
        "csv"
    };
    diff_cmd
        .args(["--output", output_format])
        .arg(to_path)
        .arg(from_path);

    match resolved {
        ResolvedViewer::Visidata => {
            let mut vd = Command::new("vd");
            vd.current_dir(workdir).arg("-");
            CommandChain::new(diff_cmd).pipe(vd)
        }
        ResolvedViewer::Csvlens => {
            let mut csvlens = Command::new("csvlens");
            csvlens
                .current_dir(workdir)
                .args(["--columns", CSVLENS_DEFAULT_COLUMNS]);
            CommandChain::new(diff_cmd).pipe(csvlens)
        }
        ResolvedViewer::Custom(parts) => {
            let mut custom = Command::new(&parts[0]);
            custom.args(&parts[1..]).current_dir(workdir);
            CommandChain::new(diff_cmd).pipe(custom)
        }
        ResolvedViewer::Table => CommandChain::new(diff_cmd),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_known_variants() {
        assert_eq!(
            "default".parse::<ViewerTool>().unwrap(),
            ViewerTool::Default
        );
        assert_eq!("vd".parse::<ViewerTool>().unwrap(), ViewerTool::Visidata);
        assert_eq!(
            "visidata".parse::<ViewerTool>().unwrap(),
            ViewerTool::Visidata
        );
        assert_eq!(
            "csvlens".parse::<ViewerTool>().unwrap(),
            ViewerTool::Csvlens
        );
    }

    #[test]
    fn test_parse_custom_single_arg() {
        assert_eq!(
            "custom:myviewer".parse::<ViewerTool>().unwrap(),
            ViewerTool::Custom(vec!["myviewer".to_string()]),
        );
    }

    #[test]
    fn test_parse_custom_multi_arg() {
        assert_eq!(
            "custom:grep chip".parse::<ViewerTool>().unwrap(),
            ViewerTool::Custom(vec!["grep".to_string(), "chip".to_string()]),
        );
    }

    #[test]
    fn test_parse_custom_extra_whitespace() {
        assert_eq!(
            "custom:  grep   -i  foo  ".parse::<ViewerTool>().unwrap(),
            ViewerTool::Custom(vec![
                "grep".to_string(),
                "-i".to_string(),
                "foo".to_string()
            ]),
        );
    }

    #[test]
    fn test_parse_custom_empty_is_error() {
        assert!("custom:".parse::<ViewerTool>().is_err());
        assert!("custom:   ".parse::<ViewerTool>().is_err());
    }

    #[test]
    fn test_parse_unknown_is_error() {
        assert!("".parse::<ViewerTool>().is_err());
        assert!("foobar".parse::<ViewerTool>().is_err());
        assert!("Custom:foo".parse::<ViewerTool>().is_err());
    }

    #[test]
    fn test_resolve_visidata() {
        assert_eq!(ViewerTool::Visidata.resolve(), ResolvedViewer::Visidata);
    }

    #[test]
    fn test_resolve_csvlens() {
        assert_eq!(ViewerTool::Csvlens.resolve(), ResolvedViewer::Csvlens);
    }

    #[test]
    fn test_resolve_custom_preserves_args() {
        let parts = vec!["grep".to_string(), "chip".to_string()];
        assert_eq!(
            ViewerTool::Custom(parts.clone()).resolve(),
            ResolvedViewer::Custom(parts),
        );
    }

    #[test]
    fn test_resolve_default_returns_valid_variant() {
        let resolved = ViewerTool::Default.resolve();
        assert!(matches!(
            resolved,
            ResolvedViewer::Visidata | ResolvedViewer::Csvlens | ResolvedViewer::Table
        ));
    }
}
