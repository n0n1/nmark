use std::env;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use crate::inputs::looks_like_url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOptions {
    pub source: Option<SourceArg>,
    pub output_dir: Option<PathBuf>,
    pub write_to_stdout: Option<bool>,
    pub include_frontmatter: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceArg {
    Url(String),
    Input(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Run(RunOptions),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    MissingValue(&'static str),
    UnknownOption(String),
    MultipleSources,
}

impl Command {
    pub fn parse() -> Result<Self, CliError> {
        Self::parse_from(env::args())
    }

    pub fn parse_from<I, T>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        let _binary = args.next();

        let mut source = None;
        let mut output_dir = None;
        let mut write_to_stdout = None;
        let mut include_frontmatter = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--url" => {
                    let value = args
                        .next()
                        .ok_or(CliError::MissingValue("--url"))?;
                    set_source(&mut source, SourceArg::Url(value))?;
                }
                "--input" => {
                    let value = args
                        .next()
                        .ok_or(CliError::MissingValue("--input"))?;
                    set_source(&mut source, SourceArg::Input(PathBuf::from(value)))?;
                }
                "--output-dir" => {
                    let value = args.next().ok_or(CliError::MissingValue("--output-dir"))?;
                    output_dir = Some(PathBuf::from(value));
                }
                "--stdout" => write_to_stdout = Some(true),
                "--no-frontmatter" => include_frontmatter = Some(false),
                "--help" | "-h" => return Ok(Command::Help),
                _ if arg.starts_with('-') => {
                    return Err(CliError::UnknownOption(arg));
                }
                _ => {
                    set_source(&mut source, parse_positional_source(arg))?;
                }
            }
        }

        Ok(Command::Run(RunOptions {
            source,
            output_dir,
            write_to_stdout,
            include_frontmatter,
        }))
    }

    pub fn usage() -> &'static str {
        concat!(
            "Usage:\n",
            "  nmark [<url-or-input>] [--url <url> | --input <path>] [--output-dir <dir>] [--stdout] [--no-frontmatter]\n",
            "\n",
            "Options:\n",
            "  --url <url>           Source article URL\n",
            "  --input <path>        Path to .txt or .toml file with URL list\n",
            "  --output-dir <dir>    Directory for saved markdown file (default: current directory)\n",
            "  --stdout              Print markdown to stdout instead of writing a file\n",
            "  --no-frontmatter      Skip YAML frontmatter\n",
            "  -h, --help            Show this help message\n",
            "\n",
            "Examples:\n",
            "  nmark https://example.com/article\n",
            "  nmark --input urls.txt\n",
            "  nmark --input download.toml\n",
            "\n",
            "download.toml examples:\n",
            "  urls = [\"https://example.com/a\", \"https://example.com/b\"]\n",
            "\n",
            "  [download]\n",
            "  urls = [\"https://example.com/a\", \"https://example.com/b\"]\n",
            "\n",
            "config.toml examples:\n",
            "  output_dir = \"articles\"\n",
            "  stdout = false\n",
            "  include_frontmatter = true\n",
        )
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::MissingValue(flag) => {
                write!(f, "{flag} requires a value. See --help for usage.")
            }
            CliError::UnknownOption(option) => {
                write!(f, "unknown option `{option}`. See --help for usage.")
            }
            CliError::MultipleSources => {
                f.write_str("multiple input sources provided. See --help for usage.")
            }
        }
    }
}

impl Error for CliError {}

fn set_source(target: &mut Option<SourceArg>, value: SourceArg) -> Result<(), CliError> {
    if target.is_some() {
        return Err(CliError::MultipleSources);
    }
    *target = Some(value);
    Ok(())
}

fn parse_positional_source(value: String) -> SourceArg {
    if looks_like_url(&value) {
        SourceArg::Url(value)
    } else {
        SourceArg::Input(PathBuf::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::{CliError, Command, RunOptions, SourceArg};
    use std::path::PathBuf;

    #[test]
    fn parses_positional_url() {
        let command = Command::parse_from(["nmark", "https://example.com"]).unwrap();
        assert_eq!(
            command,
            Command::Run(RunOptions {
                source: Some(SourceArg::Url("https://example.com".into())),
                output_dir: None,
                write_to_stdout: None,
                include_frontmatter: None,
            })
        );
    }

    #[test]
    fn parses_named_url_and_flags() {
        let command = Command::parse_from([
            "nmark",
            "--url",
            "https://example.com",
            "--output-dir",
            "out",
            "--stdout",
            "--no-frontmatter",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Run(RunOptions {
                source: Some(SourceArg::Url("https://example.com".into())),
                output_dir: Some(PathBuf::from("out")),
                write_to_stdout: Some(true),
                include_frontmatter: Some(false),
            })
        );
    }

    #[test]
    fn parses_input_file() {
        let command = Command::parse_from(["nmark", "download.toml"]).unwrap();
        assert_eq!(
            command,
            Command::Run(RunOptions {
                source: Some(SourceArg::Input(PathBuf::from("download.toml"))),
                output_dir: None,
                write_to_stdout: None,
                include_frontmatter: None,
            })
        );
    }

    #[test]
    fn parses_help() {
        let command = Command::parse_from(["nmark", "--help"]).unwrap();
        assert_eq!(command, Command::Help);
    }

    #[test]
    fn rejects_multiple_sources() {
        let error = Command::parse_from(["nmark", "https://a", "https://b"]).unwrap_err();
        assert_eq!(error, CliError::MultipleSources);
    }

    #[test]
    fn rejects_unknown_option() {
        let error = Command::parse_from(["nmark", "--wat"]).unwrap_err();
        assert_eq!(error, CliError::UnknownOption("--wat".into()));
    }

    #[test]
    fn rejects_missing_option_value() {
        let error = Command::parse_from(["nmark", "--url"]).unwrap_err();
        assert_eq!(error, CliError::MissingValue("--url"));
    }
}
