use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

const ORBIT_CONFIG_DIR: &str = ".orbit";
const IDE_CONFIG_FILE: &str = "ide.json";
const ORBIT_EXTENSION_DIR: &str = "extensions/orbit-ide";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdeTarget {
    Vscode,
    Cursor,
    Antigravity,
    Windsurf,
}

impl IdeTarget {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vscode => "vscode",
            Self::Cursor => "cursor",
            Self::Antigravity => "antigravity",
            Self::Windsurf => "windsurf",
        }
    }
}

impl fmt::Display for IdeTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeStatus {
    pub config_path: PathBuf,
    pub configured_target: Option<IdeTarget>,
    pub available_targets: Vec<IdeTarget>,
    pub extension_dev_path: Option<PathBuf>,
    pub packaged_extension_path: Option<PathBuf>,
    pub editor_config_path: Option<PathBuf>,
    pub config_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct IdeConfigFile {
    version: u8,
    default_target: IdeTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ExtensionPackageJson {
    name: String,
    publisher: String,
    version: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
pub enum IdeIntegrationError {
    Io(std::io::Error),
    Archive(zip::result::ZipError),
    InvalidTarget(String),
    ExtensionSourceNotFound(PathBuf),
    InvalidExtensionManifest(String),
    LaunchUnavailable(IdeTarget),
    InstallInvocationFailed {
        target: IdeTarget,
        source: std::io::Error,
    },
    InstallFailed {
        target: IdeTarget,
        details: String,
    },
    LaunchFailed {
        target: IdeTarget,
        source: std::io::Error,
    },
}

impl fmt::Display for IdeIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Archive(error) => write!(f, "failed to build extension package: {error}"),
            Self::InvalidTarget(value) => {
                write!(
                    f,
                    "unsupported IDE target '{value}'. Use vscode, cursor, antigravity, or windsurf."
                )
            }
            Self::ExtensionSourceNotFound(path) => write!(
                f,
                "Orbit IDE extension source not found at {}",
                path.display()
            ),
            Self::InvalidExtensionManifest(error) => {
                write!(f, "invalid extension package.json: {error}")
            }
            Self::LaunchUnavailable(target) => write!(
                f,
                "could not find a launch command for '{target}'. Ensure the IDE CLI is installed and on PATH."
            ),
            Self::InstallInvocationFailed { target, source } => {
                write!(f, "failed to invoke '{target}' extension install command: {source}")
            }
            Self::InstallFailed { target, details } => {
                write!(f, "failed to install Orbit IDE extension into '{target}': {details}")
            }
            Self::LaunchFailed { target, source } => {
                write!(f, "failed to launch '{target}': {source}")
            }
        }
    }
}

impl std::error::Error for IdeIntegrationError {}

impl From<std::io::Error> for IdeIntegrationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<zip::result::ZipError> for IdeIntegrationError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::Archive(value)
    }
}

pub fn parse_target(value: &str) -> Result<IdeTarget, IdeIntegrationError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "vscode" | "code" => Ok(IdeTarget::Vscode),
        "cursor" => Ok(IdeTarget::Cursor),
        "antigravity" => Ok(IdeTarget::Antigravity),
        "windsurf" => Ok(IdeTarget::Windsurf),
        other => Err(IdeIntegrationError::InvalidTarget(other.to_string())),
    }
}

#[must_use]
pub fn collect_status(workspace_root: &Path) -> IdeStatus {
    let config_path = ide_config_path(workspace_root);
    let (configured_target, config_error) = match read_config(&config_path) {
        Ok(config) => (Some(config.default_target), None),
        Err(ReadConfigError::Missing) => (None, None),
        Err(ReadConfigError::Io(error)) => (None, Some(error.to_string())),
        Err(ReadConfigError::Invalid(error)) => (None, Some(error)),
    };

    IdeStatus {
        packaged_extension_path: find_packaged_extension(workspace_root),
        config_path,
        configured_target,
        available_targets: detect_available_targets(),
        extension_dev_path: find_extension_dev_path(workspace_root),
        editor_config_path: configured_target
            .map(|target| editor_config_path(workspace_root, target)),
        config_error,
    }
}

