//! minive — manage miniVE environments entirely from a terminal.
//!
//! Reuses the same Docker cores as the GUI (`create_env_core`,
//! `delete_env_core`) and the same registry file, so environments created
//! here show up in the app and vice versa. Interactive shells go through
//! the `docker` CLI (`docker exec -it`), which is guaranteed present since
//! Docker Desktop is a miniVE prerequisite.

use bollard::container::StartContainerOptions;
use bollard::Docker;
use minive_lib::env_manager::{create_env_core, ctr_name, delete_env_core, list_docker_envs, CreateEnvSpec};
use minive_lib::registry::{EnvEntry, PortMap, Registry};
use minive_lib::runtime_catalog::{install_command, pkg_manager_for_image, PackagePreset};
use std::path::PathBuf;
use std::process::Command;

const USAGE: &str = "\
minive — disposable Linux dev environments from your terminal

USAGE:
  minive list
  minive create <name> [--image <image>] [--port HOST:CONTAINER]... [--preset none|minimal|full]
  minive start <name>
  minive stop <name>
  minive delete <name>
  minive shell <name>

DEFAULTS:
  --image ubuntu:24.04   --preset minimal (git + curl)

EXAMPLES:
  minive create py --image python:3.12 --port 8000:8000
  minive shell py
";

/// Same directory Tauri's `app_data_dir` resolves for identifier
/// com.sahil.minive — both frontends must read/write one registry file.
fn registry_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var("APPDATA").expect("APPDATA not set"));
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var("HOME").expect("HOME not set")).join("Library/Application Support");
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = match std::env::var("XDG_DATA_HOME") {
        Ok(x) if !x.is_empty() => PathBuf::from(x),
        _ => PathBuf::from(std::env::var("HOME").expect("HOME not set")).join(".local/share"),
    };
    base.join("com.sahil.minive").join("registry.json")
}

struct CreateArgs {
    name: String,
    image: String,
    ports: Vec<PortMap>,
    preset: PackagePreset,
}

fn parse_port(s: &str) -> Result<PortMap, String> {
    let (h, c) = s.split_once(':').ok_or(format!("Invalid --port '{s}', expected HOST:CONTAINER"))?;
    Ok(PortMap {
        host: h.parse().map_err(|_| format!("Invalid host port '{h}'"))?,
        container: c.parse().map_err(|_| format!("Invalid container port '{c}'"))?,
    })
}

fn parse_create(args: &[String]) -> Result<CreateArgs, String> {
    let mut it = args.iter();
    let name = it.next().ok_or("create needs a <name>")?.clone();
    if name.starts_with('-') {
        return Err("create needs a <name> before flags".into());
    }
    let mut image = "ubuntu:24.04".to_string();
    let mut ports = Vec::new();
    let mut preset = PackagePreset::Minimal;
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--image" => image = val()?.clone(),
            "--port" => ports.push(parse_port(val()?)?),
            "--preset" => {
                preset = match val()?.as_str() {
                    "none" => PackagePreset::None,
                    "minimal" => PackagePreset::Minimal,
                    "full" => PackagePreset::Full,
                    other => return Err(format!("Unknown preset '{other}' (none|minimal|full)")),
                }
            }
            other => return Err(format!("Unknown flag '{other}'")),
        }
    }
    Ok(CreateArgs { name, image, ports, preset })
}

fn docker() -> Result<Docker, String> {
    Docker::connect_with_local_defaults().map_err(|e| e.to_string())
}

async fn cmd_list() -> Result<(), String> {
    let containers = list_docker_envs(&docker()?).await?;
    if containers.is_empty() {
        println!("No environments. Create one: minive create <name>");
        return Ok(());
    }
    println!("{:<24} {:<28} STATUS", "NAME", "IMAGE");
    for c in containers {
        println!("{:<24} {:<28} {}", c.env_name, c.image, if c.running { "running" } else { "stopped" });
    }
    Ok(())
}

