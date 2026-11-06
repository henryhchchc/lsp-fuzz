use libafl::inputs::BytesInput;

pub type FileContentInput = BytesInput;

pub mod document_mutation;
mod generation;
mod input;
pub mod message_edit;
pub mod message_generation;
pub mod message_mutation;
pub mod messages;
mod mutation;
pub mod ops_curiosity;
pub mod server_response;
mod session;
pub mod uri;
mod workspace;

pub use generation::LspInputGenerator;
pub use input::{LspInput, LspInputBytesConverter};
pub use mutation::LspInputMutator;
pub use workspace::{HasWorkspace, WorkspaceEntry};

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Uri;
    use std::str::FromStr;

    #[test]
    fn test_lift_uri() {
        // Given a URI with workspace prefix
        let uri_str = "file:///tmp/lsp-fuzz-workspace_2333/abc/file.rs".to_owned();
        let uri = Uri::from_str(&uri_str).unwrap();

        // When lifting the URI
        let lifted = LspInput::lift_uri(&uri);

        // Then the result has fuzzer protocol prefix and workspace path
        assert_eq!(
            lifted.as_str(),
            format!("{}/abc/file.rs", LspInput::PROTOCOL_PREFIX)
        );

        // Given a URI without workspace prefix
        let uri = Uri::from_str("file:///other/path").unwrap();

        // When lifting the URI
        let lifted = LspInput::lift_uri(&uri);

        // Then the original URI is returned unchanged
        assert_eq!(lifted.as_str(), "file:///other/path");
    }
}
