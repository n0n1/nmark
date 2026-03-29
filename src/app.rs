use std::fs;

use crate::document;
use crate::error::{AppError, AppResult};
use crate::extractor;
use crate::frontmatter;
use crate::http_client;
use crate::metadata;
use crate::settings::ResolvedConfig;

pub struct App {
    client: reqwest::Client,
}

impl App {
    pub fn new(http: &http_client::HttpConfig) -> AppResult<Self> {
        Ok(Self {
            client: http_client::build_client(http)?,
        })
    }

    pub async fn run(&self, config: ResolvedConfig) -> AppResult<()> {
        let total = config.urls.len();
        let mut failed = 0usize;

        for (index, url) in config.urls.iter().enumerate() {
            if let Err(error) = self.process_url(&config, index, url).await {
                failed += 1;
                eprintln!("Failed to process `{url}`: {error}");
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

    async fn process_url(&self, config: &ResolvedConfig, index: usize, url: &str) -> AppResult<()> {
        let html = http_client::fetch_html(&self.client, url).await?;
        let metadata = metadata::extract_metadata(&html);
        let article = extractor::extract_article(&html, url)?;
        let title = article.title.as_deref().unwrap_or("article");
        let author = article.byline.as_deref().or(metadata.author.as_deref());
        let content = article.content.ok_or(AppError::MissingContent)?;
        let body = document::to_markdown(&content);

        let markdown = if config.include_frontmatter {
            let frontmatter = frontmatter::build_frontmatter(url, author, &metadata.tags)?;
            document::compose_markdown(Some(&frontmatter), &body)
        } else {
            document::compose_markdown(None, &body)
        };

        if config.write_to_stdout {
            if index > 0 {
                println!();
                println!();
            }
            println!("{markdown}");
        } else {
            let output_path = document::unique_output_path(&config.output_dir, title);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output_path, markdown)?;
            println!("Saved markdown to {}", output_path.display());
        }

        Ok(())
    }
}
