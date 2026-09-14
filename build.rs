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

fn build_rcheevos() {
    let mut build = cc::Build::new();
    build
        .include("rcheevos/include")
        .include("rcheevos/src")
        .include("rcheevos/src/rapi")
        .include("rcheevos/src/rhash")
        .define("_CRT_SECURE_NO_WARNINGS", None);

    let mut count = 0;
    for dir in &["rcheevos/src/rcheevos", "rcheevos/src/rapi", "rcheevos/src/rhash"] {
        for entry in std::fs::read_dir(dir).expect("missing vendored rcheevos source dir") {
            let entry = entry.expect("failed to read rcheevos source dir");
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "c") {
                build.file(path);
                count += 1;
            }
        }
    }
    for entry in std::fs::read_dir("rcheevos/src").expect("missing rcheevos/src") {
        let entry = entry.expect("failed to read rcheevos/src");
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "c") && path.file_name().map_or(true, |n| n != "rc_libretro.c") {
            build.file(path);
            count += 1;
        }
    }

    build.compile("rcheevos");
    println!("cargo:rerun-if-changed=rcheevos");
    println!("cargo:warning=compiled rcheevos: {} C files", count);
}

fn main() {
    embed_resource::compile("icon.rc", embed_resource::NONE);
    build_rcheevos();
    
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
