#[macro_export]
macro_rules! responder {
    ($vis:vis enum $name:ident { $($(#[$($meta:meta)*])? $variant:ident($($ty:tt)*),)* }) => {
        responder!(@enum $vis, $name { $($(#[$($meta)*])? $variant($($ty)*),)* });

        $(
            responder!(@from_impl $name, $variant, $($ty)*);
        )*
    };
    (@enum $vis:vis, $name:ident { $($(#[$($meta:meta)*])? $variant:ident($ty:ty),)* }) => {
        #[derive(Debug, ::rocket::Responder)]
        $vis enum $name {
            $(
                $(#[$($meta)*])?
                $variant($ty),
            )*
        }
    };
    (@from_impl $name:ident, $variant:ident, Box<$ty:ty>) => {
        impl From<$ty> for $name {
            fn from(value: $ty) -> Self {
                $name::$variant(Box::new(value))
            }
        }
    };
    (@from_impl $name:ident, $variant:ident, $ty:ty) => {
        impl From<$ty> for $name {
            fn from(value: $ty) -> Self {
                $name::$variant(value)
            }
        }
    };
}
