use crate::color;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

const BROWSEROS_VERSION: &str = "0.39.0.3";

struct BrowserOSPackage {
    url: &'static str,
    file_name: &'static str,
}

pub fn run_install(with_deps: bool) {
    let is_linux = cfg!(target_os = "linux");

    if is_linux {
        if with_deps {
            println!("{}", color::cyan("Installing system dependencies..."));

            let (pkg_mgr, deps) = if which_exists("apt-get") {
                let libasound = if package_exists_apt("libasound2t64") {
                    "libasound2t64"
                } else {
                    "libasound2"
                };

                (
                    "apt-get",
                    vec![
                        "libxcb-shm0",
                        "libx11-xcb1",
                        "libx11-6",
                        "libxcb1",
                        "libxext6",
                        "libxrandr2",
                        "libxcomposite1",
                        "libxcursor1",
                        "libxdamage1",
                        "libxfixes3",
                        "libxi6",
                        "libgtk-3-0",
                        "libpangocairo-1.0-0",
                        "libpango-1.0-0",
                        "libatk1.0-0",
                        "libcairo-gobject2",
                        "libcairo2",
                        "libgdk-pixbuf-2.0-0",
                        "libxrender1",
                        libasound,
                        "libfreetype6",
                        "libfontconfig1",
                        "libdbus-1-3",
                        "libnss3",
                        "libnspr4",
                        "libatk-bridge2.0-0",
                        "libdrm2",
                        "libxkbcommon0",
                        "libatspi2.0-0",
                        "libcups2",
                        "libxshmfence1",
                        "libgbm1",
                    ],
                )
            } else if which_exists("dnf") {
                (
                    "dnf",
                    vec![
                        "nss",
                        "nspr",
                        "atk",
                        "at-spi2-atk",
                        "cups-libs",
                        "libdrm",
                        "libXcomposite",
                        "libXdamage",
                        "libXrandr",
                        "mesa-libgbm",
                        "pango",
                        "alsa-lib",
                        "libxkbcommon",
                        "libxcb",
                        "libX11-xcb",
                        "libX11",
                        "libXext",
                        "libXcursor",
                        "libXfixes",
                        "libXi",
                        "gtk3",
                        "cairo-gobject",
                    ],
                )
            } else if which_exists("yum") {
                (
                    "yum",
                    vec![
                        "nss",
                        "nspr",
                        "atk",
                        "at-spi2-atk",
                        "cups-libs",
                        "libdrm",
                        "libXcomposite",
                        "libXdamage",
                        "libXrandr",
                        "mesa-libgbm",
                        "pango",
                        "alsa-lib",
                        "libxkbcommon",
                    ],
                )
            } else {
                eprintln!(
                    "{} No supported package manager found (apt-get, dnf, or yum)",
                    color::error_indicator()
                );
                exit(1);
            };

            let install_cmd = match pkg_mgr {
                "apt-get" => {
                    format!(
                        "sudo apt-get update && sudo apt-get install -y {}",
                        deps.join(" ")
                    )
                }
                _ => format!("sudo {} install -y {}", pkg_mgr, deps.join(" ")),
            };

            println!("Running: {}", install_cmd);
            let status = Command::new("sh").arg("-c").arg(&install_cmd).status();

            match status {
                Ok(s) if s.success() => {
                    println!("{} System dependencies installed", color::success_indicator())
                }
                Ok(_) => eprintln!(
                    "{} Failed to install some dependencies. You may need to run manually with sudo.",
                    color::warning_indicator()
                ),
                Err(e) => eprintln!("{} Could not run install command: {}", color::warning_indicator(), e),
            }
        } else {
            println!(
                "{} Linux detected. If browser fails to launch, run:",
                color::warning_indicator()
            );
            println!("  agent-browser install --with-deps");
            println!();
        }
    }

    let Some(package) = get_browseros_package() else {
        eprintln!(
            "{} Unsupported platform for BrowserOS install: {} / {}",
            color::error_indicator(),
            env::consts::OS,
            env::consts::ARCH
        );
        exit(1);
    };

    let browseros_home = get_browseros_home();
    let downloads_dir = browseros_home.join("downloads");
    if let Err(e) = fs::create_dir_all(&downloads_dir) {
        eprintln!(
            "{} Failed to create download directory {}: {}",
            color::error_indicator(),
            downloads_dir.display(),
            e
        );
        exit(1);
    }

    let download_path = downloads_dir.join(package.file_name);
    println!(
        "{} Downloading BrowserOS {}...",
        color::cyan("Installing"),
        BROWSEROS_VERSION
    );

    if let Err(e) = download_file(package.url, &download_path) {
        eprintln!("{} {}", color::error_indicator(), e);
        exit(1);
    }

    let installed_executable: Option<PathBuf> = {
        #[cfg(target_os = "macos")]
        {
            match install_macos_dmg(&download_path, &browseros_home) {
                Ok(path) => Some(path),
                Err(e) => {
                    eprintln!("{} {}", color::error_indicator(), e);
                    exit(1);
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            match install_linux_appimage(&download_path, &browseros_home) {
                Ok(path) => Some(path),
                Err(e) => {
                    eprintln!("{} {}", color::error_indicator(), e);
                    exit(1);
                }
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    };

    println!("{} BrowserOS package downloaded", color::success_indicator());
    println!("  {}", download_path.display());

    if let Some(executable_path) = installed_executable {
        println!(
            "{} BrowserOS executable ready:",
            color::success_indicator()
        );
        println!("  {}", executable_path.display());
        println!();
        println!("Set this in your shell:");
        println!(
            "  export AGENT_BROWSER_EXECUTABLE_PATH=\"{}\"",
            executable_path.display()
        );
    } else if cfg!(target_os = "windows") {
        println!();
        println!("Run the downloaded installer, then set:");
        println!(
            "  set AGENT_BROWSER_EXECUTABLE_PATH=C:\\Program Files\\BrowserOS\\BrowserOS.exe"
        );
    }

    if is_linux && !with_deps {
        println!();
        println!(
            "{} If BrowserOS fails to start due to missing shared libraries, run:",
            color::yellow("Note:")
        );
        println!("  agent-browser install --with-deps");
    }
}

fn get_browseros_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(env::temp_dir)
        .join(".browseros")
}

fn get_browseros_package() -> Option<BrowserOSPackage> {
    if cfg!(target_os = "macos") {
        return match env::consts::ARCH {
            "aarch64" => Some(BrowserOSPackage {
                url: "http://cdn.browseros.com/releases/0.39.0.3/macos/BrowserOS_v0.39.0.3_arm64.dmg",
                file_name: "BrowserOS_v0.39.0.3_arm64.dmg",
            }),
            "x86_64" => Some(BrowserOSPackage {
                url: "http://cdn.browseros.com/releases/0.39.0.3/macos/BrowserOS_v0.39.0.3_x64.dmg",
                file_name: "BrowserOS_v0.39.0.3_x64.dmg",
            }),
            _ => Some(BrowserOSPackage {
                url: "http://cdn.browseros.com/releases/0.39.0.3/macos/BrowserOS_v0.39.0.3_universal.dmg",
                file_name: "BrowserOS_v0.39.0.3_universal.dmg",
            }),
        };
    }

    if cfg!(target_os = "windows") {
        return Some(BrowserOSPackage {
            url: "http://cdn.browseros.com/releases/0.39.0.3/win/BrowserOS_v0.39.0.3_x64_installer.exe",
            file_name: "BrowserOS_v0.39.0.3_x64_installer.exe",
        });
    }

    if cfg!(target_os = "linux") {
        return Some(BrowserOSPackage {
            url: "http://cdn.browseros.com/releases/0.39.0.3/linux/BrowserOS_v0.39.0.3_x64.AppImage",
            file_name: "BrowserOS_v0.39.0.3_x64.AppImage",
        });
    }

    None
}

fn download_file(url: &str, output_path: &Path) -> Result<(), String> {
    let output = output_path
        .to_str()
        .ok_or_else(|| format!("Invalid output path: {}", output_path.display()))?;

    #[cfg(windows)]
    let status = {
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri '{}' -OutFile '{}'",
            url, output
        );
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .map_err(|e| format!("Failed to run PowerShell download: {}", e))?
    };

    #[cfg(not(windows))]
    let status = {
        if which_exists("curl") {
            Command::new("curl")
                .args(["-fL", "--retry", "3", "-o", output, url])
                .status()
                .map_err(|e| format!("Failed to run curl: {}", e))?
        } else if which_exists("wget") {
            Command::new("wget")
                .args(["-O", output, url])
                .status()
                .map_err(|e| format!("Failed to run wget: {}", e))?
        } else {
            return Err("Neither curl nor wget is available in PATH".to_string());
        }
    };

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Download failed for {} (exit status: {})",
            url, status
        ))
    }
}

