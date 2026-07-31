use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn find_target_dir(out_dir: &Path) -> Option<PathBuf> {
    let mut current = out_dir;
    while let Some(parent) = current.parent() {
        if parent.file_name() == Some(std::ffi::OsStr::new("target")) {
            return Some(parent.to_path_buf());
        }
        current = parent;
    }
    None
}

fn add_winlibs_to_path() {
    let winlibs = r"C:\Users\hieuk\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT.LLVM_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin";
    if Path::new(winlibs).exists() {
        if let Ok(curr_path) = env::var("PATH") {
            let new_path = format!("{};{}", winlibs, curr_path);
            env::set_var("PATH", new_path);
        }
    }
}

fn create_side_by_side_manifests() {
    let manifest_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

    if let Ok(out_dir_str) = env::var("OUT_DIR") {
        let out_dir = PathBuf::from(out_dir_str);
        if let Some(target_dir) = find_target_dir(&out_dir) {
            let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
            let dest_dir = target_dir.join(&profile);
            let deps_dir = dest_dir.join("deps");

            let _ = fs::write(dest_dir.join("auto-tts-desktop.exe.manifest"), manifest_content);

            if let Ok(entries) = fs::read_dir(&deps_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext == "exe" {
                                let manifest_path = path.with_extension("exe.manifest");
                                let _ = fs::write(manifest_path, manifest_content);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    add_winlibs_to_path();
    let res = std::panic::catch_unwind(|| {
        tauri_build::build();
    });
    if res.is_err() {
        println!("cargo:warning=Skipped Windows .ico resource compilation because windres does not support paths with spaces in GNU toolchain.");
    }
    create_side_by_side_manifests();
}
