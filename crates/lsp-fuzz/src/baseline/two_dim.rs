use std::{
    borrow::Cow,
    fs,
    hash::{Hash, Hasher},
    iter::once,
    path::PathBuf,
    str::FromStr,
};

use derive_more::Debug;
use derive_new::new as New;
use libafl::{
    generators::Generator,
    inputs::{
        BytesInput, HasTargetBytes, Input, InputToBytes, NautilusBytesConverter, NautilusInput,
    },
    mutators::{MutationResult, Mutator},
    state::{HasMaxSize, HasRand},
};
use libafl_bolts::{HasLen, Named, ownedref::OwnedSlice, rands::Rand};
use lsp_fuzz_grammars::Language;
use lsp_types::{ClientInfo, InitializedParams, TraceValue, Uri};
use serde::{Deserialize, Serialize};

use crate::{
    baseline::{BaselineByteConverter, BaselineInput},
    execution::workspace_observer::HasWorkspace,
    lsp::{self, capabilities::fuzzer_client_capabilities},
    lsp_input::LspInput,
    utils::AflContext,
};

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct TwoDimBaselineInput {
    file_name_extension: String,
    code: BytesInput,
    editor_operations: BaselineInput<NautilusInput>,
}

impl Input for TwoDimBaselineInput {}

impl HasLen for TwoDimBaselineInput {
    fn len(&self) -> usize {
        self.code.len() + self.editor_operations.len()
    }
}

impl HasWorkspace for TwoDimBaselineInput {
    fn workspace_hash(&self) -> u64 {
        let mut hasher = ahash::AHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn setup_workspace(&self, workspace_root: &std::path::Path) -> Result<(), std::io::Error> {
        let mut file_path = workspace_root.join("main");
        file_path.set_extension(&self.file_name_extension);
        fs::write(file_path, self.code.target_bytes().as_ref())?;
        todo!()
    }
}

#[derive(Debug, New)]
pub struct TwoDimInputConverter<'a> {
    workspace_root: PathBuf,
    lang_id: String,
    editor_ops_coverter: BaselineByteConverter<NautilusBytesConverter<'a>>,
}

impl InputToBytes<TwoDimBaselineInput> for TwoDimInputConverter<'_> {
    fn to_bytes<'a>(
        &mut self,
        input: &'a TwoDimBaselineInput,
    ) -> libafl_bolts::ownedref::OwnedSlice<'a, u8> {
        #[allow(
            deprecated,
            reason = "Some language servers (e.g., rust-analyzer) still rely on `root_uri`."
        )]
        let init_request = lsp::LspMessage::Initialize(lsp_types::InitializeParams {
            process_id: None,
            client_info: Some(ClientInfo {
                name: "lsp2d-fuzz".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            root_uri: Some(LspInput::root_uri()),
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: LspInput::root_uri(),
                name: "default_workspace".to_owned(),
            }]),
            capabilities: fuzzer_client_capabilities(),
            trace: Some(TraceValue::Off),
            ..Default::default()
        });
        let initialized_req = lsp::LspMessage::Initialized(InitializedParams {});

        let uri = Uri::from_str(&format!(
            "{}{}.{}",
            LspInput::PROROCOL_PREFIX,
            "main",
            &input.file_name_extension
        ))
        .unwrap();
        let did_open = lsp::LspMessage::DidOpenTextDocument(lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: uri.clone(),
                language_id: self.lang_id.clone(),
                version: 1,
                text: String::from_utf8_lossy(&input.code.target_bytes()).into_owned(),
            },
        });

        let workspace_dir = self
            .workspace_root
            .to_str()
            .expect("`workspace_dir` does not contain valid UTF-8");
        let workspace_dir = if workspace_dir.ends_with('/') {
            Cow::Borrowed(workspace_dir)
        } else {
            Cow::Owned(format!("{workspace_dir}/"))
        };
        let workspace_uri = format!("file://{workspace_dir}");
        let mut id = 0;
        let requests = once(init_request)
            .chain(once(initialized_req))
            .chain(once(did_open))
            .map(|it| it.into_json_rpc(&mut id, Some(&workspace_uri)));

        let bytes: Vec<_> = requests
            .flat_map(|it| it.to_lsp_payload())
            .chain(
                self.editor_ops_coverter
                    .to_bytes(&input.editor_operations)
                    .to_vec(),
            )
            .collect();
        OwnedSlice::from(bytes)
    }
}

#[derive(Debug, New)]
pub struct TwoDimBaselineMutator<CM, EM> {
    code_mutator: CM,
    editor_operation_mutator: EM,
}

impl<CM, EM> Named for TwoDimBaselineMutator<CM, EM> {
    fn name(&self) -> &Cow<'static, str> {
        todo!()
    }
}

impl<CM, EM, State> Mutator<TwoDimBaselineInput, State> for TwoDimBaselineMutator<CM, EM>
where
    CM: Mutator<BytesInput, State>,
    EM: Mutator<BaselineInput<NautilusInput>, State>,
    State: HasMaxSize + HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut TwoDimBaselineInput,
    ) -> Result<MutationResult, libafl::Error> {
        let mut result = MutationResult::Skipped;

        if self.code_mutator.mutate(state, &mut input.code)? == MutationResult::Mutated {
            result = MutationResult::Mutated;
        }
        if self
            .editor_operation_mutator
            .mutate(state, &mut input.editor_operations)?
            == MutationResult::Mutated
        {
            result = MutationResult::Mutated;
        }

        Ok(result)
    }

    fn post_exec(
        &mut self,
        _state: &mut State,
        _new_corpus_id: Option<libafl::corpus::CorpusId>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

#[derive(Debug, New)]
pub struct TwoDimBaselineGenerator<CodeGen, OpsGen> {
    language: Language,
    code_generator: CodeGen,
    editor_ops_generator: OpsGen,
}

impl<State, CodeGen, OpsGen> Generator<TwoDimBaselineInput, State>
    for TwoDimBaselineGenerator<CodeGen, OpsGen>
where
    CodeGen: Generator<BytesInput, State>,
    OpsGen: Generator<BaselineInput<NautilusInput>, State>,
    State: HasRand,
{
    fn generate(&mut self, state: &mut State) -> Result<TwoDimBaselineInput, libafl::Error> {
        let code = self.code_generator.generate(state)?;
        let editor_operations = self.editor_ops_generator.generate(state)?;
        let file_name_extension = state
            .rand_mut()
            .choose(self.language.file_extensions())
            .afl_context("No file extension chosen")?
            .to_owned();
        Ok(TwoDimBaselineInput {
            file_name_extension,
            code,
            editor_operations,
        })
    }
}
