#[macro_export]
macro_rules! email_template {
    (#[template(html_path = $html_path:literal, txt_path = $txt_path:literal)]
        $(#[$($meta:meta)*])?
    $vis:vis struct $name:ident $(<$($lt:lifetime),+>)? {$(
        $(#[$($field_meta:meta)*])?
        $field_vis:vis $field_name:ident : $ty:ty
    ),* $(,)?}) => {
        $(#[$($meta)*])?
        $vis struct $name $(<$($lt),+>)? {
            $(
                $(#[$($field_meta)*])?
                $field_vis $field_name : $ty
            ),*
        }

        impl$(<$($lt),+>)? $crate::email::EmailTemplate for $name $(<$($lt),+>)? {
            fn render(&self) -> ::askama::Result<$crate::email::EmailBody> {
                use ::askama::Template as _;

                #[derive(::askama::Template)]
                #[template(path = $html_path)]
                struct Html<'__inner $($(, $lt)+)?>(&'__inner $name $(<$($lt),+>)?);
                impl<'__inner $($(, $lt)+)?> ::std::ops::Deref for Html<'__inner $($(, $lt)+)?> {
                    type Target = $name $(<$($lt),+>)?;
                    fn deref(&self) -> &Self::Target {
                        &self.0
                    }
                }

                #[derive(::askama::Template)]
                #[template(path = $txt_path)]
                struct Plain<'__inner $($(, $lt)+)?>(&'__inner $name $(<$($lt),+>)?);

                impl<'__inner $($(, $lt)+)?> ::std::ops::Deref for Plain<'__inner $($(, $lt)+)?> {
                    type Target = $name $(<$($lt),+>)?;
                    fn deref(&self) -> &Self::Target {
                        &self.0
                    }
                }

                Ok($crate::email::EmailBody {
                    plain: Plain(self).render()?,
                    html: Html(self).render()?,
                })
            }
        }
    };
}