pub fn set_default_target(
    workspace_root: &Path,
    target: IdeTarget,
) -> Result<PathBuf, IdeIntegrationError> {
    let config_path = ide_config_path(workspace_root);
    if let Some(dir) = config_path.parent() {
        fs::create_dir_all(dir)?;
    }
    let config = IdeConfigFile {
        version: 1,
        default_target: target,
    };
    let payload = serde_json::to_string_pretty(&config)
        .map_err(|error| IdeIntegrationError::Io(std::io::Error::other(error.to_string())))?;
    fs::write(&config_path, format!("{payload}\n"))?;
    Ok(config_path)
}

pub fn launch_target(target: IdeTarget, workspace_root: &Path) -> Result<(), IdeIntegrationError> {
    let program =
        resolve_launch_program(target).ok_or(IdeIntegrationError::LaunchUnavailable(target))?;
    Command::new(program)
        .arg(workspace_root)
        .spawn()
        .map(|_| ())
        .map_err(|source| IdeIntegrationError::LaunchFailed { target, source })
}

pub fn install_extension(
    target: IdeTarget,
    workspace_root: &Path,
) -> Result<PathBuf, IdeIntegrationError> {
    let package_path = package_extension(workspace_root)?;
    install_packaged_extension(target, &package_path)?;
    Ok(package_path)
}

pub fn install_packaged_extension(
    target: IdeTarget,
    package_path: &Path,
) -> Result<(), IdeIntegrationError> {
    let program =
        resolve_launch_program(target).ok_or(IdeIntegrationError::LaunchUnavailable(target))?;
    let output = Command::new(program)
        .arg("--install-extension")
        .arg(package_path)
        .arg("--force")
        .output()
        .map_err(|source| IdeIntegrationError::InstallInvocationFailed { target, source })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let details = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("installer exited with status {}", output.status)
        };
        return Err(IdeIntegrationError::InstallFailed { target, details });
    }
    Ok(())
}

pub fn setup_editor_integration(
    workspace_root: &Path,
    target: IdeTarget,
) -> Result<PathBuf, IdeIntegrationError> {
    let config_path = editor_config_path(workspace_root, target);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({
        "version": 1,
        "editor": target.as_str(),
        "cliPath": "orbit",
        "notes": "This file is generated by /ide for Orbit IDE extension integration."
    });
    let formatted = serde_json::to_string_pretty(&payload)
        .map_err(|error| IdeIntegrationError::Io(std::io::Error::other(error.to_string())))?;
    fs::write(&config_path, format!("{formatted}\n"))?;
    Ok(config_path)
}

pub fn package_extension(workspace_root: &Path) -> Result<PathBuf, IdeIntegrationError> {
    let extension_path = find_extension_dev_path(workspace_root).ok_or_else(|| {
        IdeIntegrationError::ExtensionSourceNotFound(workspace_root.join(ORBIT_EXTENSION_DIR))
    })?;
    let metadata = read_extension_metadata(&extension_path)?;
    let package_dir = workspace_root.join(ORBIT_CONFIG_DIR).join("extensions");
    fs::create_dir_all(&package_dir)?;
    let package_path = package_dir.join(format!("orbit-ide-{}.vsix", metadata.version));
    write_vsix_archive(&extension_path, &metadata, &package_path)?;
    Ok(package_path)
}

fn ide_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(ORBIT_CONFIG_DIR).join(IDE_CONFIG_FILE)
}

fn find_packaged_extension(workspace_root: &Path) -> Option<PathBuf> {
    let dir = workspace_root.join(ORBIT_CONFIG_DIR).join("extensions");
    let entries = fs::read_dir(dir).ok()?;
    entries
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "vsix"))
        .max()
}

fn editor_config_path(workspace_root: &Path, target: IdeTarget) -> PathBuf {
    match target {
        IdeTarget::Vscode => workspace_root.join(".vscode").join("orbit.json"),
        IdeTarget::Cursor => workspace_root.join(".cursor").join("orbit.json"),
        IdeTarget::Antigravity => workspace_root.join(".antigravity").join("orbit.json"),
        IdeTarget::Windsurf => workspace_root.join(".windsurf").join("orbit.json"),
    }
}

