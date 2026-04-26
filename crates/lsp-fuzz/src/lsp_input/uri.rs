use std::{borrow::Cow, path::Path, sync::LazyLock};

use lsp_types::Uri;

use super::LspInput;

pub fn root_uri() -> Uri {
    static WORKSPACE_ROOT_URI: LazyLock<lsp_types::Uri> =
        LazyLock::new(|| LspInput::PROTOCOL_PREFIX.parse().unwrap());
    WORKSPACE_ROOT_URI.clone()
}

#[must_use]
pub fn path_from_virtual_uri(uri: &Uri) -> Option<&str> {
    uri.as_str().strip_prefix(LspInput::PROTOCOL_PREFIX)
}

#[must_use]
pub fn virtual_uri_for_path(path: &Path) -> Option<Uri> {
    let path = path.to_str()?;
    format!("{}{}", LspInput::PROTOCOL_PREFIX, path)
        .parse()
        .ok()
}

/// Converts a localized workspace URI back into the virtual `lsp-fuzz://` form.
///
/// # Panics
///
/// Panics if the reconstructed URI cannot be parsed as a valid [`Uri`].
#[must_use]
pub fn lift_uri(uri: &Uri) -> Cow<'_, Uri> {
    let uri_str = uri.as_str();
    if let Some(index) = uri_str.find(LspInput::WORKSPACE_DIR_PREFIX) {
        let in_workspace = uri_str[index..]
            .find('/')
            .map_or(uri_str.len(), |it| it + index + 1);
        let lifted = format!("{}/{}", LspInput::PROTOCOL_PREFIX, &uri_str[in_workspace..]);
        Cow::Owned(lifted.parse().unwrap())
    } else {
        Cow::Borrowed(uri)
    }
}

#[must_use]
pub fn workspace_uri(workspace_dir: &Path) -> Option<Cow<'_, str>> {
    let workspace_dir = workspace_dir.to_str()?;
    Some(if workspace_dir.ends_with('/') {
        Cow::Borrowed(workspace_dir)
    } else {
        Cow::Owned(format!("{workspace_dir}/"))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use lsp_types::Uri;

    use super::{virtual_uri_for_path, workspace_uri};

    #[test]
    fn create_virtual_uri_for_workspace_path() {
        let uri = virtual_uri_for_path(Path::new("src/lib.rs")).unwrap();
        assert_eq!(uri, "lsp-fuzz://src/lib.rs".parse::<Uri>().unwrap());
    }

    #[test]
    fn normalize_workspace_uri_trailing_slash() {
        assert_eq!(
            workspace_uri(Path::new("/tmp/workspace")).unwrap(),
            "/tmp/workspace/"
        );
        assert_eq!(
            workspace_uri(Path::new("/tmp/workspace/")).unwrap(),
            "/tmp/workspace/"
        );
    }
}
