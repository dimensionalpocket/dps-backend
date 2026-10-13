use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for resolving the Vite frontend assets.
///
/// Values are fixed per app and coupled to the frontend build; embedders and
/// tests override fields programmatically via `DpsBackend::with_frontend_config`.
/// `dev_mode` is sourced from `DpsConfig::get_development_mode()`.
#[derive(Debug, Clone)]
pub struct FrontendConfig {
  pub dev_mode: bool,
  /// Prod asset URL prefix, e.g. `/build/`.
  pub base: String,
  /// Directory containing the built Vite bundle, e.g. `frontend/dist`.
  pub dist_path: PathBuf,
  /// Hot file written by the Vite dev server, e.g. `frontend/hot`.
  pub hot_file_path: PathBuf,
  /// Vite entry key, e.g. `src/main.ts`.
  pub entry: String,
  /// Fallback dev server URL used when the hot file is absent.
  pub dev_server_url: String,
}

impl FrontendConfig {
  /// Defaults for the main frontend app.
  pub fn main(dev_mode: bool) -> Self {
    Self {
      dev_mode,
      base: "/build/".to_string(),
      dist_path: PathBuf::from("frontend/dist"),
      hot_file_path: PathBuf::from("frontend/hot"),
      entry: "src/main.ts".to_string(),
      dev_server_url: "http://localhost:5000".to_string(),
    }
  }
}

/// A single chunk entry in the Vite build manifest.
#[derive(Debug, Deserialize)]
struct ManifestChunk {
  file: String,
  #[serde(default)]
  css: Vec<String>,
}

/// Resolves the Vite asset tags and root HTML document for Inertia responses.
///
/// Construction is infallible so tests and API-only deployments do not require
/// a frontend build.
pub struct FrontendAssets {
  config: FrontendConfig,
  manifest: Option<HashMap<String, ManifestChunk>>,
  version: String,
}

