#[doc(hidden)]
pub mod private;

pub use tera;

/// Generates a Tera [`Function`][`tera::Function`] from a regular function.
///
/// ## Examples
/// ### Function With Default Value
/// ```
/// use tera_macros::tera_function;
/// use std::iter;
///
/// tera_function! {
///     /// Documentation goes here
///     pub fn ps_prefix(level: usize = 0) -> String {
///         iter::repeat("P.")
///             .take(level + 1)
///             .chain(iter::once("S."))
///             .collect()
///     }
/// }
/// ```
///
/// ### Function Returning a Result
/// ```
/// use tera_macros::{tera_function, tera};
///
/// tera_function! {
///     fn add(a: u64, b: u64) -> Result<u64, tera::Error> {
///         let (value, overflowed) = a.overflowing_add(b);
///         if overflowed {
///             Err(tera::Error::msg("overflow"))
///         } else {
///             Ok(value)
///         }
///     }
/// }
/// ```
///
/// ### Function With No Arguments
/// ```
/// use tera_macros::tera_function;
/// use std::time::SystemTime;
///
/// tera_function! {
///     fn now() -> SystemTime {
///         SystemTime::now()
///     }
/// }
/// ```
#[macro_export]
macro_rules! tera_function {
    ($(#[$meta:meta])* $vis:vis fn $name:ident($($arg_name:ident: $arg_type:ty $(= $default:expr)?),*) -> $ret:ty $block:block) => {
        $(#[$meta])*
        $vis fn $name(_args: &std::collections::HashMap<String, tera::Value>) -> $crate::tera::Result<$crate::tera::Value> {
            fn inner($($arg_name : $arg_type),*) -> $ret $block

            let result = inner($($crate::__tera_macros_extract_arg!("Function", $name, _args, $arg_name: $arg_type $(= $default)?)),*);
            Ok($crate::tera::to_value($crate::__tera_macros_to_result!(result, $crate::tera::Error)?)?)
        }
    };
}

/// Generates a Tera [`Filter`][`tera::Filter`] from a regular function.
///
/// ## Examples
/// ### Filter With No Extra Arguments
/// ```
/// use tera_macros::tera_filter;
///
/// tera_filter! {
///     /// Computes the string's length
///     pub fn len(s: String) -> usize {
///         s.len()
///     }
/// }
/// ```
#[macro_export]
macro_rules! tera_filter {
    ($(#[$meta:meta])* $vis:vis fn $name:ident($value:ident: $value_ty:ty $(,$arg_name:ident: $arg_type:ty $(= $default:expr)?)*) -> $ret:ty $block:block) => {
        $(#[$meta])*
        $vis fn $name(value: &$crate::tera::Value, _args: &std::collections::HashMap<String, tera::Value>) -> $crate::tera::Result<$crate::tera::Value> {
            fn inner($value:$value_ty, $($arg_name : $arg_type),*) -> $ret $block

            let value = $crate::__tera_macros_convert_filter_value!($name, $value_ty, value)?;
            let result = inner(value, $($crate::__tera_macros_extract_arg!("Filter", $name, _args, $arg_name: $arg_type $(= $default)?)),*);
            Ok($crate::tera::to_value($crate::__tera_macros_to_result!(result, $crate::tera::Error)?)?)
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __tera_macros_extract_arg {
    ($kind:literal, $fn_name:ident, $args:expr, $arg_name:ident: $arg_type:ty $(= $default:expr)?) => {{
        let argument = $args
            .get(stringify!($arg_name))
            .map(|v| $crate::__tera_macros_convert_arg!($kind, $fn_name, $arg_name, $arg_type, v))
            .transpose()?;
        $crate::__tera_macros_unwrap_arg!($kind, $fn_name, $arg_name, argument, $($default)?)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __tera_macros_convert_arg {
    ($kind:literal, $fn_name: ident, $arg_name:ident, $arg_type:ty, $value:expr) => {{
        let value = $value;
        $crate::tera::from_value::<$arg_type>(value.clone()).map_err(|_| {
            $crate::tera::Error::msg(format!(
                concat!(
                    $kind,
                    " `",
                    stringify!($fn_name),
                    "` received ",
                    stringify!($arg_name),
                    "={}, expected a ",
                    stringify!($arg_type)
                ),
                value
            ))
        })
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __tera_macros_convert_filter_value {
    ($fn_name:ident, $type:ty, $value:expr) => {{
        let value = $value;
        $crate::tera::from_value::<$type>(value.clone()).map_err(|_| {
            $crate::tera::Error::msg(format!(
                concat!(
                    "Filter `",
                    stringify!($fn_name),
                    "` received value ",
                    " {}, expected a ",
                    stringify!($type)
                ),
                value
            ))
        })
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __tera_macros_unwrap_arg {
    ($kind:literal, $fn_name:ident, $name:ident, $expr:expr,) => {
        $expr.ok_or_else(|| {
            $crate::tera::Error::msg(concat!(
                $kind,
                " `",
                stringify!($fn_name),
                "` requires argument `",
                stringify!($name),
                "`"
            ))
        })?
    };
    ($kind:literal, $fn_name:ident, $name:ident, $expr:expr, $default:expr) => {
        $expr.unwrap_or_else(|| $default)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tera::{Tera, Value};

    #[test]
    fn function_can_be_registered() {
        let mut tera = Tera::default();
        tera.register_function("example", example);

        tera_function! {
            fn example() -> () {
                ()
            }
        }
    }

    #[test]
    fn filter_can_be_registered() {
        let mut tera = Tera::default();
        tera.register_filter("example", example);

        tera_filter! {
            fn example(_input: Value) -> () {
                ()
            }
        }
    }

    #[test]
    fn default_value_is_used() {
        assert_eq!("default", with_default(&HashMap::default()).unwrap());

        tera_function! {
            fn with_default(arg: String = "default".to_string()) -> String {
                arg
            }
        }
    }

    #[test]
    fn default_value_can_be_overwritten() {
        let mut args = HashMap::new();
        args.insert("arg".to_string(), Value::from("custom"));
        assert_eq!("custom", with_default(&args).unwrap());

        tera_function! {
            fn with_default(arg: String = "default".to_string()) -> String {
                arg
            }
        }
    }

    #[test]
    fn errors_on_missing_arg() {
        assert_eq!(
            "Function `with_required` requires argument `arg`",
            with_required(&HashMap::default()).unwrap_err().to_string()
        );
        tera_function! {
            fn with_required(arg: String) -> String {
                arg
            }
        }
    }

    #[test]
    fn errors_on_invalid_argument_type() {
        let mut args = HashMap::default();
        args.insert("arg".to_string(), Value::from(42));
        assert_eq!(
            "Function `with_required` received arg=42, expected a String",
            with_required(&args).unwrap_err().to_string()
        );
        tera_function! {
            fn with_required(arg: String) -> String {
                arg
            }
        }
    }

    #[test]
    fn function_can_return_error() {
        let mut args = HashMap::new();
        args.insert("a".to_string(), Value::from(u64::MAX));
        args.insert("b".to_string(), Value::from(1));

        assert_eq!("overflow", add(&args).unwrap_err().to_string());

        tera_function! {
            fn add(a: u64, b: u64) -> Result<u64, tera::Error> {
                    let (value, overflowed) = a.overflowing_add(b);
                if overflowed {
                    Err(tera::Error::msg("overflow"))
                } else {
                    Ok(value)
                }
            }
        }
    }
}
