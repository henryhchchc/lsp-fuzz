use crate::lsp::{LspMessage, code_context::CodeContextRef};

pub fn calibrate_message(message: &mut LspMessage, input_edit: tree_sitter::InputEdit) {
    if let Some(pos) = message.position_mut() {
        update_position(pos, &input_edit);
    } else if let Some(range) = message.range_mut() {
        update_position(&mut range.start, &input_edit);
        update_position(&mut range.end, &input_edit);
    }
}

fn is_after_edit(pos: lsp_types::Position, edit: &tree_sitter::InputEdit) -> bool {
    usize::try_from(pos.line)
        .expect("u32 fits into usize on supported targets")
        .cmp(&edit.old_end_position.row)
        .then_with(|| {
            usize::try_from(pos.character)
                .expect("u32 fits into usize on supported targets")
                .cmp(&edit.old_end_position.column)
        })
        .is_ge()
}

fn adjust_component(current: u32, old_end: usize, new_end: usize) -> u32 {
    if new_end >= old_end {
        current.saturating_add(u32::try_from(new_end - old_end).unwrap_or(u32::MAX))
    } else {
        current.saturating_sub(u32::try_from(old_end - new_end).unwrap_or(u32::MAX))
    }
}

fn update_position(pos: &mut lsp_types::Position, edit: &tree_sitter::InputEdit) {
    if is_after_edit(*pos, edit) {
        pos.line = adjust_component(
            pos.line,
            edit.old_end_position.row,
            edit.new_end_position.row,
        );
        pos.character = adjust_component(
            pos.character,
            edit.old_end_position.column,
            edit.new_end_position.column,
        );
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{
        HoverParams, Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
        WorkDoneProgressParams,
    };

    use super::calibrate_message;
    use crate::lsp::LspMessage;

    #[test]
    fn recalibrate_position_after_insert() {
        let mut message = LspMessage::HoverRequest(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "lsp-fuzz://main.rs".parse::<Uri>().unwrap(),
                },
                position: Position::new(3, 4),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        });
        let input_edit = tree_sitter::InputEdit {
            start_byte: 0,
            old_end_byte: 0,
            new_end_byte: 2,
            start_position: tree_sitter::Point::new(1, 0),
            old_end_position: tree_sitter::Point::new(1, 0),
            new_end_position: tree_sitter::Point::new(2, 0),
        };

        calibrate_message(&mut message, input_edit);

        let LspMessage::HoverRequest(params) = message else {
            panic!("expected hover request");
        };
        assert_eq!(
            params.text_document_position_params.position,
            Position::new(4, 4)
        );
    }
}
