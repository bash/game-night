use crate::impl_to_from_sql;
use anyhow::bail;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::sql_types::Text;
use std::fmt;
use std::str::FromStr;

macro_rules! symbols { ($($symbol:literal,)*) => { &[$(AstronomicalSymbol($symbol),)*] }; }
pub(crate) static ASTRONOMICAL_SYMBOLS: &[AstronomicalSymbol] = symbols! {
    "☉", "☾", "☿", "♀",
    "♂", "♂I", "♂II",
    "♃", "♃I", "♃II", "♃III", "♃IV",
    "♄", "♄I", "♄II", "♄III", "♄IV", "♄V", "♄VI", "♄VII", "♄VIII",
    "♅", "♅I", "♅II", "♅III", "♅IV", "♅V",
    "♆", "♆I", "♆II", /* No symbols for neptunian moons 3-7 */ "♆VIII",
    "⚳", "⚴", "⚵", "⚶",
    "⯓", "⯓I", "⯓II", "⯓III", "⯓IV", "⯓V",
    "⯰", "⯰I", "⯲", "🜨", "🝻", "🝼", "🝽", "🝾", "🝿",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = Text)]
pub(crate) struct AstronomicalSymbol(&'static str);

impl_to_from_sql! { AstronomicalSymbol }

impl AstronomicalSymbol {
    pub(crate) fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for AstronomicalSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl<'a> TryFrom<&'a str> for AstronomicalSymbol {
    type Error = anyhow::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match ASTRONOMICAL_SYMBOLS.binary_search_by_key(&value, |s| s.as_str()) {
            Ok(index) => Ok(ASTRONOMICAL_SYMBOLS[index]),
            Err(_) => bail!("{value} is not a valid astronomical symbol"),
        }
    }
}

impl FromStr for AstronomicalSymbol {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value)
    }
}

impl rand::distr::Distribution<AstronomicalSymbol> for rand::distr::StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> AstronomicalSymbol {
        *rand::distr::slice::Choose::new(ASTRONOMICAL_SYMBOLS)
            .expect("symbols are not empty")
            .sample(rng)
    }
}

#[rocket::async_trait]
impl<'r> rocket::form::FromFormField<'r> for AstronomicalSymbol {
    fn from_value(field: rocket::form::ValueField<'r>) -> rocket::form::Result<'r, Self> {
        rocket_try_from(<&'r str>::from_value(field)?)
    }

    async fn from_data(field: rocket::form::DataField<'r, '_>) -> rocket::form::Result<'r, Self> {
        rocket_try_from(<&'r str>::from_data(field).await?)
    }
}

fn rocket_try_from(value: &str) -> rocket::form::Result<AstronomicalSymbol> {
    AstronomicalSymbol::try_from(value)
        .map_err(|_| rocket::form::Error::validation("not a valid symbol").into())
}

impl From<AstronomicalSymbol> for &'static str {
    fn from(value: AstronomicalSymbol) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_are_sorted() {
        assert!(ASTRONOMICAL_SYMBOLS.is_sorted_by_key(|s| s.as_str()));
    }
}
