#![warn(warnings)]

extern crate env_logger;
extern crate exitcode;
extern crate stager;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate structopt;

#[cfg(feature = "serde_json")]
extern crate serde_json;
#[cfg(feature = "serde_yaml")]
extern crate serde_yaml;
#[cfg(feature = "toml")]
extern crate toml;

use std::ffi;
use std::fs;
use std::io::Write;
use std::io;
use std::io::Read;
use std::path;
use std::process;

use failure::ResultExt;
use structopt::StructOpt;

#[cfg(feature = "serde_yaml")]
fn load_yaml(path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    let f = fs::File::open(path)?;
    serde_yaml::from_reader(f).map_err(|e| e.into())
}

#[cfg(not(feature = "serde_yaml"))]
fn load_yaml(_path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    bail!("yaml is unsupported");
}

#[cfg(feature = "serde_json")]
fn load_json(path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    let f = fs::File::open(path)?;
    serde_json::from_reader(f).map_err(|e| e.into())
}

#[cfg(not(feature = "serde_json"))]
fn load_json(_path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    bail!("json is unsupported");
}

#[cfg(feature = "toml")]
fn load_toml(path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    let mut f = fs::File::open(path)?;
    let mut text = String::new();
    f.read_to_string(&mut text)?;
    toml::from_str(&text).map_err(|e| e.into())
}

#[cfg(not(feature = "toml"))]
fn load_toml(_path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    bail!("toml is unsupported");
}

fn load_stage(path: &path::Path) -> Result<stager::de::Staging, failure::Error> {
    let extension = path.extension().unwrap_or_default();
    let value = if extension == ffi::OsStr::new("yaml") {
        load_yaml(path)
    } else if extension == ffi::OsStr::new("toml") {
        load_toml(path)
    } else if extension == ffi::OsStr::new("json") {
        load_json(path)
    } else {
        bail!("Unsupported file type");
    }?;

    Ok(value)
}

#[derive(StructOpt, Debug)]
#[structopt(name = "staging")]
struct Arguments {
    #[structopt(short = "i", long = "input", name = "STAGE")] input: String,
    #[structopt(short = "o", long = "output", name = "DIR")] output: String,
    #[structopt(short = "n", long = "dry-run")] dry_run: bool,
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))] verbosity: u8,
}

fn run() -> Result<exitcode::ExitCode, failure::Error> {
    let mut builder = env_logger::Builder::new();
    let args = Arguments::from_args();
    let level = match args.verbosity {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    builder.filter(None, level);
    if level == log::LevelFilter::Trace {
        builder.default_format_timestamp(false);
    } else {
        builder.format(|f, record| {
            writeln!(
                f,
                "[{}] {}",
                record.level().to_string().to_lowercase(),
                record.args()
            )
        });
    }
    builder.init();

    let staging = load_stage(path::Path::new(&args.input))
        .with_context(|_| format!("Failed to load {:?}", args.input))?;
    let output_root = path::PathBuf::from(args.output);

    let staging: Result<Vec<_>, _> = staging
        .into_iter()
        .map(|(target, sources)| {
            let sources: Vec<stager::de::Source> = sources;
            let sources: Result<Vec<_>, _> = sources.into_iter().map(|s| s.format()).collect();
            sources.map(|s| (target, s))
        })
        .collect();
    // TODO(epage): Show all errors, not just first
    let staging = match staging {
        Ok(s) => s,
        Err(e) => {
            error!("Failed reading stage file: {}", e);
            return Ok(exitcode::DATAERR);
        }
    };

    let staging: Result<Vec<_>, _> = staging
        .into_iter()
        .map(|(target, sources)| {
            let target = output_root.join(target);
            let sources: Vec<Box<stager::builder::ActionBuilder>> = sources;
            let sources: Result<Vec<_>, _> =
                sources.into_iter().map(|s| s.build(&target)).collect();
            sources
        })
        .collect();
    // TODO(epage): Show all errors, not just first
    let staging = match staging {
        Ok(s) => s,
        Err(e) => {
            error!("Failed preparing staging: {}", e);
            return Ok(exitcode::IOERR);
        }
    };
    let staging: Vec<_> = staging
        .into_iter()
        .flat_map(|v| v.into_iter().flat_map(|v| v.into_iter()))
        .collect();

    for action in staging {
        debug!("{}", action);
        if !args.dry_run {
            action
                .perform()
                .with_context(|_| format!("Failed staging files: {}", action))?;
        }
    }

    Ok(exitcode::OK)
}

fn main() {
    let code = match run() {
        Ok(e) => e,
        Err(ref e) => {
            write!(&mut io::stderr(), "{}\n", e).expect("writing to stderr won't fail");
            exitcode::SOFTWARE
        }
    };
    process::exit(code);
}
