use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ImportError {
    FileNotFound {
        path: PathBuf,
        import_location: String,
    },
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
    RecursionLimit {
        depth: usize,
    },
    CircularDependency {
        chain: Vec<PathBuf>,
    },
}

#[derive(Debug, Clone)]
pub struct DependencyInfo {
    #[allow(dead_code)] // Reserved for future dependency analysis features
    pub dependencies: HashMap<PathBuf, Vec<PathBuf>>,
    pub all_files: HashSet<PathBuf>,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::FileNotFound {
                path,
                import_location,
            } => {
                write!(
                    f,
                    "Import file not found: '{}' (imported from {})",
                    path.display(),
                    import_location
                )
            }
            ImportError::IoError { path, source } => {
                write!(
                    f,
                    "Error reading import file '{}': {}",
                    path.display(),
                    source
                )
            }
            ImportError::RecursionLimit { depth } => {
                write!(
                    f,
                    "Import recursion limit exceeded (depth: {depth}). Check for circular dependencies."
                )
            }
            ImportError::CircularDependency { chain } => {
                write!(f, "Circular dependency detected: ")?;
                for (i, path) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " → ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ImportError {}

const MAX_IMPORT_DEPTH: usize = 32;

struct DependencyTracker {
    import_chain: Vec<PathBuf>,
    processed_files: HashSet<PathBuf>,
    dependencies: HashMap<PathBuf, Vec<PathBuf>>,
}

impl DependencyTracker {
    fn new() -> Self {
        Self {
            import_chain: Vec::new(),
            processed_files: HashSet::new(),
            dependencies: HashMap::new(),
        }
    }

    fn enter_file(&mut self, file_path: PathBuf) -> Result<(), ImportError> {
        // Check for circular dependency
        if self.import_chain.contains(&file_path) {
            let mut cycle_chain = self.import_chain.clone();
            cycle_chain.push(file_path);
            return Err(ImportError::CircularDependency { chain: cycle_chain });
        }

        self.import_chain.push(file_path);
        Ok(())
    }

    fn exit_file(&mut self) {
        if let Some(file_path) = self.import_chain.pop() {
            self.processed_files.insert(file_path);
        }
    }

    fn add_dependency(&mut self, from: &Path, to: &Path) {
        self.dependencies
            .entry(from.to_path_buf())
            .or_default()
            .push(to.to_path_buf());
    }

    fn get_dependency_info(&self) -> DependencyInfo {
        DependencyInfo {
            dependencies: self.dependencies.clone(),
            all_files: self.processed_files.clone(),
        }
    }
}

pub fn process_imports(
    shader_path: &Path,
    shader_source: &str,
) -> Result<(String, DependencyInfo), ImportError> {
    let mut tracker = DependencyTracker::new();
    let result = process_imports_recursive(shader_path, shader_source, &mut tracker, 0)?;
    let deps = tracker.get_dependency_info();
    Ok((result, deps))
}

fn process_imports_recursive(
    current_file: &Path,
    source: &str,
    tracker: &mut DependencyTracker,
    depth: usize,
) -> Result<String, ImportError> {
    if depth > MAX_IMPORT_DEPTH {
        return Err(ImportError::RecursionLimit { depth });
    }

    let canonical_current = current_file
        .canonicalize()
        .map_err(|e| ImportError::IoError {
            path: current_file.to_path_buf(),
            source: e,
        })?;

    tracker.enter_file(canonical_current.clone())?;

    let current_dir = current_file.parent().unwrap_or_else(|| Path::new("."));

    let import_regex = regex::Regex::new(r#"// @import "([^"]+)""#).unwrap();
    let mut result = String::new();

    for line in source.lines() {
        if let Some(captures) = import_regex.captures(line) {
            let import_path_str = &captures[1];
            let import_path = current_dir.join(import_path_str);

            let canonical_import_path = match import_path.canonicalize() {
                Ok(path) => path,
                Err(_) => {
                    return Err(ImportError::FileNotFound {
                        path: import_path,
                        import_location: current_file.display().to_string(),
                    });
                }
            };

            // Record dependency relationship
            tracker.add_dependency(&canonical_current, &canonical_import_path);

            // Skip if already processed (not in current chain, but previously completed)
            if tracker.processed_files.contains(&canonical_import_path) {
                continue;
            }

            let import_content = match fs::read_to_string(&canonical_import_path) {
                Ok(content) => content,
                Err(e) => {
                    return Err(ImportError::IoError {
                        path: canonical_import_path,
                        source: e,
                    });
                }
            };

            let processed_import = process_imports_recursive(
                &canonical_import_path,
                &import_content,
                tracker,
                depth + 1,
            )?;

            result.push_str(&processed_import);
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    tracker.exit_file();

    if result.ends_with('\n') {
        result.pop();
    }

    Ok(result)
}
