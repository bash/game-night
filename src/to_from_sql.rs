#[macro_export]
macro_rules! impl_to_from_sql {
    ($T:ty) => {
        impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for $T
        where
            DB: diesel::backend::Backend,
            String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
        {
            fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
                let t = <String as diesel::deserialize::FromSql<diesel::sql_types::Text, DB>>::from_sql(bytes)?;
                Ok(t.as_str().parse()?)
            }
        }

        impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for $T
            where
                for<'c> DB: diesel::backend::Backend<BindCollector<'c> = diesel::query_builder::bind_collector::RawBytesBindCollector<DB>>,
                String: diesel::serialize::ToSql<diesel::sql_types::Text, DB>
        {
            fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, DB>) -> diesel::serialize::Result {
                use std::io::Write as _;
                write!(out, "{self}")?;
                Ok(diesel::serialize::IsNull::No)
            }
        }
    };
}
