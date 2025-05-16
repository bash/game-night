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

        impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for $T
        {
            fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>) -> diesel::serialize::Result {
                out.set_value(self.to_string());
                Ok(diesel::serialize::IsNull::No)
            }
        }
    };
}
