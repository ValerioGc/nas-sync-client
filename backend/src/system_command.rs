use std::{
    io::{Read, Seek},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_OUTPUT: u64 = 65536;

pub fn output(command: &mut Command, timeout: Duration) -> Option<Vec<u8>> {
    let mut capture = tempfile::NamedTempFile::new().ok()?;
    let stdout = capture.as_file().try_clone().ok()?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => {
                thread::sleep(
                    Duration::from_millis(10).min(timeout.saturating_sub(started.elapsed())),
                );
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    capture.as_file_mut().rewind().ok()?;
    let mut bytes = Vec::new();
    capture
        .as_file_mut()
        .take(MAX_OUTPUT + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= MAX_OUTPUT).then_some(bytes)
}

#[cfg(test)]
#[path = "../tests/system_command/mod.rs"]
mod tests;
