use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    os::unix::net::UnixStream,
    thread,
    time::Duration,
};

use log::{error, warn};
use serde::{Deserialize, Serialize};
use zygisk_api::api::{V4, ZygiskApi};

use crate::state::{ACTIVE_RESET_SESSION, ActiveResetSession};

pub fn spoof_system_props_via_companion(
    api: &mut ZygiskApi<V4>,
    prop_map: &HashMap<String, String>,
    delete_props: &[String],
    package_name: &str,
) -> anyhow::Result<()> {
    if prop_map.is_empty() && delete_props.is_empty() {
        return Ok(());
    }

    let request = CompanionRequest::Apply(ResetpropSessionRequest {
        pid: std::process::id(),
        props: prop_map.clone(),
        delete_props: delete_props.to_vec(),
    });

    let response = send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion resetprop failed".to_string())
        );
    }

    if let Some(backups) = response.backups {
        *ACTIVE_RESET_SESSION.lock().unwrap() = Some(ActiveResetSession {
            package: package_name.to_string(),
            backups,
        });
    } else {
        warn!("Companion did not return property backups; automatic restore may be skipped");
    }

    Ok(())
}

pub fn restore_previous_resetprop_if_needed(
    api: &mut ZygiskApi<V4>,
    current_package: &str,
) -> anyhow::Result<()> {
    let mut guard = ACTIVE_RESET_SESSION.lock().unwrap();
    let pending = guard.take();

    match pending {
        Some(session) if session.package != current_package => {
            if let Err(e) = restore_props_via_companion(api, &session.backups) {
                error!("Failed to restore previous resetprop session: {e}");
            }
        }
        other => {
            *guard = other;
        }
    }

    Ok(())
}

fn restore_props_via_companion(
    api: &mut ZygiskApi<V4>,
    backups: &HashMap<String, String>,
) -> anyhow::Result<()> {
    if backups.is_empty() {
        return Ok(());
    }

    let request = CompanionRequest::Restore(RestoreRequest {
        props: backups.clone(),
    });

    let response = send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion restore failed".to_string())
        );
    }

    Ok(())
}

fn send_companion_command(
    api: &mut ZygiskApi<V4>,
    request: &CompanionRequest,
) -> anyhow::Result<CompanionResponse> {
    let payload = serde_json::to_vec(request)?;
    let response = api
        .with_companion(|stream| -> anyhow::Result<CompanionResponse> {
            stream.write_all(&(payload.len() as u32).to_le_bytes())?;
            stream.write_all(&payload)?;
            stream.flush()?;

            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf)?;
            let resp_len = u32::from_le_bytes(len_buf) as usize;
            let mut resp_buf = vec![0u8; resp_len];
            stream.read_exact(&mut resp_buf)?;

            let resp = serde_json::from_slice::<CompanionResponse>(&resp_buf)?;
            Ok(resp)
        })
        .map_err(|e| anyhow::anyhow!("Failed to talk to companion: {e}"))??;

    Ok(response)
}

pub fn handle_companion_request(stream: &mut UnixStream) {
    let response = match read_companion_request(stream) {
        Ok(CompanionRequest::Apply(request)) => match apply_resetprop_session(request) {
            Ok(backups) => CompanionResponse::ok_with_backups(backups),
            Err(err) => {
                error!("Companion failed to apply resetprop session: {err}");
                CompanionResponse::err(err.to_string())
            }
        },
        Ok(CompanionRequest::Restore(request)) => match restore_properties(request) {
            Ok(_) => CompanionResponse::ok(),
            Err(err) => {
                error!("Companion failed to restore properties: {err}");
                CompanionResponse::err(err.to_string())
            }
        },
        Err(err) => {
            error!("Companion failed to parse request: {err}");
            CompanionResponse::err("invalid request")
        }
    };

    if let Err(e) = write_companion_response(stream, &response) {
        warn!("Failed to write companion response: {e}");
    }
}

fn read_companion_request(stream: &mut UnixStream) -> anyhow::Result<CompanionRequest> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let payload_len = u32::from_le_bytes(len_buf) as usize;
    if payload_len == 0 {
        anyhow::bail!("empty request payload");
    }

    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;
    let request = serde_json::from_slice::<CompanionRequest>(&payload)?;
    Ok(request)
}

fn write_companion_response(
    stream: &mut UnixStream,
    response: &CompanionResponse,
) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(response)?;
    stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

