use anyhow::{anyhow, Context, Result};
use clap::Parser;
use log::info;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};
use tempfile::tempdir;
//TODO: add logging

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, trailing_var_arg=true)]
struct Args {
    #[clap(short, long, value_parser, name = "crate")]
    install_crate: String,
    #[clap(
        long,
        multiple_values = true,
        allow_hyphen_values = true,
        required = false,
        help = "Arguments to pass to the executable being tried"
    )]
    sub_args: Vec<String>,
}

fn create_crate_install_command(install_crate: &str, path: &Path) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("install")
        .arg(install_crate)
        .arg("--root")
        .arg(path);
    cmd
}

fn find_first_executable(crate_name: &str, search_dir: &Path) -> Result<PathBuf> {
    info!(
        "Searching for {}in: {}",
        crate_name,
        search_dir.to_string_lossy()
    );
    for path in fs::read_dir(search_dir)? {
        let path = path.unwrap();
        let file_name = path
            .path()
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        if file_name == crate_name {
            return Ok(path.path());
        }
    }
    Err(anyhow!("Could not find crate with name {}", crate_name))
}

fn valid_crate_name(name: &str) -> bool {
    // From crates.io
    let mut chars = name.chars();
    let first = match chars.next() {
        None => return false,
        Some(c) => c,
    };
    first.is_ascii_alphanumeric()
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn main_body(args: &Args) -> Result<ExitStatus> {
    if !valid_crate_name(&args.install_crate) {
        println!("Invalid crate name provided");
        return Err(anyhow!("Invalid crate name provided"));
    }

    let dir = tempdir()?;

    info!("Installing to: {}", dir.path().display());

    let output = create_crate_install_command(&args.install_crate, dir.path())
        .status()
        .context("Failed to execute cargo install")?;

    if !output.success() {
        return Err(anyhow!(
            "Failed to install, returned with status code: {}",
            output.code().unwrap_or_default()
        ));
    }

    let exec_file = find_first_executable(&args.install_crate, dir.path().join("bin").as_path())
        .context("Failed to find same-named executable after install")?;
    info!(
        "Found executable matching install name: {}",
        &args.install_crate
    );
    let cwd = dir.path().join("cwd");

    info!("Creating CWD dir at: {}", cwd.display());
    fs::create_dir(&cwd)?;

    let mut inner_cmd = std::process::Command::new(exec_file);
    inner_cmd.current_dir(&cwd);

    if args.sub_args.is_empty() {
        info!("Running {}", &args.install_crate);
    } else {
        info!(
            "Running {} with args {}",
            &args.install_crate,
            &args.sub_args.join(" ")
        );
    }

    let inner_status = inner_cmd.args(&args.sub_args).status()?;

    info!(
        "Exited with status code: {}",
        inner_status.code().unwrap_or_default()
    );
    Ok(inner_status)
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    main_body(&args).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
    #[test]
    fn check_install() -> Result<()> {
        init();
        let args = Args {
            install_crate: "status-return".into(),
            sub_args: vec![],
        };
        assert!(main_body(&args)?.code().unwrap() == 42);
        Ok(())
    }

    #[test]
    fn check_install_with_sub_args() -> Result<()> {
        init();
        let args = Args {
            install_crate: "status-return".into(),
            sub_args: vec!["99".into()],
        };
        assert!(main_body(&args)?.code().unwrap() == 99);
        Ok(())
    }
}
