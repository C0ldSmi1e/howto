use std::collections::HashSet;
use std::path::Path;

pub struct MachineContext {
    pub os: String,
    pub arch: &'static str,
    pub shell: String,
    pub available: Vec<&'static str>,
    pub missing: Vec<&'static str>,
    pub markers: Vec<&'static str>,
}

const PROBES: &[&str] = &[
    "lsof", "ss", "fuser", "netstat", "curl", "wget", "jq", "fd", "rg", "fzf", "git", "docker",
    "kubectl", "brew", "apt-get", "dnf", "pacman", "systemctl", "launchctl", "tar", "zip",
    "unzip", "ffmpeg", "magick", "cwebp", "python3", "node", "npm", "cargo", "go", "gsed",
    "gawk",
];

const MARKERS: &[&str] = &[
    "Cargo.toml", "package.json", "pyproject.toml", "requirements.txt", "go.mod", "Gemfile",
    "pom.xml", "Makefile", "docker-compose.yml", "compose.yaml", ".git",
];

pub fn gather(shell_override: Option<&str>) -> MachineContext {
    let (shell, _) = shell_name(shell_override);
    let names = path_binaries();
    let mut available = Vec::new();
    let mut missing = Vec::new();
    for &probe in PROBES {
        if names.contains(probe) {
            available.push(probe);
        } else {
            missing.push(probe);
        }
    }
    let markers = MARKERS
        .iter()
        .copied()
        .filter(|m| Path::new(m).exists())
        .collect();
    MachineContext {
        os: detect_os(),
        arch: std::env::consts::ARCH,
        shell,
        available,
        missing,
        markers,
    }
}

pub fn shell_name(override_: Option<&str>) -> (String, &'static str) {
    if let Some(shell) = override_ {
        return (shell.to_string(), "config file");
    }
    let detected = std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(str::to_string))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "sh".into());
    (detected, "detected from $SHELL")
}

fn detect_os() -> String {
    if cfg!(target_os = "macos") {
        let version = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if version.is_empty() {
            "macOS".into()
        } else {
            format!("macOS {version}")
        }
    } else if cfg!(target_os = "linux") {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|raw| {
                raw.lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "Linux".into())
    } else {
        std::env::consts::OS.to_string()
    }
}

/// One pass over $PATH collecting entry names — probing is set lookups, not per-binary stats.
fn path_binaries() -> HashSet<String> {
    let mut names = HashSet::new();
    let Some(path) = std::env::var_os("PATH") else {
        return names;
    };
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                names.insert(name);
            }
        }
    }
    names
}

impl MachineContext {
    pub fn to_prompt(&self) -> String {
        let mut s = format!(
            "OS: {} ({})\nShell: {}\nAvailable tools: {}\nNot installed: {}",
            self.os,
            self.arch,
            self.shell,
            self.available.join(", "),
            self.missing.join(", "),
        );
        if !self.markers.is_empty() {
            s.push_str("\nCurrent directory contains: ");
            s.push_str(&self.markers.join(", "));
        }
        s
    }
}
