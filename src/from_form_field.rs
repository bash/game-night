#[macro_export]
macro_rules! impl_from_form_field {
    ($T:ty) => {
        #[rocket::async_trait]
        impl<'r> rocket::form::FromFormField<'r> for $T {
            fn from_value(field: rocket::form::ValueField<'r>) -> rocket::form::Result<'r, Self> {
                use std::str::FromStr as _;
                Self::from_str(<&'r str>::from_value(field)?)
                    .map_err(|_| rocket::form::Error::validation("not a valid value").into())
            }

            async fn from_data(
                field: rocket::form::DataField<'r, '_>,
            ) -> rocket::form::Result<'r, Self> {
                use std::str::FromStr as _;
                Self::from_str(<&'r str>::from_data(field).await?)
                    .map_err(|_| rocket::form::Error::validation("not a valid value").into())
            }
        }
    };
}
