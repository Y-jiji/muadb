use crate::{sql_parser_expr::SQLError, sql_parser_space::SQLSpace, util_pratt_parser::*};
use bumpalo::collections::Vec as BVec;

// SQLSchema
pub enum SQLSchema<'a> {
    NamedTuple {
        name:  &'a[&'a str],
        tuple: &'a BVec<'a, &'a SQLSchema<'a>>,
    },
    Tuple {
        tuple: &'a BVec<'a, &'a SQLSchema<'a>>
    },
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
}

// type SQLParser<'a> = Tag<'a, Recursive<'a, SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>>>;

// fn sql_parser_schema<'a>() -> SQLParser<'a> {
//     recurse(|this| {
//         (Token::new("(") + parse_tuple(this) + Token::new(")"))
//             .out(|extra: &'a SQLSpace<'a>, ((_, x), _)| *x)
//             .err(|extra: &'a SQLSpace<'a>, _| extra.bump.alloc(SQLError::Unknown))
//     })
// }

// fn parse_tuple<'a>(parser: SQLParser<'a>) -> impl Parser<'a, O=SQLSchema<'a>, E=SQLError<'a>, X=SQLSpace<'a>> {
//     fn init<'a>(extra: &'a SQLSpace<'a>) -> &'a mut BVec<'a, &'a SQLSchema<'a>> {
//         extra.bump.alloc(BVec::with_capacity_in(5, extra.bump))
//     }
//     fn fold<'a, A>(extra: &'a SQLSpace<'a>, collector: &'a mut BVec<'a, &'a SQLSchema<'a>>, next: &'a (&'a SQLSchema<'a>, &'a A)) -> &'a mut BVec<'a, &'a SQLSchema<'a>> {
//         collector.push(next.0);
//         collector
//     }
//     (parser + Pad::new() % Token::new(",") >> (init, fold))
//     .out(|extra: &'a SQLSpace<'a>, tuple: &'a _| extra.bump.alloc(SQLSchema::Tuple { tuple }))
//     .err(|extra: &'a SQLSpace<'a>, _| extra.bump.alloc(SQLError::Unknown))
// }