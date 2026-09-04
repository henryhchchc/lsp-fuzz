use std::{borrow::Cow, ops::Range, path::Path, sync::LazyLock};

use libafl_bolts::rands::Rand;
use lsp_types::Uri;
use serde_json::Value;

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

pub(crate) fn generate_random_uri_content<R: Rand>(rand: &mut R, max_length: usize) -> String {
    static AVAILABLE_CHARS: &str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./";

    let length = if max_length > 0 {
        rand.below_or_zero(max_length) + 1
    } else {
        0
    };

    let mut result = String::with_capacity(length);
    for _ in 0..length {
        let ch = rand.choose(AVAILABLE_CHARS.chars()).unwrap();
        result.push(ch);
    }
    result
}

/// Replaces virtual workspace URIs in a JSON value with a local workspace URI.
pub(crate) fn localize_json_value(value: &mut Value, workspace_uri: &str) {
    use Value::{Array, Object, String};

    const LSP_FUZZ_PREFIX_RANGE: Range<usize> = 0..LspInput::PROTOCOL_PREFIX.len();
    match value {
        Object(inner) => inner
            .values_mut()
            .for_each(|value| localize_json_value(value, workspace_uri)),
        Array(items) => items
            .iter_mut()
            .for_each(|value| localize_json_value(value, workspace_uri)),
        String(str_val) if str_val.starts_with(LspInput::PROTOCOL_PREFIX) => {
            str_val.replace_range(LSP_FUZZ_PREFIX_RANGE, workspace_uri);
        }
        _ => {}
    }
}

/// Replaces local workspace URIs in a JSON value with virtual workspace URIs.
pub(crate) fn lift_localized_json(value: &mut Value) {
    use Value::{Array, Object, String};

    match value {
        Object(inner) => inner.values_mut().for_each(lift_localized_json),
        Array(items) => items.iter_mut().for_each(lift_localized_json),
        String(str_val) => {
            if let Some(index) = str_val.find(LspInput::WORKSPACE_DIR_PREFIX) {
                let next_slash = str_val[index..]
                    .find('/')
                    .map_or(str_val.len(), |it| it + index + 1);
                str_val.replace_range(0..next_slash, LspInput::PROTOCOL_PREFIX);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use lsp_types::Uri;

    use super::{lift_localized_json, localize_json_value, virtual_uri_for_path, workspace_uri};

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

    #[test]
    fn localizes_virtual_uris_in_nested_json() {
        let mut value = serde_json::json!({
            "uri": "lsp-fuzz://path/to/file",
            "other_attr": { "uri": "lsp-fuzz://path/to/other_file" },
            "some_arr": ["lsp-fuzz://path/to/element"],
            "other_arr": [{ "uri": "lsp-fuzz://path/to/element" }]
        });

        localize_json_value(&mut value, "file:///path/to/workspace_dir/");

        assert_eq!(
            value,
            serde_json::json!({
                "uri": "file:///path/to/workspace_dir/path/to/file",
                "other_attr": { "uri": "file:///path/to/workspace_dir/path/to/other_file" },
                "some_arr": ["file:///path/to/workspace_dir/path/to/element"],
                "other_arr": [{ "uri": "file:///path/to/workspace_dir/path/to/element" }]
            })
        );
    }

    #[test]
    fn lifts_localized_uris_in_nested_json() {
        let mut value = serde_json::json!({
            "uri": "file:///path/to/lsp-fuzz-workspace_2333/path/to/file",
            "other_attr": { "uri": "file:///path/to/lsp-fuzz-workspace_2333/path/to/other_file" },
            "some_arr": [
                "file:///path/to/lsp-fuzz-workspace_2333/path/to/element",
                "file:///path/to/lsp-fuzz-workspace_2333/",
                "file:///path/to/lsp-fuzz-workspace_2333"
            ],
            "other_arr": [{ "uri": "file:///path/to/lsp-fuzz-workspace_2333/path/to/element" }]
        });

        lift_localized_json(&mut value);

        assert_eq!(
            value,
            serde_json::json!({
                "uri": "lsp-fuzz://path/to/file",
                "other_attr": { "uri": "lsp-fuzz://path/to/other_file" },
                "some_arr": ["lsp-fuzz://path/to/element", "lsp-fuzz://", "lsp-fuzz://"],
                "other_arr": [{ "uri": "lsp-fuzz://path/to/element" }]
            })
        );
    }
}
