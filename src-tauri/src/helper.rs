#[cfg(target_os = "macos")]
use std::{
    env, fs,
    io::{Read, Write},
    os::unix::{
        fs::PermissionsExt,
        io::AsRawFd,
        net::{UnixListener, UnixStream},
    },
    path::Path,
    process::Command,
    thread,
    time::Duration,
};

#[cfg(target_os = "macos")]
const HELPER_LABEL: &str = "com.lovstudio.mactoolkits.helper";
#[cfg(target_os = "macos")]
const HELPER_PATH: &str = "/Library/PrivilegedHelperTools/com.lovstudio.mactoolkits.helper";
#[cfg(target_os = "macos")]
const HELPER_PLIST_PATH: &str = "/Library/LaunchDaemons/com.lovstudio.mactoolkits.helper.plist";
#[cfg(target_os = "macos")]
const HELPER_SOCKET_PATH: &str = "/var/run/com.lovstudio.mactoolkits.helper.sock";
#[cfg(target_os = "macos")]
const HELPER_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-helper.3");

#[cfg(target_os = "macos")]
pub fn run() -> Result<(), String> {
    let allowed_uid = allowed_uid_from_args()?;
    if unsafe { libc::geteuid() } != 0 {
        return Err("Helper must run as root".to_string());
    }

    if Path::new(HELPER_SOCKET_PATH).exists() {
        let _ = fs::remove_file(HELPER_SOCKET_PATH);
    }

    let listener = UnixListener::bind(HELPER_SOCKET_PATH)
        .map_err(|error| format!("Bind helper socket: {error}"))?;
    secure_socket(allowed_uid)?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if !peer_is_allowed(&stream, allowed_uid) {
                    let _ = write_response(&mut stream, "err permission denied\n");
                    continue;
                }

                if let Err(error) = handle_request(&mut stream) {
                    let _ = write_response(&mut stream, &format!("err {error}\n"));
                }
            }
            Err(error) => eprintln!("helper socket error: {error}"),
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn run() -> Result<(), String> {
    Err("Privileged helper is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
pub fn install_if_needed() -> Result<(), String> {
    if helper_is_current() {
        return Ok(());
    }

    install()?;

    for _ in 0..30 {
        if helper_is_current() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err("Helper installed but did not start".to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn install_if_needed() -> Result<(), String> {
    Err("Privileged helper is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
pub fn set_lid_sleep_prevention(enabled: bool) -> Result<bool, String> {
    let value = if enabled { "1" } else { "0" };
    parse_state_response(&helper_request(&format!("set {value}"))?)
}

#[cfg(not(target_os = "macos"))]
pub fn set_lid_sleep_prevention(_enabled: bool) -> Result<bool, String> {
    Err("Privileged helper is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn helper_is_current() -> bool {
    matches!(
      helper_request("version"),
      Ok(response) if response.trim() == format!("version {HELPER_VERSION}")
    )
}

#[cfg(target_os = "macos")]
fn install() -> Result<(), String> {
    let source =
        env::current_exe().map_err(|error| format!("Resolve current executable: {error}"))?;
    let source = source.to_string_lossy();
    let uid = unsafe { libc::getuid() };
    let plist = helper_plist(uid);
    let script = format!(
        r#"set -e
/bin/launchctl bootout system/{label} >/dev/null 2>&1 || true
/bin/rm -f {socket}
/bin/mkdir -p /Library/PrivilegedHelperTools
/bin/cp {source} {helper}
/usr/sbin/chown root:wheel {helper}
/bin/chmod 555 {helper}
/usr/bin/xattr -d com.apple.quarantine {helper} >/dev/null 2>&1 || true
/usr/bin/codesign --force --sign - {helper}
/bin/cat > {plist_path} <<'PLIST'
{plist}
PLIST
/usr/sbin/chown root:wheel {plist_path}
/bin/chmod 644 {plist_path}
/bin/launchctl bootstrap system {plist_path}
/bin/launchctl enable system/{label}
/bin/launchctl kickstart -k system/{label}
"#,
        label = HELPER_LABEL,
        socket = shell_quote(HELPER_SOCKET_PATH),
        source = shell_quote(&source),
        helper = shell_quote(HELPER_PATH),
        plist_path = shell_quote(HELPER_PLIST_PATH),
        plist = plist,
    );

    run_admin_shell_script(&script)
}

#[cfg(target_os = "macos")]
fn helper_plist(uid: libc::uid_t) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{helper}</string>
    <string>--helper</string>
    <string>--allowed-uid</string>
    <string>{uid}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/var/log/{label}.log</string>
  <key>StandardErrorPath</key>
  <string>/var/log/{label}.log</string>
</dict>
</plist>
"#,
        label = HELPER_LABEL,
        helper = HELPER_PATH,
        uid = uid,
    )
}

#[cfg(target_os = "macos")]
fn run_admin_shell_script(script: &str) -> Result<(), String> {
    let script_path =
        env::temp_dir().join(format!("{HELPER_LABEL}.install.{}.sh", std::process::id()));
    fs::write(&script_path, script).map_err(|error| format!("Write helper installer: {error}"))?;
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("Secure helper installer: {error}"))?;

    let command = format!("/bin/sh {}", shell_quote(&script_path.to_string_lossy()));
    let apple_script = format!(
        "do shell script \"{}\" with administrator privileges",
        escape_applescript(&command)
    );
    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(apple_script)
        .output()
        .map_err(|error| format!("Request administrator privileges: {error}"));

    let _ = fs::remove_file(&script_path);
    let output = output?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("Helper installation failed", &output))
    }
}

#[cfg(target_os = "macos")]
fn allowed_uid_from_args() -> Result<libc::uid_t, String> {
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--allowed-uid" {
            let value = args
                .next()
                .ok_or_else(|| "--allowed-uid requires a value".to_string())?;
            return value
                .parse::<libc::uid_t>()
                .map_err(|error| format!("Invalid allowed uid: {error}"));
        }
    }

    Err("Missing --allowed-uid".to_string())
}

#[cfg(target_os = "macos")]
fn secure_socket(allowed_uid: libc::uid_t) -> Result<(), String> {
    let path = std::ffi::CString::new(HELPER_SOCKET_PATH).map_err(|error| error.to_string())?;
    let chown_result = unsafe { libc::chown(path.as_ptr(), allowed_uid, !0 as libc::gid_t) };
    if chown_result != 0 {
        return Err(format!(
            "chown helper socket: {}",
            std::io::Error::last_os_error()
        ));
    }

    fs::set_permissions(HELPER_SOCKET_PATH, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("chmod helper socket: {error}"))
}

#[cfg(target_os = "macos")]
fn peer_is_allowed(stream: &UnixStream, allowed_uid: libc::uid_t) -> bool {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };

    result == 0 && (uid == allowed_uid || uid == 0)
}

