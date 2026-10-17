//! Bettet das App-Icon (`assets\icon\firepixel.ico`, 16–256 px) und die Versionsangaben als
//! Windows-Ressource in die exe ein. Allgemein gehalten (Name/Version aus Cargo), damit
//! `firepixel-cli` dieselbe Datei über `build = "../firepixel/build.rs"` nutzen kann.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let ico = manifest
        .join("..")
        .join("..")
        .join("assets")
        .join("icon")
        .join("firepixel.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let ico = std::fs::canonicalize(&ico).unwrap_or(ico);
    // rc.exe versteht keine \\?\-Präfixe
    let ico_s = ico
        .display()
        .to_string()
        .trim_start_matches(r"\\?\")
        .replace('\\', "\\\\");
    let name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "firepixel".into());
    let ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let mut nums: Vec<u16> = ver
        .split(['.', '-'])
        .filter_map(|p| p.parse().ok())
        .collect();
    nums.resize(4, 0);
    let (a, b, c, d) = (nums[0], nums[1], nums[2], nums[3]);
    let desc = if name == "firepixel" {
        "Firepixel"
    } else {
        "Firepixel Kommandozeile"
    };
    let rc = format!(
        r#"1 ICON "{ico_s}"
1 VERSIONINFO
FILEVERSION {a},{b},{c},{d}
PRODUCTVERSION {a},{b},{c},{d}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040704b0"
    BEGIN
      VALUE "CompanyName", "Firepixel"
      VALUE "FileDescription", "{desc}"
      VALUE "FileVersion", "{ver}"
      VALUE "InternalName", "{name}"
      VALUE "OriginalFilename", "{name}.exe"
      VALUE "ProductName", "Firepixel"
      VALUE "ProductVersion", "{ver}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0407, 1200
  END
END
"#
    );
    let out =
        PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join(format!("{name}.rc"));
    if std::fs::write(&out, rc).is_err() {
        println!("cargo:warning=Ressourcendatei konnte nicht geschrieben werden");
        return;
    }
    if let Err(e) = embed_resource::compile(&out, embed_resource::NONE).manifest_optional() {
        println!("cargo:warning=App-Icon nicht eingebettet: {e}");
    }
}
