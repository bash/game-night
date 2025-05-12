use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;

#[derive(Debug)]
pub(crate) enum Error {
    Syn(syn::Error),
    Diagnostic(Diagnostic),
}

impl From<syn::Error> for Error {
    fn from(value: syn::Error) -> Self {
        Self::Syn(value)
    }
}

impl From<Diagnostic> for Error {
    fn from(value: Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl Error {
    pub(crate) fn emit_as_item_tokens(self) -> TokenStream {
        match self {
            Error::Syn(error) => error.into_compile_error(),
            Error::Diagnostic(diagnostic) => diagnostic.emit_as_item_tokens(),
        }
    }
}
