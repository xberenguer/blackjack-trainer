fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../Cargo.toml");
    let manifest=std::fs::read_to_string("../Cargo.toml").expect("read core package manifest");
    let mut package=false;
    let version=manifest.lines().find_map(|line|{let line=line.trim();if line.starts_with('['){package=line=="[package]";}if package&&line.starts_with("version"){line.split_once('=').map(|(_,v)|v.trim().trim_matches('"').to_owned())}else{None}}).expect("core package version");
    println!("cargo:rustc-env=STARDUST_CORE_VERSION={version}");
}
