#[allow(
    clippy::wildcard_imports,
    reason = "LSP parameter types are dense here"
)]
use lsp_types::*;

compose! {
    LogTraceParams {
        message: String,
        verbose: Option<String>
    }
}
