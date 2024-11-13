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
    };
}

pub(crate) use lsp_messages;
