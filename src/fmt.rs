use std::fmt;

pub(crate) struct LongEventTitle<'a>(pub(crate) &'a str);

impl fmt::Display for LongEventTitle<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const BASE_TITLE: &str = "Tau's Game Night";
        if self.0.is_empty() {
            f.write_str(BASE_TITLE)
        } else {
            write!(f, "{BASE_TITLE} «{title}»", title = self.0)
        }
    }
}
