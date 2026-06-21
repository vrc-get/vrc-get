use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub fn dpkg_apt_available() -> bool {
    fn impl_(cmd: &str) -> bool {
        Command::new(cmd)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|x| x.success())
    }

    impl_("apt-cache") && impl_("dpkg-query")
}

#[derive(Debug)]
pub struct PackageInfo {
    pub package_name: String,
    pub architecture: Option<String>,
}

pub fn dpkg_query_search(
    files: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> io::Result<HashMap<String, Vec<PackageInfo>>> {
    let mut child = Command::new("dpkg-query")
        .arg("--search")
        .arg("--")
        .args(files)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .env("LC_ALL", "C")
        .spawn()?;

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line_buf = String::new();

    let mut result = HashMap::new();

    while reader.read_line(&mut line_buf)? != 0 {
        let line = line_buf.trim_end_matches(['\r', '\n']);
        let Some((packages, path)) = line.split_once(": ") else {
            return Err(io::Error::other("dpkg-query output does not include ': '"));
        };

        result.insert(
            path.to_string(),
            packages
                .split(", ")
                .map(|package| {
                    if let Some((pkg, arch)) = package.rsplit_once(':') {
                        PackageInfo {
                            package_name: pkg.to_owned(),
                            architecture: Some(arch.to_owned()),
                        }
                    } else {
                        PackageInfo {
                            package_name: package.to_string(),
                            architecture: None,
                        }
                    }
                })
                .collect::<Vec<_>>(),
        );

        line_buf.clear();
    }

    let output = child.wait_with_output()?;

    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(io::Error::other(format!(
            "dpkg-query returned non-zero status code: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(result)
}

pub fn dpkg_query_list_files(
    packages: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> io::Result<Vec<String>> {
    // dpkg-query --listfiles --

    let mut child = Command::new("dpkg-query")
        .arg("--listfiles")
        .arg("--")
        .args(packages)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .env("LC_ALL", "C")
        .spawn()?;

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line_buf = String::new();

    let mut result = Vec::new();

    while reader.read_line(&mut line_buf)? != 0 {
        let line = line_buf.trim_end_matches(['\r', '\n']);
        result.push(line.to_string());

        line_buf.clear();
    }

    let output = child.wait_with_output()?;

    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(io::Error::other(format!(
            "dpkg-query returned non-zero status code: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(result)
}

#[derive(Debug)]
pub struct PackageDepends {
    // depends[and][or]
    pub depends: Vec<Vec<String>>,
}

#[derive(Default)]
pub struct AptCacheDepends {
    pub recurse: bool,
}

impl AptCacheDepends {
    pub fn recurse(mut self) -> AptCacheDepends {
        self.recurse = true;
        self
    }

    pub fn run(
        &self,
        packages: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> io::Result<HashMap<String, PackageDepends>> {
        apt_cache_depends(packages, self)
    }
}

pub fn apt_cache_depends(
    packages: impl IntoIterator<Item = impl AsRef<OsStr>>,
    options: &AptCacheDepends,
) -> io::Result<HashMap<String, PackageDepends>> {
    let mut child = Command::new("apt-cache")
        .arg("depends")
        .args([
            "--no-generate",
            "--no-recommends",
            "--no-suggests",
            "--no-conflicts",
            "--no-breaks",
            "--no-replaces",
            "--no-enhances",
        ])
        .args(options.recurse.then_some("--recurse"))
        .arg("--")
        .args(packages)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .env("LC_ALL", "C")
        .spawn()?;

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line_buf = String::new();

    struct CollectContext {
        current_package: String,
        depends: Vec<Vec<String>>,
        options: Vec<String>,
        result: HashMap<String, PackageDepends>,
    }

    impl CollectContext {
        fn start_new_package(&mut self) {
            if !self.current_package.is_empty() {
                assert!(self.options.is_empty());
                self.result.insert(
                    self.current_package.clone(),
                    PackageDepends {
                        depends: std::mem::take(&mut self.depends),
                    },
                );
            }
        }
    }

    let mut context = CollectContext {
        current_package: String::new(),
        depends: Vec::new(),
        options: Vec::new(),
        result: HashMap::new(),
    };

    while reader.read_line(&mut line_buf)? != 0 {
        let line = line_buf.trim_end_matches(['\r', '\n']);
        if let Some(pkg) = line
            .strip_prefix("  Depends: ")
            .or(line.strip_prefix("  PreDepends: "))
        {
            context.options.push(pkg.to_owned());
            context.depends.push(std::mem::take(&mut context.options));
        } else if let Some(pkg) = line
            .strip_prefix(" |Depends: ")
            .or(line.strip_prefix(" |PreDepends: "))
        {
            context.options.push(pkg.to_owned());
        } else if line.strip_prefix("    ").is_some() {
            // list of instance of virtual packages
        } else if line.starts_with(" ") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("dpkg-cache dependency line '{}' not found", line),
            ));
        } else {
            context.start_new_package();

            context.current_package = line.to_string();
        }

        line_buf.clear();
    }

    context.start_new_package();

    let output = child.wait_with_output()?;

    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(io::Error::other(format!(
            "dpkg-query returned non-zero status code: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(context.result)
}
