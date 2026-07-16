//! Shared test helper: a hermetic stand-in for the `orbit` CLI.
//!
//! The fake binary records the argv/cwd it was invoked with (NUL-separated)
//! and echoes a configurable JSONL payload, so SDK tests need no real binary,
//! API key, or network access.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tempfile::TempDir;

#[allow(dead_code)]
pub struct MockOrbit {
    #[allow(dead_code)]
    pub dir: TempDir,
    pub bin_path: PathBuf,
    pub capture_path: PathBuf,
    pub cwd_path: PathBuf,
    pub response_path: PathBuf,
}

impl MockOrbit {
    pub fn new(response: &str, exit_code: i32) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin_path = dir.path().join("orbit");
        let capture_path = dir.path().join("capture.bin");
        let cwd_path = dir.path().join("cwd.txt");
        let response_path = dir.path().join("response.jsonl");
        fs::write(&response_path, response).expect("write response");

        let script = format!(
            "#!/usr/bin/env bash\n\
             CAP=\"${{ORBIT_MOCK_CAPTURE:-/dev/null}}\"\n\
             CWD=\"${{ORBIT_MOCK_CWD:-/dev/null}}\"\n\
             RESP=\"{resp}\"\n\
             EXIT=\"{exit}\"\n\
             pwd > \"$CWD\"\n\
             printf '%s\\0' \"$@\" > \"$CAP\"\n\
             cat \"$RESP\"\n\
             exit \"$EXIT\"\n",
            resp = response_path.display(),
            exit = exit_code,
        );
        fs::write(&bin_path, script).expect("write script");
        let mut perms = fs::metadata(&bin_path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");

        Self {
            dir,
            bin_path,
            capture_path,
            cwd_path,
            response_path,
        }
    }

    /// Environment routing a spawned CLI to this mock.
    pub fn env(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(
            "ORBIT_MOCK_CAPTURE".into(),
            self.capture_path.to_string_lossy().into_owned(),
        );
        map.insert(
            "ORBIT_MOCK_CWD".into(),
            self.cwd_path.to_string_lossy().into_owned(),
        );
        map.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
        map
    }

    /// The argv the mock captured, one element per entry.
    pub fn captured_args(&self) -> Vec<String> {
        let raw = fs::read(&self.capture_path).unwrap_or_default();
        let text = String::from_utf8_lossy(&raw);
        text.split('\0')
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// The working directory the mock recorded.
    pub fn captured_cwd(&self) -> String {
        fs::read_to_string(&self.cwd_path)
            .unwrap_or_default()
            .trim()
            .to_string()
    }
}
