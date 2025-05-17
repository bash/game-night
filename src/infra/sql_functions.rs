use diesel::sql_types::Text;

diesel::define_sql_function! {
    fn unixepoch(x: Text) -> BigInt;
}
