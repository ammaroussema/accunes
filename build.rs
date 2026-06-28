fn main() {
    embed_resource::compile("icon.rc", embed_resource::NONE);
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap();
    
    for file in &["dip.cfg", "accunesicon.ico"] {
        let source = std::path::Path::new(file);
        if source.exists() {
            let dest = target_dir.join(file);
            std::fs::copy(source, &dest).expect(&format!("Failed to copy {}", file));
            println!("cargo:rerun-if-changed={}", file);
        }
    }
}
