use crate::{sql_parser_expr::SQLError, sql_parser_space::SQLSpace, util_pratt_parser::*};
use bumpalo::collections::Vec as BVec;

// SQLSchema
#[derive(Debug, Clone)]
pub enum SQLSchema<'a> {
    NamedTuple {
        name:  BVec<'a, &'a str>,
        tuple: BVec<'a, SQLSchema<'a>>,
    },
    Tuple {
        tuple: BVec<'a, SQLSchema<'a>>
    },
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
}

type SQLTag<'a, P> = Tag<SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>, P>;
type SQLRec<'a> = Recursive<SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>>;

pub fn sql_parser_schema<'a>() -> impl Parser<SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>> {
    recurse(|this| {
        let tuple = (this / (Pad::new() / Token::new(",") / Pad::new())) >> (
            |extra: SQLSpace<'a>| BVec::with_capacity_in(5, extra.bump),
            |extra: SQLSpace<'a>, mut collector: BVec<'a, _>, another: SQLSchema<'a>| { collector.push(another); collector }
        );
        let tuple = Token::new("(") % (tuple / Token::new(")"));
        let tuple = tuple.out(|extra, tuple| SQLSchema::Tuple { tuple });
        let i64 = Token::new("i64").out(|extra: SQLSpace<'a>, _| SQLSchema::I64);
        (Pad::new()%(tuple ^ i64)/Pad::new()).err(|extra, _| SQLError::Unknown)
    })
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;
    use super::*;

    #[test]
    fn parse_tuple() {
        let input = "( i64 , )";
        let bump = Bump::new();
        let space = SQLSpace::new(&bump, input);
        let parser = sql_parser_schema();
        println!("{:?}", parser.parse(input, 0, space));
    }
}