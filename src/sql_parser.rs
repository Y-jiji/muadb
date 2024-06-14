use std::ops::Add;
use crate::sql_parser_combinator::*;

#[derive(Debug, Clone, Copy)]
pub struct SQLInput<'a> {
    string: &'a str,
}

pub struct SQLState {
    bytes: crate::util_bytes::Bytes,
}

#[derive(Debug, Default)]
pub enum SQLError<'a> {
    Token{expect: &'static str, input: &'a str},
    #[default]
    Default,
}