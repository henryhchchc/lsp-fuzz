use lsp_types::*;

pub trait CodeContextRef {
    fn document(&self) -> Option<&TextDocumentIdentifier>;
    fn position(&self) -> Option<&Position>;
    fn range(&self) -> Option<&lsp_types::Range>;

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier>;
    fn position_mut(&mut self) -> Option<&mut Position>;
    fn range_mut(&mut self) -> Option<&mut lsp_types::Range>;
}

#[trait_gen::trait_gen(T ->
    (),
    CallHierarchyIncomingCallsParams,
    CallHierarchyOutgoingCallsParams,
    CancelParams,
    CodeAction,
    CodeLens,
    CompletionItem,
    CreateFilesParams,
    DeleteFilesParams,
    DocumentLink,
    ExecuteCommandParams,
    InitializeParams,
    InitializedParams,
    RenameFilesParams,
    TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams,
    InlayHint,
    WorkspaceDiagnosticParams,
    WorkspaceSymbolParams,
    WorkspaceSymbol,
    DidChangeWatchedFilesParams,
    DidChangeNotebookDocumentParams,
    SetTraceParams,
    DidSaveNotebookDocumentParams,
    WorkDoneProgressCancelParams,
    DidOpenTextDocumentParams,
    DidChangeConfigurationParams,
    LogTraceParams,
    DidOpenNotebookDocumentParams,
    DidCloseTextDocumentParams,
    DidCloseNotebookDocumentParams,
    DidChangeTextDocumentParams,
    DidChangeWorkspaceFoldersParams
)]
impl CodeContextRef for T {
    fn document(&self) -> Option<&TextDocumentIdentifier> {
        None
    }

    fn position(&self) -> Option<&Position> {
        None
    }

    fn range(&self) -> Option<&lsp_types::Range> {
        None
    }

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier> {
        None
    }

    fn position_mut(&mut self) -> Option<&mut Position> {
        None
    }

    fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
        None
    }
}

#[trait_gen::trait_gen(T ->
    DocumentLinkParams,
    DocumentColorParams,
    DocumentDiagnosticParams,
    FoldingRangeParams,
    SemanticTokensParams,
    SemanticTokensDeltaParams,
    CodeLensParams,
    ColorPresentationParams,
    SelectionRangeParams,
    WillSaveTextDocumentParams,
    DocumentSymbolParams,
    DidSaveTextDocumentParams,
    DocumentFormattingParams,
)]
impl CodeContextRef for T {
    fn document(&self) -> Option<&TextDocumentIdentifier> {
        Some(&self.text_document)
    }

    fn position(&self) -> Option<&Position> {
        None
    }

    fn range(&self) -> Option<&lsp_types::Range> {
        None
    }

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier> {
        Some(&mut self.text_document)
    }

    fn position_mut(&mut self) -> Option<&mut Position> {
        None
    }

    fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
        None
    }
}

#[trait_gen::trait_gen(T ->
    CallHierarchyPrepareParams,
    DocumentHighlightParams,
    GotoDefinitionParams,
    HoverParams,
    LinkedEditingRangeParams,
    MonikerParams,
    SignatureHelpParams,
    TypeHierarchyPrepareParams,
)]
impl CodeContextRef for T {
    fn document(&self) -> Option<&TextDocumentIdentifier> {
        Some(&self.text_document_position_params.text_document)
    }

    fn position(&self) -> Option<&Position> {
        Some(&self.text_document_position_params.position)
    }

    fn range(&self) -> Option<&lsp_types::Range> {
        None
    }

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier> {
        Some(&mut self.text_document_position_params.text_document)
    }

    fn position_mut(&mut self) -> Option<&mut Position> {
        Some(&mut self.text_document_position_params.position)
    }

    fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
        None
    }
}

#[trait_gen::trait_gen(T ->
    RenameParams,
    CompletionParams,
    ReferenceParams,
    DocumentOnTypeFormattingParams,
)]
impl CodeContextRef for T {
    fn document(&self) -> Option<&TextDocumentIdentifier> {
        Some(&self.text_document_position.text_document)
    }

    fn position(&self) -> Option<&Position> {
        Some(&self.text_document_position.position)
    }

    fn range(&self) -> Option<&lsp_types::Range> {
        None
    }

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier> {
        Some(&mut self.text_document_position.text_document)
    }

    fn position_mut(&mut self) -> Option<&mut Position> {
        Some(&mut self.text_document_position.position)
    }

    fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
        None
    }
}

#[trait_gen::trait_gen(T ->
    DocumentRangeFormattingParams,
    InlayHintParams,
    InlineValueParams,
    SemanticTokensRangeParams,
    CodeActionParams,
)]
impl CodeContextRef for T {
    fn document(&self) -> Option<&TextDocumentIdentifier> {
        Some(&self.text_document)
    }

    fn position(&self) -> Option<&Position> {
        None
    }

    fn range(&self) -> Option<&lsp_types::Range> {
        Some(&self.range)
    }

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier> {
        Some(&mut self.text_document)
    }

    fn position_mut(&mut self) -> Option<&mut Position> {
        None
    }

    fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
        Some(&mut self.range)
    }
}

impl CodeContextRef for TextDocumentPositionParams {
    fn document(&self) -> Option<&TextDocumentIdentifier> {
        Some(&self.text_document)
    }

    fn position(&self) -> Option<&Position> {
        Some(&self.position)
    }

    fn range(&self) -> Option<&lsp_types::Range> {
        None
    }

    fn document_mut(&mut self) -> Option<&mut TextDocumentIdentifier> {
        Some(&mut self.text_document)
    }

    fn position_mut(&mut self) -> Option<&mut Position> {
        Some(&mut self.position)
    }

    fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
        None
    }
}
