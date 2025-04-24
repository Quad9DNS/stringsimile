use std::path::PathBuf;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Output {
    Stdout,
    File(PathBuf),
}
