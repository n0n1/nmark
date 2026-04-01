use std::fs;
use std::path::PathBuf;

use crate::document;
use crate::error::{AppError, AppResult};
use crate::extractor;
use crate::frontmatter;
use crate::http_client;
use crate::metadata;
use crate::settings::ResolvedConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertRequest {
    pub url: String,
    pub output_dir: Option<PathBuf>,
    pub include_frontmatter: bool,
    pub save_to_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertResult {
    pub url: String,
    pub title: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub markdown: String,
    pub output_path: Option<PathBuf>,
}

pub struct App {
    client: reqwest::Client,
    http: http_client::HttpConfig,
}

impl App {
    pub fn new(http: &http_client::HttpConfig) -> AppResult<Self> {
        Ok(Self {
            client: http_client::build_client(http)?,
            http: http.clone(),
        })
    }

    pub async fn convert(&self, request: &ConvertRequest) -> AppResult<ConvertResult> {
        let html = http_client::fetch_html(&self.client, &self.http, &request.url).await?;
        let metadata = metadata::extract_metadata(&html);
        let article = extractor::extract_article(&html, &request.url)?;
        let title = article.title.unwrap_or_else(|| "article".to_string());
        let author = article.byline.or(metadata.author);
        let content = article.content.ok_or(AppError::MissingContent)?;
        let body = document::to_markdown(&content);

        let markdown = if request.include_frontmatter {
            let frontmatter =
                frontmatter::build_frontmatter(&request.url, author.as_deref(), &metadata.tags)?;
            document::compose_markdown(Some(&frontmatter), &body)
        } else {
            document::compose_markdown(None, &body)
        };

        let output_path = if request.save_to_file {
            let output_dir = match &request.output_dir {
                Some(output_dir) => output_dir.clone(),
                None => std::env::current_dir()?,
            };
            let output_path = document::unique_output_path(&output_dir, &title);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output_path, &markdown)?;
            Some(output_path)
        } else {
            None
        };

        Ok(ConvertResult {
            url: request.url.clone(),
            title,
            author,
            tags: metadata.tags,
            markdown,
            output_path,
        })
    }

    pub async fn run(&self, config: ResolvedConfig) -> AppResult<()> {
        let total = config.urls.len();
        if total == 1 {
            let request = ConvertRequest::from_config(&config, &config.urls[0]);
            let result = self.convert(&request).await?;
            self.emit_result(&result, 0, config.write_to_stdout);
            return Ok(());
        }

        let mut failed = 0usize;

        for (index, url) in config.urls.iter().enumerate() {
            let request = ConvertRequest::from_config(&config, url);
            match self.convert(&request).await {
                Ok(result) => self.emit_result(&result, index, config.write_to_stdout),
                Err(error) => {
                    failed += 1;
                    eprintln!("Failed to process `{url}`: {error}");
                }
            }
        }

        if failed > 0 {
            if total > 1 {
                eprintln!(
                    "Completed with failures: {} succeeded, {} failed, {} total",
                    total - failed,
                    failed,
                    total
                );
            }
            return Err(AppError::BatchFailed { failed, total });
        }

        if total > 1 {
            println!("Completed successfully: {total} URLs processed");
        }

        Ok(())
    }

    fn emit_result(&self, result: &ConvertResult, index: usize, write_to_stdout: bool) {
        if write_to_stdout {
            if index > 0 {
                println!();
                println!();
            }
            println!("{}", result.markdown);
        } else if let Some(output_path) = &result.output_path {
            println!("Saved markdown to {}", output_path.display());
        }
    }
}

impl ConvertRequest {
    pub fn from_config(config: &ResolvedConfig, url: &str) -> Self {
        Self {
            url: url.to_string(),
            output_dir: Some(config.output_dir.clone()),
            include_frontmatter: config.include_frontmatter,
            save_to_file: !config.write_to_stdout,
        }
    }
}
