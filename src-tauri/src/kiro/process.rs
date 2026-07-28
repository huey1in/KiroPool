use crate::kiro::switch::KiroProcess;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct SystemKiroProcess {
    executable: PathBuf,
}

impl SystemKiroProcess {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }
}

impl KiroProcess for SystemKiroProcess {
    fn is_running(&self) -> bool {
        process_ids()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    }

    fn close(&self) -> Result<(), String> {
        let command_succeeded = if cfg!(target_os = "windows") {
            let output = hidden_command("taskkill")
                .args(["/F", "/T", "/IM", "Kiro.exe"])
                .output()
                .map_err(|error| format!("close Kiro processes: {error}"))?;
            output.status.success()
        } else {
            let mut succeeded = true;
            for pid in process_ids()? {
                let output = hidden_command("kill")
                    .args(["-9", &pid])
                    .output()
                    .map_err(|error| format!("close Kiro process {pid}: {error}"))?;
                succeeded &= output.status.success();
            }
            succeeded
        };
        for _ in 0..20 {
            if !self.is_running() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        close_command_result(command_succeeded, false)?;
        Err("Kiro process did not exit".to_string())
    }

    fn launch(&self) -> Result<(), String> {
        if !self.executable.is_file() {
            return Err(format!(
                "Kiro executable does not exist: {}",
                self.executable.display()
            ));
        }
        hidden_command(&self.executable.to_string_lossy())
            .spawn()
            .map_err(|error| format!("launch Kiro: {error}"))?;
        Ok(())
    }
}

fn close_command_result(command_succeeded: bool, processes_exited: bool) -> Result<(), String> {
    if command_succeeded || processes_exited {
        Ok(())
    } else {
        Err("close Kiro processes failed".to_string())
    }
}

pub fn discover_kiro_executable(configured: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = configured.filter(|path| path.is_file()) {
        return Ok(path.to_path_buf());
    }
    let mut candidates = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        candidates.push(local.join("Programs").join("Kiro").join("Kiro.exe"));
        candidates.push(local.join("Kiro").join("Kiro.exe"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("Kiro").join("Kiro.exe"));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| "KIRO_EXECUTABLE_NOT_FOUND".to_string())
}

fn process_ids() -> Result<Vec<String>, String> {
    let output = if cfg!(target_os = "windows") {
        hidden_command("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output()
    } else {
        hidden_command("ps")
            .args(["-A", "-o", "pid=,comm="])
            .output()
    }
    .map_err(|error| format!("list Kiro processes: {error}"))?;
    if !output.status.success() {
        return Err("list Kiro processes failed".to_string());
    }
    Ok(parse_process_ids(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_process_ids(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            if cfg!(target_os = "windows") {
                let fields: Vec<&str> = line.split(',').collect();
                let name = fields.first()?.trim_matches('"').to_ascii_lowercase();
                if name != "kiro.exe" {
                    return None;
                }
                fields.get(1).map(|pid| pid.trim_matches('"').to_string())
            } else {
                let mut fields = line.split_whitespace();
                let pid = fields.next()?;
                let name = fields.next()?.to_ascii_lowercase();
                (name == "kiro" || name == "kiro.exe").then(|| pid.to_string())
            }
        })
        .collect()
}

fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_close_failure_when_processes_already_exited() {
        assert_eq!(close_command_result(false, true), Ok(()));
    }

    #[test]
    fn reports_close_failure_when_processes_remain() {
        assert_eq!(
            close_command_result(false, false),
            Err("close Kiro processes failed".to_string())
        );
    }
}
