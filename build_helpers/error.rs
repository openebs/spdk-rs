use std::fmt::{Debug, Display, Formatter};

/// TODO
#[allow(dead_code)]
pub enum Error {
    /// Error from `pkg_config` crate.
    PkgConfig(pkg_config::Error),
    /// Error from `cc` crate.
    Compiler(cc::Error),
    /// Any other error.
    Generic(String),
}

impl From<pkg_config::Error> for Error {
    fn from(err: pkg_config::Error) -> Self {
        Error::PkgConfig(err)
    }
}

impl From<cc::Error> for Error {
    fn from(err: cc::Error) -> Self {
        Error::Compiler(err)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PkgConfig(e) => {
                write!(f, "pkg_config::error {e}")
            }
            Error::Compiler(e) => {
                write!(f, "cc::builder::error {e}")
            }
            Error::Generic(e) => {
                write!(f, "exec error {e}")
            }
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PkgConfig(e) => {
                write!(f, "Failed to run 'pkg_config': {e}")
            }
            Error::Compiler(e) => {
                write!(f, "Failed to run compiler tool: {e}")
            }
            Error::Generic(e) => {
                write!(f, "{e}")
            }
        }
    }
}