#[derive(Debug)]
enum ReadConfigError {
    Missing,
    Io(std::io::Error),
    Invalid(String),
}

fn read_config(path: &Path) -> Result<IdeConfigFile, ReadConfigError> {
    if !path.exists() {
        return Err(ReadConfigError::Missing);
    }
    let raw = fs::read_to_string(path).map_err(ReadConfigError::Io)?;
    serde_json::from_str::<IdeConfigFile>(&raw).map_err(|error| {
        ReadConfigError::Invalid(format!("invalid IDE config at {}: {error}", path.display()))
    })
}

fn detect_available_targets() -> Vec<IdeTarget> {
    let mut available = Vec::new();
    for target in [
        IdeTarget::Vscode,
        IdeTarget::Cursor,
        IdeTarget::Antigravity,
        IdeTarget::Windsurf,
    ] {
        if resolve_launch_program(target).is_some() {
            available.push(target);
        }
    }
    available
}

fn resolve_launch_program(target: IdeTarget) -> Option<PathBuf> {
    let command_name = match target {
        IdeTarget::Vscode => "code",
        IdeTarget::Cursor => "cursor",
        IdeTarget::Antigravity => "antigravity",
        IdeTarget::Windsurf => "windsurf",
    };

    if command_on_path(command_name) {
        return Some(PathBuf::from(command_name));
    }

    macos_fallback_binary(target).filter(|path| path.is_file())
}

fn command_on_path(command: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    #[cfg(windows)]
    let candidates = vec![
        command.to_string(),
        format!("{command}.exe"),
        format!("{command}.cmd"),
    ];
    #[cfg(not(windows))]
    let candidates = [command.to_string()];

    env::split_paths(&paths).any(|dir| candidates.iter().any(|name| dir.join(name).is_file()))
}

#[allow(clippy::unnecessary_wraps)]
fn macos_fallback_binary(target: IdeTarget) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let path = match target {
            IdeTarget::Vscode => PathBuf::from(
                "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
            ),
            IdeTarget::Cursor => {
                PathBuf::from("/Applications/Cursor.app/Contents/Resources/app/bin/cursor")
            }
            IdeTarget::Antigravity => PathBuf::from(
                "/Applications/Antigravity.app/Contents/Resources/app/bin/antigravity",
            ),
            IdeTarget::Windsurf => {
                PathBuf::from("/Applications/Windsurf.app/Contents/Resources/app/bin/windsurf")
            }
        };
        Some(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = target;
        None
    }
}

fn find_extension_dev_path(workspace_root: &Path) -> Option<PathBuf> {
    for base in workspace_root.ancestors() {
        let candidate = base.join(ORBIT_EXTENSION_DIR);
        if candidate.join("package.json").is_file() {
            return Some(candidate);
        }
    }
    None
}

fn read_extension_metadata(
    extension_path: &Path,
) -> Result<ExtensionPackageJson, IdeIntegrationError> {
    let package_json = extension_path.join("package.json");
    let raw = fs::read_to_string(&package_json).map_err(IdeIntegrationError::Io)?;
    serde_json::from_str::<ExtensionPackageJson>(&raw)
        .map_err(|error| IdeIntegrationError::InvalidExtensionManifest(error.to_string()))
}