#[cfg(target_os = "macos")]
fn handle_request(stream: &mut UnixStream) -> Result<(), String> {
    let mut request = String::new();
    std::io::Read::by_ref(stream)
        .take(4096)
        .read_to_string(&mut request)
        .map_err(|error| format!("Read helper request: {error}"))?;

    let mut parts = request.split_whitespace();
    match parts.next() {
        Some("version") => write_response(stream, &format!("version {HELPER_VERSION}\n")),
        Some("status") => write_state(stream, super::query_lid_sleep_prevention()?),
        Some("set") => {
            let enabled = match parts.next() {
                Some("1") => true,
                Some("0") => false,
                _ => return Err("set requires 0 or 1".to_string()),
            };
            apply_pmset(enabled)?;
            write_state(stream, super::query_lid_sleep_prevention()?)
        }
        _ => Err("unknown command".to_string()),
    }
}

#[cfg(target_os = "macos")]
fn apply_pmset(enabled: bool) -> Result<(), String> {
    let value = if enabled { "1" } else { "0" };
    let output = Command::new("/usr/bin/pmset")
        .args(["-a", "disablesleep", value])
        .output()
        .map_err(|error| format!("Run pmset: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("pmset failed", &output))
    }
}

#[cfg(target_os = "macos")]
fn write_state(stream: &mut UnixStream, enabled: bool) -> Result<(), String> {
    write_response(stream, if enabled { "ok 1\n" } else { "ok 0\n" })
}

#[cfg(target_os = "macos")]
fn write_response(stream: &mut UnixStream, response: &str) -> Result<(), String> {
    stream
        .write_all(response.as_bytes())
        .map_err(|error| format!("Write helper response: {error}"))
}

#[cfg(target_os = "macos")]
fn helper_request(command: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(HELPER_SOCKET_PATH)
        .map_err(|error| format!("Connect helper: {error}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    stream
        .write_all(command.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| format!("Write helper request: {error}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| format!("Close helper request: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("Read helper response: {error}"))?;

    if let Some(error) = response.trim().strip_prefix("err ") {
        Err(error.to_string())
    } else {
        Ok(response)
    }
}

#[cfg(target_os = "macos")]
fn parse_state_response(response: &str) -> Result<bool, String> {
    match response.trim() {
        "ok 1" => Ok(true),
        "ok 0" => Ok(false),
        other => Err(format!("Unexpected helper response: {other}")),
    }
}

#[cfg(target_os = "macos")]
fn command_error(prefix: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if !stderr.is_empty() {
        format!("{prefix}: {stderr}")
    } else if !stdout.is_empty() {
        format!("{prefix}: {stdout}")
    } else {
        prefix.to_string()
    }
}

#[cfg(target_os = "macos")]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn escape_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
