///! TODO
use core::result;
use snafu::Snafu;

/// Errors for SPDK wrappers.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub), context(suffix(false)), module(spdk_error))]
pub enum SpdkError {
    #[snafu(display("Bdev module '{name}' does not exist"))]
    BdevModuleNotFound { name: String },

    #[snafu(display("Bdev '{name}' is already claimed by another module"))]
    BdevAlreadyClaimed { name: String },

    #[snafu(display(
        "Bdev '{name}' is not claimed by this module '{mod_name}'",
    ))]
    BdevNotClaimed { name: String, mod_name: String },

    #[snafu(display("Failed to unregister Bdev '{name}': {source}"))]
    BdevUnregisterFailed {
        source: nix::errno::Errno,
        name: String,
    },

    #[snafu(display("Serde JSON serialization failed: {source}"))]
    SerdeFailed { source: serde_json::Error },

    #[snafu(display("SPDK JSON write failed: error code {code}"))]
    JsonWriteFailed { code: i32 },
}

/// TODO
pub type SpdkResult<T> = result::Result<T, SpdkError>;