#[cfg(target_os = "macos")]
fn install_macos_dmg(dmg_path: &Path, browseros_home: &Path) -> Result<PathBuf, String> {
    let mount_dir = browseros_home.join("mount");
    let app_target = browseros_home.join("BrowserOS.app");

    if let Err(e) = fs::create_dir_all(browseros_home) {
        return Err(format!(
            "Failed to prepare BrowserOS directory {}: {}",
            browseros_home.display(),
            e
        ));
    }

    if mount_dir.exists() {
        let mount_str = mount_dir.to_string_lossy().to_string();
        let _ = Command::new("hdiutil")
            .args(["detach", &mount_str, "-force"])
            .status();
        let _ = fs::remove_dir_all(&mount_dir);
    }
    fs::create_dir_all(&mount_dir)
        .map_err(|e| format!("Failed to create mount directory {}: {}", mount_dir.display(), e))?;

    let mount_str = mount_dir.to_string_lossy().to_string();
    let dmg_str = dmg_path.to_string_lossy().to_string();
    let attach = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-quiet", "-mountpoint", &mount_str, &dmg_str])
        .status()
        .map_err(|e| format!("Failed to mount BrowserOS DMG: {}", e))?;

    if !attach.success() {
        let _ = fs::remove_dir_all(&mount_dir);
        return Err("Failed to mount BrowserOS DMG".to_string());
    }

    let result = (|| {
        let app_in_dmg = mount_dir.join("BrowserOS.app");
        if !app_in_dmg.exists() {
            return Err(format!(
                "BrowserOS.app not found in mounted DMG: {}",
                mount_dir.display()
            ));
        }

        if app_target.exists() {
            fs::remove_dir_all(&app_target).map_err(|e| {
                format!(
                    "Failed to remove previous BrowserOS.app at {}: {}",
                    app_target.display(),
                    e
                )
            })?;
        }

        let app_in_dmg_str = app_in_dmg.to_string_lossy().to_string();
        let app_target_str = app_target.to_string_lossy().to_string();
        let copy_status = Command::new("cp")
            .args(["-R", &app_in_dmg_str, &app_target_str])
            .status()
            .map_err(|e| format!("Failed to copy BrowserOS.app: {}", e))?;

        if !copy_status.success() {
            return Err("Failed to copy BrowserOS.app from DMG".to_string());
        }

        let executable = app_target.join("Contents").join("MacOS").join("BrowserOS");
        if !executable.exists() {
            return Err(format!(
                "Installed BrowserOS executable not found: {}",
                executable.display()
            ));
        }

        Ok(executable)
    })();

    let _ = Command::new("hdiutil")
        .args(["detach", &mount_str, "-quiet"])
        .status();
    let _ = fs::remove_dir_all(&mount_dir);

    result
}

#[cfg(target_os = "linux")]
fn install_linux_appimage(appimage_path: &Path, browseros_home: &Path) -> Result<PathBuf, String> {
    let bin_dir = browseros_home.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| {
        format!(
            "Failed to create BrowserOS bin directory {}: {}",
            bin_dir.display(),
            e
        )
    })?;

    let executable = bin_dir.join("BrowserOS");
    fs::copy(appimage_path, &executable).map_err(|e| {
        format!(
            "Failed to install BrowserOS AppImage to {}: {}",
            executable.display(),
            e
        )
    })?;

    let executable_str = executable.to_string_lossy().to_string();
    let chmod = Command::new("chmod")
        .args(["+x", &executable_str])
        .status()
        .map_err(|e| format!("Failed to run chmod +x on {}: {}", executable.display(), e))?;

    if !chmod.success() {
        return Err(format!(
            "Failed to mark BrowserOS executable as runnable: {}",
            executable.display()
        ));
    }

    Ok(executable)
}

fn which_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn package_exists_apt(pkg: &str) -> bool {
    Command::new("apt-cache")
        .arg("show")
        .arg(pkg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
