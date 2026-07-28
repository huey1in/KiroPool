use crate::kiro::switch::MachineIdStore;
use std::process::Command;

const MACHINE_GUID_KEY: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography";

pub struct WindowsMachineIdStore;

impl MachineIdStore for WindowsMachineIdStore {
    fn current(&self) -> Result<String, String> {
        let output = hidden_command("reg")
            .args(["query", MACHINE_GUID_KEY, "/v", "MachineGuid"])
            .output()
            .map_err(|error| format!("read Windows MachineGuid: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "read Windows MachineGuid failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        parse_machine_guid(&String::from_utf8_lossy(&output.stdout))
            .ok_or_else(|| "Windows MachineGuid was not found".to_string())
    }

    fn set(&self, value: &str) -> Result<(), String> {
        if !valid_machine_guid(value) {
            return Err("invalid MachineGuid format".to_string());
        }
        machine_guid_write_permission(crate::utils::check_admin_privileges()?)?;
        let output = hidden_command("reg")
            .args([
                "add",
                MACHINE_GUID_KEY,
                "/v",
                "MachineGuid",
                "/t",
                "REG_SZ",
                "/d",
                value,
                "/f",
            ])
            .output()
            .map_err(|error| format!("write Windows MachineGuid: {error}"))?;
        if !output.status.success() {
            let detail = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if output.status.code() == Some(5) {
                return Err("ADMIN_REQUIRED".to_string());
            }
            return Err(format!("write Windows MachineGuid failed: {detail}"));
        }
        Ok(())
    }
}

fn machine_guid_write_permission(has_admin: bool) -> Result<(), String> {
    if has_admin {
        Ok(())
    } else {
        Err("ADMIN_REQUIRED".to_string())
    }
}

fn parse_machine_guid(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let lower = line.to_lowercase();
        if !lower.contains("machineguid") || !lower.contains("reg_sz") {
            return None;
        }
        line.split_whitespace()
            .last()
            .filter(|value| valid_machine_guid(value))
            .map(|value| value.to_lowercase())
    })
}

fn valid_machine_guid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.chars().enumerate().all(|(index, character)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            character == '-'
        } else {
            character.is_ascii_hexdigit()
        }
    })
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
    fn write_requires_admin_before_running_reg() {
        assert_eq!(
            machine_guid_write_permission(false),
            Err("ADMIN_REQUIRED".to_string())
        );
        assert_eq!(machine_guid_write_permission(true), Ok(()));
    }

    #[test]
    fn parses_reg_query_output() {
        let output = r#"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography
    MachineGuid    REG_SZ    11111111-1111-4111-8111-111111111111"#;
        assert_eq!(
            parse_machine_guid(output).as_deref(),
            Some("11111111-1111-4111-8111-111111111111")
        );
    }
}