impl FrontendAssets {
  /// Load the manifest (prod) or the hot file (dev). Never panics.
  pub fn load(config: FrontendConfig) -> Self {
    if config.dev_mode {
      return Self {
        config,
        manifest: None,
        version: "dev".to_string(),
      };
    }

    let manifest_path = config.dist_path.join(".vite").join("manifest.json");
    let manifest = match std::fs::read_to_string(&manifest_path) {
      Ok(contents) => match serde_json::from_str::<HashMap<String, ManifestChunk>>(&contents) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
          tracing::warn!(?error, path = %manifest_path.display(), "failed to parse frontend manifest");
          None
        }
      },
      Err(error) => {
        tracing::warn!(?error, path = %manifest_path.display(), "failed to read frontend manifest");
        None
      }
    };

    let version = manifest
      .as_ref()
      .and_then(|manifest| manifest.get(&config.entry))
      .map(|chunk| chunk.file.clone())
      .unwrap_or_else(|| "prod".to_string());

    Self {
      config,
      manifest,
      version,
    }
  }

  /// Returns the asset version used by Inertia's stale-asset check.
  pub fn version(&self) -> String {
    self.version.clone()
  }

  /// Returns the `<script>`/`<link>` tags for the frontend entry.
  pub fn asset_tags(&self) -> String {
    if self.config.dev_mode {
      return self.dev_asset_tags();
    }
    self.prod_asset_tags()
  }

  fn dev_asset_tags(&self) -> String {
    let hot = std::fs::read_to_string(&self.config.hot_file_path)
      .ok()
      .map(|contents| contents.trim().to_string())
      .filter(|contents| !contents.is_empty())
      .unwrap_or_else(|| self.config.dev_server_url.clone());

    format!(
      r#"<script type="module" src="{hot}/{}"></script>"#,
      self.config.entry
    )
  }

  fn prod_asset_tags(&self) -> String {
    let Some(manifest) = &self.manifest else {
      return String::new();
    };
    let Some(chunk) = manifest.get(&self.config.entry) else {
      tracing::warn!(entry = %self.config.entry, "frontend entry missing from manifest");
      return String::new();
    };

    let mut tags = String::new();
    for css in &chunk.css {
      tags.push_str(&format!(
        r#"<link rel="stylesheet" href="{}{}">"#,
        self.config.base, css
      ));
    }
    tags.push_str(&format!(
      r#"<script type="module" src="{}{}"></script>"#,
      self.config.base, chunk.file
    ));
    tags
  }

  /// Build the full HTML document for an Inertia page object.
  pub fn root_html(&self, data_page: &str) -> String {
    format!(
      concat!(
        "<!doctype html>\n",
        "<html lang=\"en\">\n",
        "  <head>\n",
        "    <meta charset=\"UTF-8\" />\n",
        "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n",
        "    {tags}\n",
        "  </head>\n",
        "  <body>\n",
        "    <script data-page=\"app\" type=\"application/json\">{data_page}</script>\n",
        "    <div id=\"app\"></div>\n",
        "  </body>\n",
        "</html>\n"
      ),
      tags = self.asset_tags(),
      data_page = data_page
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  fn dev_config(hot_file_path: PathBuf) -> FrontendConfig {
    FrontendConfig {
      hot_file_path,
      ..FrontendConfig::main(true)
    }
  }

  fn prod_config(dist_path: PathBuf) -> FrontendConfig {
    FrontendConfig {
      dist_path,
      ..FrontendConfig::main(false)
    }
  }

  #[test]
  fn dev_with_hot_file_includes_hot_url_and_entry() {
    let temp = TempDir::new().unwrap();
    let hot_file = temp.path().join("hot");
    fs::write(&hot_file, "http://localhost:5000\n").unwrap();

    let assets = FrontendAssets::load(dev_config(hot_file));

    let tags = assets.asset_tags();
    assert!(tags.contains("http://localhost:5000/src/main.ts"));
    assert_eq!(assets.version(), "dev");
  }

  #[test]
  fn dev_without_hot_file_falls_back_to_dev_server_url() {
    let temp = TempDir::new().unwrap();
    let hot_file = temp.path().join("hot");

    let assets = FrontendAssets::load(dev_config(hot_file));

    let tags = assets.asset_tags();
    assert!(tags.contains("http://localhost:5000/src/main.ts"));
  }

  #[test]
  fn prod_with_manifest_uses_base_and_hashed_file() {
    let temp = TempDir::new().unwrap();
    let vite_dir = temp.path().join(".vite");
    fs::create_dir_all(&vite_dir).unwrap();
    fs::write(
      vite_dir.join("manifest.json"),
      r#"{
        "src/main.ts": {
          "file": "assets/main-abc123.js",
          "css": ["assets/main-def456.css"]
        }
      }"#,
    )
    .unwrap();

    let assets = FrontendAssets::load(prod_config(temp.path().to_path_buf()));

    let tags = assets.asset_tags();
    assert!(tags.contains("/build/assets/main-abc123.js"));
    assert!(tags.contains("/build/assets/main-def456.css"));
    assert!(!assets.version().is_empty());
  }

  #[test]
  fn prod_with_missing_manifest_returns_empty_tags_without_panicking() {
    let temp = TempDir::new().unwrap();

    let assets = FrontendAssets::load(prod_config(temp.path().to_path_buf()));

    assert!(assets.asset_tags().is_empty());
  }

  #[test]
  fn root_html_contains_data_page_app_json_and_mount_point() {
    let temp = TempDir::new().unwrap();
    let hot_file = temp.path().join("hot");
    fs::write(&hot_file, "http://localhost:5000").unwrap();

    let assets = FrontendAssets::load(dev_config(hot_file));
    let html = assets.root_html(r#"{"component":"Home"}"#);

    assert!(html.contains(r#"data-page="app""#));
    assert!(html.contains(r#"{"component":"Home"}"#));
    assert!(html.contains(r#"<div id="app"></div>"#));
  }
}
