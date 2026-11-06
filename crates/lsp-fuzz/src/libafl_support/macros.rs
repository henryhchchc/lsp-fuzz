macro_rules! prop_mutator {
    ($vis: vis impl $mutator_ty_name: ident for $input_ty_name: ident :: $field:ident type $field_ty: ty) => {
        const __OFFSET: usize = ::core::mem::offset_of!($input_ty_name, $field);

        #[automatically_derived]
        impl crate::libafl_support::mutators::HasMutProp<__OFFSET> for $input_ty_name {
            type PropType = $field_ty;

            #[inline]
            fn get_field(&mut self) -> &mut Self::PropType {
                &mut self.$field
            }
        }
        $vis type $mutator_ty_name<PM> = crate::libafl_support::mutators::PropMutator<PM, __OFFSET>;
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
pub(crate) use {afl_oops, prop_mutator};
