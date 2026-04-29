use indexmap::IndexMap;
use url::{Host, Url};

pub struct RepositoriesFile {
    repositories: Vec<RepositoryInfo>,
}

pub struct RepositoryInfo {
    url: Url,
    headers: IndexMap<Box<str>, Box<str>>,
}

pub struct RepositoriesFileParseResult {
    parsed: RepositoriesFile,
    unparseable_lines: Vec<String>,
}

impl RepositoriesFile {
    pub fn parse(file: &str) -> RepositoriesFileParseResult {
        let mut parsed_lines = vec![];
        let mut unparseable_lines = vec![];

        for line in file.lines() {
            // remove comments
            let line = line
                .split_once('#')
                .map(|(before_hash, _)| before_hash)
                .unwrap_or(line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let Ok(url) = Url::parse(line) else {
                unparseable_lines.push(line.to_string());
                continue;
            };

            let mut repository: Option<RepositoryInfo> = None;

            match url.scheme() {
                "vcc" => {
                    repository = parse_add_repo_link(url);
                }
                "http" | "https" => {
                    repository = Some(RepositoryInfo {
                        url,
                        headers: IndexMap::new(),
                    });
                }
                _ => {}
            }

            if let Some(repository) = repository {
                parsed_lines.push(repository);
            } else {
                unparseable_lines.push(line.to_string());
            }
        }

        RepositoriesFileParseResult {
            parsed: RepositoriesFile {
                repositories: parsed_lines,
            },
            unparseable_lines,
        }
    }

    pub fn repositories(&self) -> &[RepositoryInfo] {
        &self.repositories
    }
}

impl RepositoriesFileParseResult {
    pub fn parsed(&self) -> &RepositoriesFile {
        &self.parsed
    }

    pub fn unparseable_lines(&self) -> &[String] {
        &self.unparseable_lines
    }
}

impl RepositoryInfo {
    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        &self.headers
    }
}

fn parse_add_repo_link(vcc_url: Url) -> Option<RepositoryInfo> {
    if vcc_url.scheme() != "vcc" {
        return None;
    }

    if vcc_url.host() != Some(Host::Domain("vpm")) {
        return None;
    }

    if "/addRepo" != vcc_url.path() {
        return None;
    }

    // add repo
    let mut url = None;
    let mut headers = IndexMap::new();
    for (key, value) in vcc_url.query_pairs() {
        match key.as_ref() {
            "url" => {
                if url.is_some() {
                    return None;
                }
                let parsed = Url::parse(&value)
                    .ok()
                    .filter(|x| x.scheme() == "http" || x.scheme() == "https")?;
                url = Some(parsed);
            }
            "headers[]" => {
                let (key, value) = value.split_once(':')?;
                headers.insert(
                    key.to_string().into_boxed_str(),
                    value.to_string().into_boxed_str(),
                );
            }
            _ => {
                log::error!("Unknown query parameter: {key}");
            }
        }
    }

    Some(RepositoryInfo { url: url?, headers })
}
