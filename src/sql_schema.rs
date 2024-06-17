use crate::{sql_error::SQLError, sql_parser_space::SQLSpace, util_pratt_parser::*};
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
type SQLDyn<'a> = Box<dyn Parser<(), SQLError<'a>, SQLSpace<'a>>>;


pub fn sql_parser_schema<'a>() -> impl Parser<SQLSchema<'a>, SQLError<'a>, SQLSpace<'a>> {
    let sql_token = |token: &'static str| Token::new(token).err(|_, at, _| SQLError::UndefinedSymbol(at, token)).erase();
    recurse(|this| {
        let tuple = this.pad() / sql_token(",");
        let tuple = tuple >> (
            |extra: &mut SQLSpace<'a>| BVec::with_capacity_in(5, extra.bump),
            |extra: &mut SQLSpace<'a>, mut collector: BVec<'a, _>, another: SQLSchema<'a>| { collector.push(another); collector }
        );
        let tuple = Token::new("(") % (tuple / Token::new(")"));
        let tuple = tuple.out(|extra, tuple| SQLSchema::Tuple { tuple }).err(|_, _, _| SQLError::Unknown);
        let i64 = Token::new("i64").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::I64).err(|_, _, _| SQLError::Unknown);
        (tuple ^ i64).pad().err(|_, _, _| SQLError::Unknown)
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
        let mut space = SQLSpace::new(&bump, input);
        let parser = sql_parser_schema();
        println!("{:?}", parser.parse(input, 0, &mut space));
    }
}