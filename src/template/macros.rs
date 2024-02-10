#[macro_export]
macro_rules! tera_function {
    (fn $name:ident($($arg_name:ident : $arg_type:ty $(= $default:expr)?),*) $block:block) => {
        fn $name(__tera_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
            $(
                let $arg_name: Option<$arg_type> = __tera_args
                    .get(stringify!($arg_name))
                    .map(|v| ::rocket_dyn_templates::tera::from_value(v.clone()).map_err(|_|
                        tera::Error::msg(format!(
                            "Function `{name}` received {arg_name}={arg_value}, expected a {arg_type}",
                            name = stringify!($name),
                            arg_name = stringify!($arg_name),
                            arg_type = stringify!($arg_type),
                            arg_value = v,
                        ))))
                    .transpose()?;
                let $arg_name = tera_function!(__unwrap_arg $name, $arg_name, $arg_name, or $($default)?);
            )*
            $block
        }
    };
    (__unwrap_arg $fn_name:ident, $name:ident, $expr:expr, or) => {
        $expr.ok_or_else(|| tera::Error::msg(format!(
            "Function `{}` requires argument `{}`",
            stringify!($fn_name),
            stringify!($name),
        )))?
    };
    (__unwrap_arg $fn_name:ident, $name:ident, $expr:expr, or $default:expr) => {
        $expr.unwrap_or_else(|| $default)
    };
}

#[macro_export]
macro_rules! tera_filter {
    (fn $name:ident($value:ident : $value_ty:ty $(, $arg_name:ident : $arg_type:ty $(= $default:expr)?)*) $block:block) => {
        fn $name(__tera_value: &tera::Value, __tera_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
            let $value: $value_ty = ::rocket_dyn_templates::tera::from_value(__tera_value.clone()).map_err(|_|
                tera::Error::msg(format!(
                    "Filter `{name}` received value {__tera_value}, expected a {ty}",
                    name = stringify!($name),
                    ty = stringify!($value_ty),
            )))?;

            $(
                let $arg_name: Option<$arg_type> = __tera_args
                    .get(stringify!($arg_name))
                    .map(|v| ::rocket_dyn_templates::tera::from_value(v.clone()).map_err(|_|
                        tera::Error::msg(format!(
                            "Filter `{name}` received {arg_name}={arg_value}, expected a {arg_type}",
                            name = stringify!($name),
                            arg_name = stringify!($arg_name),
                            arg_type = stringify!($arg_type),
                            arg_value = v,
                        ))))
                    .transpose()?;
                let $arg_name = tera_filter!(__unwrap_arg $name, $arg_name, $arg_name, or $($default)?);
            )*
            $block
        }
    };
    (__unwrap_arg $fn_name:ident, $name:ident, $expr:expr, or) => {
        $expr.ok_or_else(|| tera::Error::msg(format!(
            "Filter `{}` requires argument `{}`",
            stringify!($fn_name),
            stringify!($name),
        )))?
    };
    (__unwrap_arg $fn_name:ident, $name:ident, $expr:expr, or $default:expr) => {
        $expr.unwrap_or_else(|| $default)
    };
}
