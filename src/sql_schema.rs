use crate::{sql_parser_expr::SQLError, sql_parser_space::SQLSpace, util_pratt_parser::*};
use bumpalo::collections::Vec as BVec;

// SQLSchema
#[derive(Debug)]
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

type SQLTag<'a, P> = Tag<'a, SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>, P>;
type SQLRec<'a> = Recursive<'a, SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>>;

pub fn sql_parser_schema<'a>() -> SQLTag<'a, SQLRec<'a>> {
    recurse(|this| {
        let tuple = (this / (Pad::new() / Token::new(",") / Pad::new())) >> (
            |extra: &'a SQLSpace<'a>| extra.bump.alloc(BVec::with_capacity_in(5, extra.bump)),
            |extra, collector: &'a mut BVec<'a, _>, another| { collector.push(another); collector }
        );
        let tuple = Token::new("(") % (tuple / Token::new(")"));
        let tuple = tuple.out(|extra, tuple| extra.bump.alloc(SQLSchema::Tuple { tuple }));
        let i64 = Token::new("i64").out(|extra: &'a SQLSpace<'a>, _| extra.bump.alloc(SQLSchema::I64));
        return (Pad::new()%(tuple ^ i64)/Pad::new()).err(|extra, _| extra.bump.alloc(SQLError::Unknown));
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
        println!("{:?}", parser.parse(input, 0, &space));
    }
}