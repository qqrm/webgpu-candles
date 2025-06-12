/// Helper macro to define functions returning global signals.
/// Usage: `global_signal!(fn_name, field: Type);`
#[macro_export]
macro_rules! global_signal {
    ($vis:vis $name:ident, $field:ident : $ty:ty) => {
        $vis fn $name() -> ::leptos::RwSignal<$ty> {
            $crate::global_state::globals().$field
        }
    };
}

/// Generate multiple global signal accessors at once.
///
/// Usage:
/// `global_signals! {
///     pub fn1 => field1: Type1,
///     fn2 => field2: Type2,
/// }`
#[macro_export]
macro_rules! global_signals {
    ( $( $vis:vis $name:ident => $field:ident : $ty:ty ),+ $(,)? ) => {
        $(
            $vis fn $name() -> ::leptos::RwSignal<$ty> {
                $crate::global_state::globals().$field
            }
        )+
    };
}
