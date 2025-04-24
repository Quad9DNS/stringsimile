use std::path::PathBuf;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Input {
    Stdin,
    File(PathBuf),
}
