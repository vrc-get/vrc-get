use std::io::Write;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    if std::env::var("TARGET").unwrap().contains("linux") {
        // start stop gc is not supported by dotnet.
        println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
    }

    build_templates();
}

fn build_templates() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let templates = std::fs::read_to_string(std::path::Path::new("templates/list.txt")).unwrap();

    struct GzLoader<'a>(&'a str);

    impl std::fmt::Debug for GzLoader<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{}.tgz\"))",
                self.0
            )
        }
    }

    let mut list = Vec::new();
    let mut threads = Vec::new();

    for x in templates.split('\n') {
        if x.is_empty() {
            continue;
        }
        let (id, name) = x.split_once(':').unwrap();
        let id = id.trim();
        let name = name.trim();
        list.push((id, name, GzLoader(id)));

        let tar_file = out_dir.join(format!("{id}.tgz"));
        let dir = std::path::Path::new("templates").join(id);

        threads.push(std::thread::spawn(move || build_tar(tar_file, dir)));
    }

    fn build_tar(file: PathBuf, dir: PathBuf) {
        let mut tar = tar::Builder::new(flate2::GzBuilder::new().write(
            std::fs::File::create(file).unwrap(),
            flate2::Compression::best(),
        ));
        tar.append_dir_all("", dir).unwrap();
        tar.into_inner().unwrap().finish().unwrap().flush().unwrap();
    }

    let mut file = std::fs::File::create(out_dir.join("templates.rs")).unwrap();
    writeln!(
        file,
        "pub const TEMPLATES: &[(&str, &str, &[u8])] = &{list:#?};"
    )
    .unwrap();
    file.flush().unwrap();

    for t in threads {
        t.join().unwrap();
    }
}
