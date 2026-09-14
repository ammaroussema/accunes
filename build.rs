fn copy_file(source: &str, dest_dir: &std::path::Path) {
    let source = std::path::Path::new(source);
    if source.exists() {
        let dest = dest_dir.join(source.file_name().unwrap());
        if !dest_dir.exists() {
            std::fs::create_dir_all(dest_dir).ok();
        }
        std::fs::copy(source, &dest).expect(&format!("Failed to copy {}", source.display()));
        println!("cargo:rerun-if-changed={}", source.display());
    }
}

fn main() {
    embed_resource::compile("icon.rc", embed_resource::NONE);
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap();
    
    let package_dir = target_dir.join("accunes");
    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir).ok();
    }
    std::fs::create_dir_all(&package_dir).ok();
    
    for file in &["dip.cfg", "accunesicon.ico", "readme.txt"] {
        copy_file(file, target_dir);
        copy_file(file, &package_dir);
    }
}
