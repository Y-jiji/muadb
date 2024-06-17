use crate::util_pratt_parser::*;

// SQLError
#[derive(Debug, Clone, Copy)]
pub enum SQLError<'a> {
    UndefinedSymbol(usize, &'a str),
    Unknown,
    Visited,
}

impl<'a> Visited for SQLError<'a> {
    fn visited() -> Self { Self::Visited }
}