fn apply_resetprop_session(
    request: ResetpropSessionRequest,
) -> anyhow::Result<HashMap<String, String>> {
    if request.props.is_empty() && request.delete_props.is_empty() {
        return Ok(HashMap::new());
    }

    let resetprop_path = find_resetprop_path()
        .ok_or_else(|| anyhow::anyhow!("resetprop binary not found in known locations"))?;

    let mut backups = Vec::with_capacity(request.props.len() + request.delete_props.len());

    for key in request.props.keys() {
        let original = backup_property(key)?;
        backups.push(PropBackup {
            key: key.clone(),
            original_value: original,
        });
    }

    for key in &request.delete_props {
        let original = backup_property(key)?;
        backups.push(PropBackup {
            key: key.clone(),
            original_value: original,
        });
    }

    let backups_for_response: HashMap<String, String> = backups
        .iter()
        .map(|entry| (entry.key.clone(), entry.original_value.clone()))
        .collect();

    for (key, value) in &request.props {
        apply_resetprop(&resetprop_path, key, value)?;
    }

    for key in &request.delete_props {
        resetprop_delete(&resetprop_path, key)?;
    }

    spawn_restore_watcher(request.pid, backups, resetprop_path)?;

    Ok(backups_for_response)
}

fn restore_properties(request: RestoreRequest) -> anyhow::Result<()> {
    if request.props.is_empty() {
        return Ok(());
    }

    let resetprop_path = find_resetprop_path()
        .ok_or_else(|| anyhow::anyhow!("resetprop binary not found in known locations"))?;

    for (key, value) in request.props {
        apply_resetprop(&resetprop_path, &key, &value)?;
    }

    Ok(())
}

fn backup_property(key: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("getprop").arg(key).output()?;
    if !output.status.success() {
        anyhow::bail!("getprop failed for {key}");
    }

    let value = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Ok(value)
}

fn apply_resetprop(path: &str, key: &str, value: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new(path)
        .arg(key)
        .arg(value)
        .status()?;
    if !status.success() {
        anyhow::bail!("resetprop failed for {key}");
    }
    Ok(())
}

fn resetprop_delete(path: &str, key: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new(path)
        .arg("--delete")
        .arg(key)
        .status()?;
    if !status.success() {
        anyhow::bail!("resetprop delete failed for {key}");
    }
    Ok(())
}

fn spawn_restore_watcher(
    pid: u32,
    backups: Vec<PropBackup>,
    resetprop_path: String,
) -> anyhow::Result<()> {
    unsafe {
        match libc::fork() {
            -1 => anyhow::bail!("fork failed: {}", std::io::Error::last_os_error()),
            0 => {
                if libc::setsid() == -1 {
                    libc::_exit(1);
                }
                wait_for_process_inactive(pid);
                for entry in backups {
                    if let Err(e) =
                        apply_resetprop(&resetprop_path, &entry.key, &entry.original_value)
                    {
                        error!(
                            "Failed to restore property {} for pid {}: {}",
                            entry.key, pid, e
                        );
                    }
                }
                libc::_exit(0);
            }
            _ => Ok(()),
        }
    }
}

fn wait_for_process_inactive(pid: u32) {
    let proc_path = format!("/proc/{pid}");
    loop {
        if !std::path::Path::new(&proc_path).exists() {
            break;
        }

        if !is_process_in_top_app(pid) {
            break;
        }

        thread::sleep(Duration::from_millis(500));
    }
}

fn is_process_in_top_app(pid: u32) -> bool {
    let cgroup_path = format!("/proc/{pid}/cgroup");
    match fs::read_to_string(&cgroup_path) {
        Ok(content) => content.lines().any(|line| line.contains("top-app")),
        Err(_) => true,
    }
}

fn find_resetprop_path() -> Option<String> {
    let possible_paths = [
        "/data/adb/ksu/bin/resetprop",
        "/data/adb/magisk/resetprop",
        "/debug_ramdisk/resetprop",
        "/data/adb/ap/bin/resetprop",
        "/system/bin/resetprop",
        "/vendor/bin/resetprop",
    ];

    for path in possible_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    std::process::Command::new("which")
        .arg("resetprop")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty() && std::path::Path::new(path).exists())
}

#[derive(Serialize, Deserialize, Debug)]
struct ResetpropSessionRequest {
    pid: u32,
    props: HashMap<String, String>,
    delete_props: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RestoreRequest {
    props: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd", content = "payload")]
enum CompanionRequest {
    Apply(ResetpropSessionRequest),
    Restore(RestoreRequest),
}

#[derive(Serialize, Deserialize, Debug)]
struct CompanionResponse {
    status: i32,
    message: Option<String>,
    backups: Option<HashMap<String, String>>,
}

impl CompanionResponse {
    fn ok() -> Self {
        Self {
            status: 0,
            message: None,
            backups: None,
        }
    }

    fn err(msg: impl Into<String>) -> Self {
        Self {
            status: -1,
            message: Some(msg.into()),
            backups: None,
        }
    }

    fn ok_with_backups(backups: HashMap<String, String>) -> Self {
        Self {
            status: 0,
            message: None,
            backups: Some(backups),
        }
    }
}

#[derive(Clone)]
struct PropBackup {
    key: String,
    original_value: String,
}
