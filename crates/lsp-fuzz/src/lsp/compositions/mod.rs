macro_rules! compose {
    (
        $(#[$outer:meta])*
        $output: ty {
            $( $field: ident: $field_type: ty ),*
        }
    ) => {
        $(#[$outer])*
        impl crate::lsp::Compose for $output {
            type Components = tuple_list::tuple_list_type![
                $( $field_type ),*
            ];

            #[inline]
            fn compose(components: Self::Components) -> Self {
                let ( $( $field, )* ) = tuple_list::TupleList::into_tuple(components);
                Self { $( $field ),* }
            }
        }
    };
}

mod diagnostics;
mod formatting;
mod hierarchy;
mod navigation;
mod symbols;
mod tracing_misc;
mod workspace;
