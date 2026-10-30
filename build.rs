use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/jusage.ico");
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target != "windows" || !cfg!(windows) {
        return;
    }

    // winres 0.1.12 iterates HashMaps while generating VERSIONINFO.
    // Generate the same fields in a fixed order before letting winres compile the .rc.
    let major: u16 = env::var("CARGO_PKG_VERSION_MAJOR").unwrap().parse().unwrap();
    let minor: u16 = env::var("CARGO_PKG_VERSION_MINOR").unwrap().parse().unwrap();
    let patch: u16 = env::var("CARGO_PKG_VERSION_PATCH").unwrap().parse().unwrap();
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let name = env::var("CARGO_PKG_NAME").unwrap();
    let rc = format!(
        "#pragma code_page(65001)\n\
         1 VERSIONINFO\n\
         FILEVERSION {major}, {minor}, {patch}, 0\n\
         PRODUCTVERSION {major}, {minor}, {patch}, 0\n\
         FILEOS 0x40004\n\
         FILETYPE 0x1\n\
         FILESUBTYPE 0x0\n\
         FILEFLAGSMASK 0x3f\n\
         FILEFLAGS 0x0\n\
         {{\n\
         BLOCK \"StringFileInfo\"\n\
         {{\n\
         BLOCK \"000004b0\"\n\
         {{\n\
         VALUE \"FileDescription\", \"{name}\"\n\
         VALUE \"FileVersion\", \"{version}\"\n\
         VALUE \"ProductName\", \"{name}\"\n\
         VALUE \"ProductVersion\", \"{version}\"\n\
         }}\n\
         }}\n\
         BLOCK \"VarFileInfo\"\n\
         {{\n\
         VALUE \"Translation\", 0x0, 0x04b0\n\
         }}\n\
         }}\n\
         1 ICON \"assets/jusage.ico\"\n"
    );
    let rc_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("jusage-deterministic.rc");
    fs::write(&rc_path, rc).expect("could not write deterministic Windows resource");

    let mut res = winres::WindowsResource::new();
    res.set_resource_file(rc_path.to_str().expect("resource path must be UTF-8"));
    res.compile().expect("could not compile deterministic Windows resource");
}
