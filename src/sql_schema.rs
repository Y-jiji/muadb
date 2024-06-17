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

pub fn sql_parser_schema<'a>() -> SQLTag<'a, SQLRec<'a>> {
    let sql_token = |token: &'static str| Token::new(token).err(|_, at, _| SQLError::UndefinedSymbol(at, token));
    recurse(|this| {
        let tuple = this.pad() / sql_token(",") >> (
            |extra: &mut SQLSpace<'a>| BVec::with_capacity_in(5, extra.bump),
            |extra: &mut SQLSpace<'a>, mut collector: BVec<'a, _>, another: SQLSchema<'a>| { collector.push(another); collector }
        );
        let tuple = sql_token("(") % (tuple / sql_token(")"));
        let tuple = tuple.out(|extra, tuple| SQLSchema::Tuple { tuple }).err(|_, _, _| SQLError::Unknown);
        let i64 = sql_token("i64").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::I64).err(|_, _, _| SQLError::Unknown);
        let i32 = sql_token("i32").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::I32).err(|_, _, _| SQLError::Unknown);
        let i16 = sql_token("i16").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::I16).err(|_, _, _| SQLError::Unknown);
        let i8 = sql_token("i8").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::I8).err(|_, _, _| SQLError::Unknown);
        let u64 = sql_token("u64").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::U64).err(|_, _, _| SQLError::Unknown);
        let u32 = sql_token("u32").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::U32).err(|_, _, _| SQLError::Unknown);
        let u16 = sql_token("u16").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::U16).err(|_, _, _| SQLError::Unknown);
        let u8 = sql_token("u8").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::U8).err(|_, _, _| SQLError::Unknown);
        let nil = sql_token("nil").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::Nil).err(|_, _, _| SQLError::Unknown);
        let f32 = sql_token("f32").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::F32).err(|_, _, _| SQLError::Unknown);
        let f64 = sql_token("f64").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::F64).err(|_, _, _| SQLError::Unknown);
        let str = sql_token("str").out(|extra: &mut SQLSpace<'a>, _| SQLSchema::Str).err(|_, _, _| SQLError::Unknown);
        (tuple ^ i64 ^ i32 ^ i16 ^ i8 ^ u64 ^ u32 ^ u16 ^ u8 ^ nil ^ f32 ^ f64 ^ str).pad()
    })
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;
    use super::*;

    #[test]
    fn parse_tuple() {
        let input = "( i64 , str , )";
        let bump = Bump::new();
        let mut space = SQLSpace::new(&bump, input);
        let parser = sql_parser_schema();
        println!("{:?}", parser.parse(input, 0, &mut space));
    }
}