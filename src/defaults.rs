use std::collections::BTreeSet;

pub const DEFAULT_SCAN_PATTERNS: &[&str] = &["docs/**", "README.md", "AGENTS.md", "*.md"];

const DEFAULT_EXTENSIONS: &[&str] = &[
    ".md",
    ".mdx",
    ".txt",
    ".rst",
    ".adoc",
    ".html",
    ".css",
    ".scss",
    ".js",
    ".jsx",
    ".ts",
    ".tsx",
    ".json",
    ".jsonc",
    ".yaml",
    ".yml",
    ".toml",
    ".ini",
    ".cfg",
    ".conf",
    ".sh",
    ".bash",
    ".zsh",
    ".fish",
    ".ps1",
    ".py",
    ".rb",
    ".rs",
    ".go",
    ".java",
    ".kt",
    ".swift",
    ".c",
    ".h",
    ".cpp",
    ".cc",
    ".hpp",
    ".cs",
    ".csproj",
    ".fs",
    ".sql",
    ".tf",
    ".tfvars",
    ".proto",
    ".xml",
    ".xsd",
    ".svg",
    ".dart",
    ".elm",
    ".ex",
    ".exs",
    ".php",
    ".pl",
    ".lua",
    ".r",
    ".scala",
    ".dockerfile",
    ".gradle",
    ".make",
    ".mk",
];

const DEFAULT_SPECIAL_FILENAMES: &[&str] = &[
    "README",
    "README.md",
    "AGENTS.md",
    "LICENSE",
    "LICENSE.md",
    "Cargo.toml",
    "Cargo.lock",
    "Makefile",
    "Justfile",
    "Dockerfile",
    ".gitignore",
    ".editorconfig",
];

pub fn default_extensions() -> BTreeSet<String> {
    DEFAULT_EXTENSIONS
        .iter()
        .map(|value| value.to_string())
        .collect()
}

pub fn default_special_filenames() -> BTreeSet<String> {
    DEFAULT_SPECIAL_FILENAMES
        .iter()
        .map(|value| value.to_string())
        .collect()
}
