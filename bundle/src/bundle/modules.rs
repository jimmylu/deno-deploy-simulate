use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    ModuleLoader, ModulePath, ModuleSource,
    bundle::transpilers::{Jsx, TypeScript, Wasm},
};
use anyhow::{Result, anyhow, bail};
use colored::Colorize as _;
use lazy_static::lazy_static;
use path_absolutize::Absolutize;
use regex::Regex;
use serde_json::Value;
use sha::{
    sha1::Sha1,
    utils::{Digest as _, DigestExt as _},
};
use url::Url;

lazy_static! {
    // Windows absolute path regex validator.
    static ref WINDOWS_REGEX: Regex = Regex::new(r"^[a-zA-Z]:\\").unwrap();
    // URL regex validator (string begins with http:// or https://).
    static ref URL_REGEX: Regex = Regex::new(r"^(http|https)://").unwrap();
}

/// Resolves an import using the appropriate loader.
pub fn resolve_import(
    base: Option<&str>,
    specifier: &str,
    import_map: Option<ImportMap>,
) -> Result<ModulePath> {
    // Use import-maps if available.
    let specifier = match import_map {
        Some(map) => map.lookup(specifier).unwrap_or_else(|| specifier.into()),
        None => specifier.into(),
    };

    // Look the params and choose a loader.
    let loader: Box<dyn ModuleLoader> = {
        let is_url_import = URL_REGEX.is_match(&specifier)
            || match base {
                Some(base) => URL_REGEX.is_match(base),
                None => false,
            };

        if is_url_import {
            Box::new(UrlModuleLoader::default())
        } else {
            Box::new(FsModuleLoader)
        }
    };

    // Resolve module.
    loader.resolve(base, &specifier)
}

/// Loads an import using the appropriate loader.
pub fn load_import(specifier: &str, skip_cache: bool) -> Result<ModuleSource> {
    // Look the params and choose a loader.
    let loader: Box<dyn ModuleLoader> = match (
        WINDOWS_REGEX.is_match(specifier),
        Url::parse(specifier).is_ok(),
    ) {
        (_, true) => Box::new(UrlModuleLoader { skip_cache }),
        _ => Box::new(FsModuleLoader),
    };

    // Load module.
    loader.load(specifier)
}

/// A single import mapping (specifier, target).
type ImportMapEntry = (String, String);

/// Key-Value entries representing WICG import-maps.
#[derive(Debug, Clone)]
pub struct ImportMap {
    map: Vec<ImportMapEntry>,
}

impl ImportMap {
    /// Creates an ImportMap from JSON text.
    pub fn parse_from_json(text: &str) -> Result<ImportMap> {
        // Parse JSON string into serde value.
        let json: Value = serde_json::from_str(text)?;
        let imports = json["imports"].to_owned();

        if imports.is_null() || !imports.is_object() {
            return Err(anyhow!("Import map's 'imports' must be an object"));
        }

        let map: HashMap<String, String> = serde_json::from_value(imports)?;
        let mut map: Vec<ImportMapEntry> = Vec::from_iter(map);

        // Note: We're sorting the imports because we need to support "Packages"
        // via trailing slashes, so the lengthier mapping should always be selected.
        //
        // https://github.com/WICG/import-maps#packages-via-trailing-slashes

        map.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(ImportMap { map })
    }

    /// Tries to match a specifier against an import-map entry.
    pub fn lookup(&self, specifier: &str) -> Option<String> {
        // Find a mapping if exists.
        let (base, mut target) = match self.map.iter().find(|(k, _)| specifier.starts_with(k)) {
            Some(mapping) => mapping.to_owned(),
            None => return None,
        };

        // The following code treats "./" as an alias for the CWD.
        if target.starts_with("./") {
            let cwd = env::current_dir().unwrap().to_string_lossy().to_string();
            target = target.replacen('.', &cwd, 1);
        }

        // Note: The reason we need this additional check below with the specifier's
        // extension (if exists) is to be able to support extension-less imports.
        //
        // https://github.com/WICG/import-maps#extension-less-imports

        match Path::new(specifier).extension() {
            Some(ext) => match Path::new(specifier) == Path::new(&base).with_extension(ext) {
                false => Some(specifier.replacen(&base, &target, 1)),
                _ => None,
            },
            None => Some(specifier.replacen(&base, &target, 1)),
        }
    }
}

static EXTENSIONS: &[&str] = &["js", "jsx", "ts", "tsx", "json", "wasm"];

#[derive(Default)]
pub struct FsModuleLoader;

impl FsModuleLoader {
    /// Transforms PathBuf into String.
    fn transform(&self, path: PathBuf) -> String {
        path.into_os_string().into_string().unwrap()
    }

    /// Checks if path is a JSON file.
    fn is_json_import(&self, path: &Path) -> bool {
        match path.extension() {
            Some(value) => value == "json",
            None => false,
        }
    }

    /// Wraps JSON data into an ES module (using v8's built in objects).
    fn wrap_json(&self, source: &str) -> String {
        format!("export default JSON.parse(`{source}`);")
    }

    /// Loads contents from a file.
    fn load_source(&self, path: &Path) -> Result<ModuleSource> {
        let source = fs::read_to_string(path)?;
        let source = match self.is_json_import(path) {
            true => self.wrap_json(source.as_str()),
            false => source,
        };

        Ok(source)
    }

