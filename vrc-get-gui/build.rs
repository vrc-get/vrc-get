use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    build_templates();
    get_commit_hash();
}

fn build_templates() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let templates =
        std::fs::read_to_string(std::path::Path::new("project-templates/list.txt")).unwrap();

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

    struct Variable(String);

    impl std::fmt::Debug for Variable {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    let mut constants = String::new();
    let mut list = Vec::new();
    let mut threads = Vec::new();

    for x in templates.split('\n') {
        if x.is_empty() {
            continue;
        }
        let (id, name) = x.split_once(':').unwrap();
        let id = id.trim();
        let name = name.trim();
        let ident = id.replace(['.', '-'], "_").to_uppercase();
        writeln!(
            constants,
            "pub const {ident}: &[u8] = {loader:?};",
            loader = GzLoader(id)
        )
        .unwrap();
        list.push((id, name, Variable(ident)));

        let tar_file = out_dir.join(format!("{id}.tgz"));
        let dir = std::path::Path::new("project-templates").join(id);

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
    file.write_all(constants.as_bytes()).unwrap();
    file.flush().unwrap();

    for t in threads {
        t.join().unwrap();
    }
}

fn get_commit_hash() {
    use std::process::*;

    let Ok(output) = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return;
    };

    let Some(hash_value) = (std::str::from_utf8(&output.stdout).ok())
        .and_then(|x| x.lines().next())
        .map(|x| x.trim())
    else {
        return;
    };

    println!("cargo:rustc-env=COMMIT_HASH={}", hash_value);
}
