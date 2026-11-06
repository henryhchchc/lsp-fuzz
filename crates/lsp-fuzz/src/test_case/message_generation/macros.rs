macro_rules! append_randoms {
    (
        $(#[$outer:meta])*
        $vis: vis fn $fn_name:ident(config: &GeneratorsConfig) -> $return_ty: ident {
            $(
                $( request::$req_variant: ident )?
                $( notification::$not_variant: ident )?
            ),*
        }
    ) => {
        #[allow(unused_imports, reason = "The imports are used in the generated code.")]
        use lsp_types::{request, notification};
        $vis type $return_ty<State> = tuple_list::tuple_list_type![
            $(
                $($crate::test_case::message_generation::registration::AppendMessage::<request::$req_variant, State>, )?
                $($crate::test_case::message_generation::registration::AppendMessage::<notification::$not_variant, State>, )?
            )*
        ];

        $(#[$outer])*
        $vis fn $fn_name<State>(config: &$crate::test_case::message_generation::GeneratorsConfig) -> $return_ty<State>
        where
            State: libafl::state::HasRand + libafl::common::HasMetadata + libafl::state::HasCurrentTestcase<$crate::test_case::LspInput> + 'static
        {
            tuple_list::tuple_list![
                $(
                    $($crate::test_case::message_generation::registration::AppendMessage::<request::$req_variant, State>::with_predefined(config),)?
                    $($crate::test_case::message_generation::registration::AppendMessage::<notification::$not_variant, State>::with_predefined(config),)?
                )*
            ]
        }
    };
}

#[allow(unused_imports)]
pub(crate) use append_randoms;
