use crate::parser_space::SQLSpace;
use muadb_util::*;

// SQLError
#[derive(Debug, Clone, Copy)]
pub enum SQLError<'a> {
    MismatchToken(usize /* offset */, &'a str /* expected */),
    Unknown,
    Visited,
    Merge(&'a SQLError<'a>, &'a SQLError<'a>),
    CannotFindIdent(usize),
}

impl<'a> MergeIn<SQLSpace<'a>> for SQLError<'a> {
    fn merge(self, with: Self, x: &mut SQLSpace<'a>) -> Self {
        SQLError::Merge(x.bump.alloc(self), x.bump.alloc(with))
    }
}

impl<'a> Visited for SQLError<'a> {
    fn visited() -> Self { Self::Visited }
}