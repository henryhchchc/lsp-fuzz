pub(crate) trait ToTreeSitterPoint {
    fn to_ts_point(&self) -> tree_sitter::Point;
}

impl ToTreeSitterPoint for lsp_types::Position {
    fn to_ts_point(&self) -> tree_sitter::Point {
        tree_sitter::Point {
            row: self.line as _,
            column: self.character as _,
        }
    }
}

pub(crate) trait ToLspPosition {
    fn to_lsp_position(&self) -> lsp_types::Position;
}

impl ToLspPosition for tree_sitter::Point {
    fn to_lsp_position(&self) -> lsp_types::Position {
        lsp_types::Position {
            line: self
                .row
                .try_into()
                .expect("The row is too large to fit into a LSP request"),
            character: self
                .column
                .try_into()
                .expect("The column is too large to fit into a LSP request"),
        }
    }
}

pub(crate) trait ToLspRange {
    fn to_lsp_range(&self) -> lsp_types::Range;
}

impl ToLspRange for tree_sitter::Range {
    fn to_lsp_range(&self) -> lsp_types::Range {
        lsp_types::Range {
            start: self.start_point.to_lsp_position(),
            end: self.end_point.to_lsp_position(),
        }
    }
}