fn write_vsix_archive(
    extension_path: &Path,
    metadata: &ExtensionPackageJson,
    destination: &Path,
) -> Result<(), IdeIntegrationError> {
    let file = fs::File::create(destination)?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    archive.start_file("[Content_Types].xml", options)?;
    archive.write_all(br#"<?xml version="1.0" encoding="utf-8"?>"#)?;
    archive.write_all(
        br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="json" ContentType="application/json"/><Default Extension="md" ContentType="text/markdown"/><Default Extension="js" ContentType="application/javascript"/><Default Extension="txt" ContentType="text/plain"/><Default Extension="vsixmanifest" ContentType="text/xml"/><Default Extension="xml" ContentType="text/xml"/></Types>"#,
    )?;

    archive.start_file("extension.vsixmanifest", options)?;
    archive.write_all(render_vsix_manifest(metadata).as_bytes())?;

    for entry in WalkDir::new(extension_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let relative = entry
            .path()
            .strip_prefix(extension_path)
            .map_err(|error| IdeIntegrationError::Io(std::io::Error::other(error.to_string())))?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        let zip_path = format!("extension/{relative}");
        archive.start_file(zip_path, options)?;
        let bytes = fs::read(entry.path())?;
        archive.write_all(&bytes)?;
    }

    archive.finish()?;
    Ok(())
}

fn render_vsix_manifest(metadata: &ExtensionPackageJson) -> String {
    let display_name = metadata
        .display_name
        .as_deref()
        .unwrap_or(metadata.name.as_str());
    let description = metadata
        .description
        .as_deref()
        .unwrap_or("Orbit IDE extension");
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="{id}" Version="{version}" Publisher="{publisher}" />
    <DisplayName>{display_name}</DisplayName>
    <Description>{description}</Description>
    <Tags>orbit</Tags>
    <Categories>Other</Categories>
    <GalleryFlags>Public</GalleryFlags>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="^1.85.0" />
    </Properties>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code" />
  </Installation>
  <Dependencies />
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" />
  </Assets>
</PackageManifest>
"#,
        id = metadata.name,
        version = metadata.version,
        publisher = metadata.publisher,
        display_name = display_name,
        description = description,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        collect_status, package_extension, parse_target, set_default_target,
        setup_editor_integration, IdeTarget,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("orbit-ide-integration-{label}-{nanos}"))
    }

    fn remove_temp_dir(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn parse_target_accepts_supported_aliases() {
        assert_eq!(
            parse_target("vscode").expect("vscode should parse"),
            IdeTarget::Vscode
        );
        assert_eq!(
            parse_target("code").expect("code should parse"),
            IdeTarget::Vscode
        );
        assert_eq!(
            parse_target("cursor").expect("cursor should parse"),
            IdeTarget::Cursor
        );
        assert_eq!(
            parse_target("antigravity").expect("antigravity should parse"),
            IdeTarget::Antigravity
        );
        assert_eq!(
            parse_target("windsurf").expect("windsurf should parse"),
            IdeTarget::Windsurf
        );
    }

    #[test]
    fn status_reads_configured_target() {
        let root = temp_path("status");
        fs::create_dir_all(&root).expect("temp root should create");
        set_default_target(&root, IdeTarget::Cursor).expect("should persist default target");
        let status = collect_status(&root);
        assert_eq!(status.configured_target, Some(IdeTarget::Cursor));
        assert!(status
            .editor_config_path
            .as_ref()
            .expect("configured path should be present")
            .ends_with(".cursor/orbit.json"));
        assert!(status.config_error.is_none());
        remove_temp_dir(&root);
    }

    #[test]
    fn setup_editor_integration_writes_workspace_config() {
        let root = temp_path("setup-editor");
        fs::create_dir_all(&root).expect("temp root should create");
        let config_path =
            setup_editor_integration(&root, IdeTarget::Vscode).expect("editor config should write");
        assert!(config_path.ends_with(".vscode/orbit.json"));
        let content = fs::read_to_string(&config_path).expect("editor config should be readable");
        assert!(content.contains("\"editor\": \"vscode\""));
        remove_temp_dir(&root);
    }

    #[test]
    fn package_extension_builds_vsix_from_local_extension_source() {
        let root = temp_path("package-vsix");
        let extension_root = root.join("extensions").join("orbit-ide");
        fs::create_dir_all(&extension_root).expect("extension root should create");
        fs::write(
            extension_root.join("package.json"),
            r#"{
  "name": "orbit-ide",
  "publisher": "orbit",
  "version": "0.1.0",
  "displayName": "Orbit IDE",
  "description": "Orbit integration for VS Code and Cursor"
}"#,
        )
        .expect("package json should write");
        fs::write(extension_root.join("extension.js"), "module.exports = {};")
            .expect("extension file should write");
        fs::write(extension_root.join("README.md"), "# Orbit IDE").expect("readme should write");

        let packaged = package_extension(&root).expect("vsix should package");
        assert!(packaged.ends_with(".orbit/extensions/orbit-ide-0.1.0.vsix"));
        assert!(packaged.is_file());

        remove_temp_dir(&root);
    }
}
