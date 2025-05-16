macro_rules! lsp_responses {
    (
        $(#[$outer:meta])*
        $vis: vis enum $type_name: ident {
            $( request::$res_variant: ident ),*
        }
    ) => {

        $(#[$outer])*
        $vis enum $type_name {
            $( $res_variant(<::lsp_types::request::$res_variant as ::lsp_types::request::Request>::Result) ),*
        }

        impl $type_name {

            /// Returns the method name of the request.
            pub const fn method<'a>(&self) -> &'a str {
                match self {
                    $( Self::$res_variant(_) => <::lsp_types::request::$res_variant as ::lsp_types::request::Request>::METHOD ),*
                }
            }

            pub fn try_from_json(method: impl AsRef<str>, json: serde_json::Value) -> Result<Self, crate::lsp::message::MessageDecodeError> {
                let result = match method.as_ref() {
                    $(<::lsp_types::request::$res_variant as ::lsp_types::request::Request>::METHOD => {
                        Self::$res_variant(serde_json::from_value(json)?)
                    })*,
                    _ => return Err(crate::lsp::message::MessageDecodeError::MethodMismatch),
                };
                Ok(result)
            }

        }

        $(
            impl crate::lsp::LspRequestMeta for ::lsp_types::request::$res_variant {
                type Response = <Self as ::lsp_types::request::Request>::Result;
            }


            impl crate::lsp::MessageResponse<::lsp_types::request::$res_variant> for <::lsp_types::request::$res_variant as ::lsp_types::request::Request>::Result {
                fn from_response_ref(response: &$type_name) -> Option<&Self> {
                    if let crate::lsp::$type_name::$res_variant(result) = response {
                        Some(result)
                    } else {
                        None
                    }
                }
            }
        )*

    };
}

macro_rules! lsp_messages {
    (
        $(#[$outer:meta])*
        $vis: vis enum $type_name: ident {
            $(
                $( request::$req_variant: ident )?
                $( notification::$not_variant: ident )?
            ),*
        }
    ) => {

        $(#[$outer])*
        $vis enum $type_name {
            $(
                $( $req_variant(<::lsp_types::request::$req_variant as ::lsp_types::request::Request>::Params) )?
                $( $not_variant(<::lsp_types::notification::$not_variant as ::lsp_types::notification::Notification>::Params) )?
            ),*
        }

        impl $type_name {

            /// Returns the method name of the request.
            pub const fn method<'a>(&self) -> &'a str {
                match self {
                    $(
                        $( Self::$req_variant(_) => <::lsp_types::request::$req_variant as ::lsp_types::request::Request>::METHOD )?
                        $( Self::$not_variant(_) => <::lsp_types::notification::$not_variant as ::lsp_types::notification::Notification>::METHOD )?
                    ),*
                }
            }

            /// Returns if the message is a request.
            pub const fn is_request(&self) -> bool {
                match self {
                    $(
                        $( Self::$req_variant(_) => true)?
                        $( Self::$not_variant(_) => false)?
                    ),*
                }
            }

            /// Returns if the message is a notification.
            pub const fn is_notification(&self) -> bool {
                match self {
                    $(
                        $( Self::$req_variant(_) => false)?
                        $( Self::$not_variant(_) => true)?
                    ),*
                }
            }

            /// Creates a JSON-RPC request object.
            fn into_json(self) -> (&'static str, serde_json::Value) {
                match self {
                    $(
                        $(
                            Self::$req_variant(params) =>  (
                                <::lsp_types::request::$req_variant as ::lsp_types::request::Request>::METHOD,
                                serde_json::json!(params)
                            )
                        )?
                        $(
                            Self::$not_variant(params) => (
                                <::lsp_types::notification::$not_variant as ::lsp_types::notification::Notification>::METHOD,
                                serde_json::json!(params)
                            )
                        )?
                    ),*
                }
            }

            pub fn from_params<M>(params: M::Params) -> Self
                where
                    M: crate::lsp::LspMessageMeta,
                    M::Params: crate::lsp::MessageParam<M>
            {
                <M::Params as crate::lsp::MessageParam<M>>::into_message(params)
            }

            pub fn try_from_json(method: impl AsRef<str>, json: serde_json::Value) -> Result<Self, crate::lsp::message::MessageDecodeError> {
                let result = match method.as_ref() {
                    $(
                        $( <::lsp_types::request::$req_variant as ::lsp_types::request::Request>::METHOD => {
                            Self::$req_variant(serde_json::from_value(json)?)
                        })?
                        $( <::lsp_types::notification::$not_variant as ::lsp_types::notification::Notification>::METHOD => {
                            Self::$not_variant(serde_json::from_value(json)?)
                        })?
                    ),*
                    _ => return Err(crate::lsp::message::MessageDecodeError::MethodMismatch),
                };
                Ok(result)
            }
        }

        impl crate::lsp::code_context::CodeContextRef for $type_name {
            fn document(&self) -> Option<&lsp_types::TextDocumentIdentifier> {
                match self {
                    $(
                        $( Self::$req_variant(params) => params.document())?
                        $( Self::$not_variant(params) => params.document())?
                    ),*
                }
            }

            fn position(&self) -> Option<&lsp_types::Position> {
                match self {
                    $(
                        $( Self::$req_variant(params) => params.position())?
                        $( Self::$not_variant(params) => params.position())?
                    ),*
                }
            }

            fn range(&self) -> Option<&lsp_types::Range> {
                match self {
                    $(
                        $( Self::$req_variant(params) => params.range())?
                        $( Self::$not_variant(params) => params.range())?
                    ),*
                }
            }

            fn document_mut(&mut self) -> Option<&mut lsp_types::TextDocumentIdentifier> {
                match self {
                    $(
                        $( Self::$req_variant(params) => params.document_mut())?
                        $( Self::$not_variant(params) => params.document_mut())?
                    ),*
                }
            }

            fn position_mut(&mut self) -> Option<&mut lsp_types::Position> {
                match self {
                    $(
                        $( Self::$req_variant(params) => params.position_mut())?
                        $( Self::$not_variant(params) => params.position_mut())?
                    ),*
                }
            }

            fn range_mut(&mut self) -> Option<&mut lsp_types::Range> {
                match self {
                    $(
                        $( Self::$req_variant(params) => params.range_mut())?
                        $( Self::$not_variant(params) => params.range_mut())?
                    ),*
                }
            }
        }

        $(
            $(
                impl crate::lsp::LspMessageMeta for ::lsp_types::request::$req_variant {
                    const METHOD: &'static str = <Self as ::lsp_types::request::Request>::METHOD;
                    type Params = <Self as ::lsp_types::request::Request>::Params;
                }

                impl crate::lsp::MessageParam<::lsp_types::request::$req_variant> for <::lsp_types::request::$req_variant as ::lsp_types::request::Request>::Params {
                    fn into_message(self) -> $type_name {
                        $type_name::$req_variant(self)
                    }

                    fn from_message_ref(message: &$type_name) -> Option<&Self> {
                        if let LspMessage::$req_variant(params) = message {
                            Some(params)
                        } else {
                            None
                        }
                    }
                }
            )?
            $(
                impl crate::lsp::LspMessageMeta for ::lsp_types::notification::$not_variant {
                    const METHOD: &'static str = <Self as ::lsp_types::notification::Notification>::METHOD;
                    type Params = <Self as ::lsp_types::notification::Notification>::Params;
                }

                impl crate::lsp::MessageParam<::lsp_types::notification::$not_variant> for <::lsp_types::notification::$not_variant as ::lsp_types::notification::Notification>::Params {
                    fn into_message(self) -> $type_name {
                        $type_name::$not_variant(self)
                    }

                    fn from_message_ref(message: &$type_name) -> Option<&Self> {
                        if let LspMessage::$not_variant(params) = message {
                            Some(params)
                        } else {
                            None
                        }
                    }
                }
            )?
        )*
    };
}

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
                $(AppendRandomlyGeneratedMessage::<request::$req_variant, State>, )?
                $(AppendRandomlyGeneratedMessage::<notification::$not_variant, State>, )?
            )*
        ];

        $(#[$outer])*
        $vis fn $fn_name<State>(config: &GeneratorsConfig) -> $return_ty<State>
        where
            State: libafl::state::HasRand + libafl::common::HasMetadata + libafl::state::HasCurrentTestcase<LspInput> + 'static
        {
            tuple_list::tuple_list![
                $(
                    $(AppendRandomlyGeneratedMessage::<request::$req_variant, State>::with_predefined(config),)?
                    $(AppendRandomlyGeneratedMessage::<notification::$not_variant, State>::with_predefined(config),)?
                )*
            ]
        }
    };
}

macro_rules! prop_mutator {
    ($vis: vis impl $mutator_ty_name: ident for $input_ty_name: ident :: $field:ident type $field_ty: ty) => {
        const __OFFSET: usize = ::core::mem::offset_of!($input_ty_name, $field);

        #[automatically_derived]
        impl crate::mutators::HasMutProp<__OFFSET> for $input_ty_name {
            type PropType = $field_ty;

            #[inline]
            fn get_field(&mut self) -> &mut Self::PropType {
                &mut self.$field
            }
        }
        $vis type $mutator_ty_name<PM> = crate::mutators::PropMutator<PM, __OFFSET>;
    };
}

#[allow(unused_macros)]
macro_rules! afl_oops {
    ($msg:literal $(,)?) => {
        return Err(libafl::Error::unknown(format!($msg)))
    };
    ($err:expr $(,)?) => {
        return Err(libafl::Error::unknown(format!($err)))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(libafl::Error::unknown(format!($fmt, $($arg)*)))
    };
}

#[allow(unused_imports)]
pub(crate) use {afl_oops, append_randoms, lsp_messages, lsp_responses, prop_mutator};
