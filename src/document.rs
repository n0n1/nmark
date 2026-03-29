use std::fs;
use std::path::{Path, PathBuf};

pub fn to_markdown(content: &str) -> String {
    html2md::parse_html(content)
}

pub fn compose_markdown(frontmatter: Option<&str>, body: &str) -> String {
    match frontmatter {
        Some(frontmatter) => format!("{frontmatter}\n{body}"),
        None => body.to_string(),
    }
}

pub fn output_path(output_dir: &Path, title: &str) -> PathBuf {
    output_dir.join(format!("{}.md", sanitize_filename(title)))
}

pub fn unique_output_path(output_dir: &Path, title: &str) -> PathBuf {
    let stem = sanitize_filename(title);
    let mut candidate = output_path(output_dir, title);
    let mut suffix = 2usize;

    while fs::exists(&candidate).unwrap_or(false) {
        candidate = output_dir.join(format!("{stem}-{suffix}.md"));
        suffix += 1;
    }

    candidate
}

fn sanitize_filename(title: &str) -> String {
    let mut sanitized = String::with_capacity(title.len());

    for ch in title.chars() {
        let safe = match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            c if c.is_control() => '-',
            c => c,
        };
        sanitized.push(safe);
    }

    let sanitized = sanitized.trim().trim_matches('.').to_string();
    if sanitized.is_empty() {
        "article".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::{output_path, unique_output_path};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn sanitizes_output_filename() {
        let path = output_path(Path::new("out"), "A/B:C*D?");
        assert_eq!(path, Path::new("out").join("A-B-C-D-.md"));
    }

    #[test]
    fn creates_unique_output_filename_when_file_exists() {
        let dir = temp_dir("unique-output");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Article.md"), "existing").unwrap();

        let path = unique_output_path(&dir, "Article");
        assert_eq!(path, dir.join("Article-2.md"));

        let _ = fs::remove_dir_all(dir);
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rmark-{nonce}-{name}"))
    }
}
