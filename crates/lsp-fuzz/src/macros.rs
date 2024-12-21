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
        use lsp_types::request::{self, Request};
        use lsp_types::notification::{self, Notification};
        use crate::lsp::{LspMessage, MessageParam, LocalizeToWorkspace};

        $(#[$outer])*
        $vis enum $type_name {
            $(
                $( $req_variant(<request::$req_variant as Request>::Params) )?
                $( $not_variant(<notification::$not_variant as Notification>::Params) )?
            ),*
        }

        impl $type_name {

            /// Returns the method name of the request.
            pub const fn method<'a>(&self) -> &'a str {
                match self {
                    $(
                        $( Self::$req_variant(_) => <request::$req_variant as Request>::METHOD )?
                        $( Self::$not_variant(_) => <notification::$not_variant as Notification>::METHOD )?
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
            pub fn as_json(&self) -> (&'static str, serde_json::Value) {
                match self {
                    $(
                        $(
                            Self::$req_variant(params) =>  (
                                <request::$req_variant as Request>::METHOD,
                                serde_json::json!(params)
                            )
                        )?
                        $(
                            Self::$not_variant(params) => (
                                <notification::$not_variant as Notification>::METHOD,
                                serde_json::json!(params)
                            )
                        )?
                    ),*
                }
            }

            pub fn from_params<M>(params: M::Params) -> Self
                where
                    M: LspMessage,
                    M::Params: MessageParam<M>
            {
                M::Params::into_message(params)
            }

            pub fn with_workspace_dir(mut self, workspace_dir: &str) -> Self {
                match self {
                    $(
                        $( Self::$req_variant(ref mut params) => params.localize(workspace_dir) )?
                        $( Self::$not_variant(ref mut params) => params.localize(workspace_dir) )?
                    ),*
                };
                self
            }
        }

        $(
            $(
                impl LspMessage for request::$req_variant {
                    const METHOD: &'static str = <Self as Request>::METHOD;
                    type Params = <Self as Request>::Params;
                }

                impl MessageParam<request::$req_variant> for <request::$req_variant as Request>::Params {
                    fn into_message(self) -> $type_name {
                        $type_name::$req_variant(self)
                    }
                }
            )?
            $(
                impl LspMessage for notification::$not_variant {
                    const METHOD: &'static str = <Self as Notification>::METHOD;
                    type Params = <Self as Notification>::Params;
                }

                impl MessageParam<notification::$not_variant> for <notification::$not_variant as Notification>::Params {
                    fn into_message(self) -> $type_name {
                        $type_name::$not_variant(self)
                    }
                }
            )?
        )*
    };
}

macro_rules! append_randoms {
    (
        $(#[$outer:meta])*
        $vis: vis fn $fn_name:ident() -> $return_ty: ident {
            $(
                $( request::$req_variant: ident )?
                $( notification::$not_variant: ident )?
            ),*
        }
    ) => {
        use lsp_types::{request, notification};
        $vis type $return_ty<S> = tuple_list::tuple_list_type![
            $(
                $(AppendRandomlyGeneratedMessage::<request::$req_variant, S>, )?
                $(AppendRandomlyGeneratedMessage::<notification::$not_variant, S>, )?
            )*
        ];

        $(#[$outer])*
        $vis fn $fn_name<S>() -> $return_ty<S>
        where
            S: libafl::state::HasRand + 'static
        {
            tuple_list::tuple_list![
                $(
                    $(AppendRandomlyGeneratedMessage::<request::$req_variant, S>::with_predefined(),)?
                    $(AppendRandomlyGeneratedMessage::<notification::$not_variant, S>::with_predefined(),)?
                )*
            ]
        }
    };
}

/// Implements the `LocalizeToWorkspace` trait for the given type
/// by calling the `localize` method on the specified fields of the type.
macro_rules! impl_localize {
    (
        $type: ty
        $(;
            $( $field:ident ),*
        )?
    ) => {
        #[automatically_derived]
        impl LocalizeToWorkspace for $type {

            #[inline]
            fn localize(&mut self, workspace_dir: &str) {
                $(
                    $( self.$field.localize(workspace_dir);)*
                )?
            }
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

pub(crate) use {append_randoms, impl_localize, lsp_messages, prop_mutator};