async fn cmd_create(a: CreateArgs) -> Result<(), String> {
    create_env_core(&docker()?, &CreateEnvSpec {
        name: a.name.clone(),
        image: a.image.clone(),
        ports: a.ports.clone(),
        preset: a.preset,
    }, &mut |s| {
        // \r + clear-line keeps layer-by-layer pull progress on one line
        print!("\r\x1b[K{s}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })
    .await?;
    println!();

    if let Some(cmd) = install_command(a.preset, pkg_manager_for_image(&a.image)) {
        println!("Installing package preset...");
        let status = Command::new("docker")
            .arg("exec").arg(ctr_name(&a.name)).args(&cmd)
            .status()
            .map_err(|e| format!("docker exec failed: {e}"))?;
        if !status.success() {
            eprintln!("[minive] package preset install failed (non-fatal)");
        }
    }

    // Best-effort ports metadata for the GUI; if the app is running it will
    // adopt the container from its Docker label anyway.
    Registry::load(registry_path()).upsert(EnvEntry { name: a.name.clone(), image: a.image, ports: a.ports });

    println!("Created '{}'. Open a shell: minive shell {}", a.name, a.name);
    Ok(())
}

fn cmd_shell(name: &str) -> Result<i32, String> {
    let status = Command::new("docker")
        .args(["exec", "-it", "-w", "/workspace", &ctr_name(name), "sh", "-c",
               "command -v bash >/dev/null 2>&1 && exec bash || exec sh"])
        .status()
        .map_err(|e| format!("docker exec failed: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

async fn run(args: Vec<String>) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        Some("list") => cmd_list().await.map(|_| 0),
        Some("create") => cmd_create(parse_create(&args[1..])?).await.map(|_| 0),
        Some("start") => {
            let name = args.get(1).ok_or("start needs a <name>")?;
            docker()?
                .start_container(&ctr_name(name), None::<StartContainerOptions<String>>)
                .await
                .map_err(|e| e.to_string())?;
            println!("Started '{name}'.");
            Ok(0)
        }
        Some("stop") => {
            let name = args.get(1).ok_or("stop needs a <name>")?;
            docker()?.stop_container(&ctr_name(name), None).await.map_err(|e| e.to_string())?;
            println!("Stopped '{name}'.");
            Ok(0)
        }
        Some("delete") => {
            let name = args.get(1).ok_or("delete needs a <name>")?;
            delete_env_core(&docker()?, name).await;
            Registry::load(registry_path()).remove(name);
            println!("Deleted '{name}' (container + volume).");
            Ok(0)
        }
        Some("shell") => cmd_shell(args.get(1).ok_or("shell needs a <name>")?),
        Some("--help" | "-h" | "help") | None => {
            print!("{USAGE}");
            Ok(0)
        }
        Some(other) => Err(format!("Unknown command '{other}'\n\n{USAGE}")),
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(args).await {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn create_defaults() {
        let a = parse_create(&v(&["py"])).unwrap();
        assert_eq!(a.image, "ubuntu:24.04");
        assert_eq!(a.preset, PackagePreset::Minimal);
        assert!(a.ports.is_empty());
    }

    #[test]
    fn create_full_flags() {
        let a = parse_create(&v(&["py", "--image", "python:3.12", "--port", "8000:80", "--preset", "full"])).unwrap();
        assert_eq!(a.image, "python:3.12");
        assert_eq!(a.ports, vec![PortMap { host: 8000, container: 80 }]);
        assert_eq!(a.preset, PackagePreset::Full);
    }

    #[test]
    fn create_rejects_bad_input() {
        assert!(parse_create(&v(&[])).is_err());
        assert!(parse_create(&v(&["--image", "x"])).is_err());
        assert!(parse_create(&v(&["py", "--port", "8000"])).is_err());
        assert!(parse_create(&v(&["py", "--preset", "mega"])).is_err());
        assert!(parse_create(&v(&["py", "--image"])).is_err());
    }
}
