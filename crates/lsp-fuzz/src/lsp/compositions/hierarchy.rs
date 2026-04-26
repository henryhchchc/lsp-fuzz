#[allow(
    clippy::wildcard_imports,
    reason = "LSP parameter types are dense here"
)]
use lsp_types::*;

compose! {
    TypeHierarchySupertypesParams {
        item: TypeHierarchyItem,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

compose! {
    TypeHierarchySubtypesParams {
        item: TypeHierarchyItem,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

compose! {
    CallHierarchyIncomingCallsParams {
        item: CallHierarchyItem,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}

compose! {
    CallHierarchyOutgoingCallsParams {
        item: CallHierarchyItem,
        work_done_progress_params: WorkDoneProgressParams,
        partial_result_params: PartialResultParams
    }
}
