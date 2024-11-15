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
        use crate::lsp::{LspMessage, IntoMessage};

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
        }

        $(
            $(
                impl LspMessage for request::$req_variant {
                    const METHOD: &'static str = <Self as Request>::METHOD;
                    type Params = <Self as Request>::Params;
                }

                impl IntoMessage<request::$req_variant> for request::$req_variant {
                    fn into_message(params: <request::$req_variant as Request>::Params) -> $type_name {
                        $type_name::$req_variant(params)
                    }
                }
            )?
            $(
                impl LspMessage for notification::$not_variant {
                    const METHOD: &'static str = <Self as Notification>::METHOD;
                    type Params = <Self as Notification>::Params;
                }

                impl IntoMessage<notification::$not_variant> for notification::$not_variant {
                    fn into_message(params: <notification::$not_variant as Notification>::Params) -> $type_name {
                        $type_name::$not_variant(params)
                    }
                }
            )?
        )*

    };
}

pub(crate) use lsp_messages;