    /// Loads import as file.
    fn load_as_file(&self, path: &Path) -> Result<ModuleSource> {
        // 1. Check if path is already a valid file.
        if path.is_file() {
            return self.load_source(path);
        }

        // 2. Check if we need to add an extension.
        if path.extension().is_none() {
            for ext in EXTENSIONS {
                let path = &path.with_extension(ext);
                if path.is_file() {
                    return self.load_source(path);
                }
            }
        }

        // 3. Bail out with an error.
        bail!(format!("Module not found \"{}\"", path.display()));
    }

    /// Loads import as directory using the 'index.[ext]' convention.
    fn load_as_directory(&self, path: &Path) -> Result<ModuleSource> {
        for ext in EXTENSIONS {
            let path = &path.join(format!("index.{ext}"));
            if path.is_file() {
                return self.load_source(path);
            }
        }
        bail!(format!("Module not found \"{}\"", path.display()));
    }
}

impl ModuleLoader for FsModuleLoader {
    fn resolve(&self, base: Option<&str>, specifier: &str) -> Result<ModulePath> {
        // Windows platform full path regex.
        lazy_static! {
            static ref WINDOWS_REGEX: Regex = Regex::new(r"^[a-zA-Z]:\\").unwrap();
        }

        // Resolve absolute import.
        if specifier.starts_with('/') || WINDOWS_REGEX.is_match(specifier) {
            return Ok(self.transform(Path::new(specifier).absolutize()?.to_path_buf()));
        }

        // Resolve relative import.
        let cwd = &env::current_dir().unwrap();
        let base = base.map(|v| Path::new(v).parent().unwrap()).unwrap_or(cwd);

        if specifier.starts_with("./") || specifier.starts_with("../") {
            return Ok(self.transform(base.join(specifier).absolutize()?.to_path_buf()));
        }

        bail!(format!("Module not found \"{specifier}\""));
    }

    fn load(&self, specifier: &str) -> Result<ModuleSource> {
        // Load source.
        let path = Path::new(specifier);
        let maybe_source = self
            .load_as_file(path)
            .or_else(|_| self.load_as_directory(path));

        // Append default extension (if none specified).
        let path = match path.extension() {
            Some(_) => path.into(),
            None => path.with_extension("js"),
        };

        let source = match maybe_source {
            Ok(source) => source,
            Err(_) => bail!(format!("Module not found \"{}\"", path.display())),
        };

        let path_extension = path.extension().unwrap().to_str().unwrap();
        let fname = path.to_str();

        // Use a preprocessor if necessary.
        match path_extension {
            "ts" => TypeScript::compile(fname, &source).map_err(|e| anyhow!(e.to_string())),
            _ => Ok(source),
        }
    }
}

lazy_static! {
    // Use local cache directory in development.
    pub static ref CACHE_DIR: PathBuf = if cfg!(debug_assertions) {
        PathBuf::from(".cache")
    } else {
        dirs::home_dir().unwrap().join(".dune/cache")
    };
}

#[derive(Default)]
/// Loader supporting URL imports.
pub struct UrlModuleLoader {
    // Ignores the cache and re-downloads the dependency.
    pub skip_cache: bool,
}

impl ModuleLoader for UrlModuleLoader {
    fn resolve(&self, base: Option<&str>, specifier: &str) -> Result<ModulePath> {
        // 1. Check if specifier is a valid URL.
        if let Ok(url) = Url::parse(specifier) {
            return Ok(url.into());
        }

        // 2. Check if the requester is a valid URL.
        if let Some(base) = base {
            if let Ok(base) = Url::parse(base) {
                let options = Url::options();
                let url = options.base_url(Some(&base));
                let url = url.parse(specifier)?;

                return Ok(url.as_str().to_string());
            }
        }

        // Possibly unreachable error.
        bail!("Base is not a valid URL");
    }

    fn load(&self, specifier: &str) -> Result<ModuleSource> {
        // Create the cache directory.
        if fs::create_dir_all(CACHE_DIR.as_path()).is_err() {
            bail!("Failed to create module caching directory");
        }

        // Hash URL using sha1.
        let hash = Sha1::default().digest(specifier.as_bytes()).to_hex();
        let module_path = CACHE_DIR.join(hash);

        if !self.skip_cache {
            // Check cache, and load file.
            if module_path.is_file() {
                let source = fs::read_to_string(&module_path).unwrap();
                return Ok(source);
            }
        }

        println!("{} {}", "Downloading".green(), specifier);

        // Download file and, save it to cache.
        let source = match ureq::get(specifier).call()?.body_mut().read_to_string() {
            Ok(source) => source,
            Err(_) => bail!(format!("Module not found \"{specifier}\"")),
        };

        // Use a preprocessor if necessary.
        let source = match (
            specifier.ends_with(".wasm"),
            specifier.ends_with(".jsx"),
            specifier.ends_with(".ts"),
            specifier.ends_with(".tsx"),
        ) {
            (true, _, _, _) => Wasm::parse(&source),
            (_, true, _, _) => Jsx::compile(Some(specifier), &source)?,
            (_, _, true, _) => TypeScript::compile(Some(specifier), &source)?,
            (_, _, _, true) => Jsx::compile(Some(specifier), &source)
                .and_then(|output| TypeScript::compile(Some(specifier), &output))?,
            _ => source.to_string(),
        };

        fs::write(&module_path, &source)?;

        Ok(source)
    }
}
