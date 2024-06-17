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

#[derive(Debug, Clone, Copy)]
pub struct SQLIdent;

impl<'a> Parser<&'a str, SQLError<'a>, SQLSpace<'a>> for SQLIdent {
    fn parse(&self, input: &str, progress: usize, extra: &mut SQLSpace<'a>) -> Result<(usize, &'a str), (usize, SQLError<'a>)> {
        let trimmed = input[progress..].len() - input[progress..].trim_start_matches(|x: char| x.is_ascii_alphanumeric() || x == '_').len();
        if trimmed == 0 {
            Err((progress, SQLError::CannotFindIdent(progress)))
        }
        else {
            log::debug!("IDENT {}", &input[progress..progress+trimmed]);
            Ok((progress+trimmed, extra.bump.alloc_str(&input[progress..progress+trimmed])))
        }
    }
}

pub fn sql_parser_schema<'a>() -> SQLTag<'a, SQLRec<'a>> {
    let tok = |token: &'static str| Token::new(token).pad().err(|_, at, _| SQLError::MismatchToken(at, token));
    let id = || Tag::new(SQLIdent).pad();
    // somehow write these simple options here makes it compiles faster
    let i64 = tok("i64").out(|_, _| SQLSchema::I64);
    let i32 = tok("i32").out(|_, _| SQLSchema::I32);
    let i16 = tok("i16").out(|_, _| SQLSchema::I16);
    let i8 = tok("i8").out(|_, _| SQLSchema::I8);
    let u64 = tok("u64").out(|_, _| SQLSchema::U64);
    let u32 = tok("u32").out(|_, _| SQLSchema::U32);
    let u16 = tok("u16").out(|_, _| SQLSchema::U16);
    let u8 = tok("u8").out(|_, _| SQLSchema::U8);
    let nil = tok("nil").out(|_, _| SQLSchema::Nil);
    let f32 = tok("f32").out(|_, _| SQLSchema::F32);
    let f64 = tok("f64").out(|_, _| SQLSchema::F64);
    let str = tok("str").out(|_, _| SQLSchema::Str);
    let simple = i64 ^ i32 ^ i16 ^ i8 ^ u64 ^ u32 ^ u16 ^ u8 ^ nil ^ f32 ^ f64 ^ str;
    let simple = simple.erase();
    recurse(move |this| {
        // tuple with recursion
        let tuple = (this.clone() / tok(",")).err(|_, _, e| e.unwrap()) >> (
            |extra: &mut SQLSpace<'a>| BVec::with_capacity_in(5, extra.bump),
            |extra: &mut SQLSpace<'a>, mut v: BVec<'a, _>, a: SQLSchema<'a>| { v.push(a); v }
        );
        let tuple =  
            ((tok("(") % tuple.clone()).err(|_, _, e| e.unwrap()) / tok(")")).err(|_, _, e| e.unwrap()) ^
            ((tok("(") % tuple.clone()).err(|_, _, e| e.unwrap()) + (this.clone() / tok(")")).err(|_, _, e| e.unwrap())).out(|extra, (mut v, a)| { v.push(a); v });
        let tuple = tuple.out(|extra, tuple| SQLSchema::Tuple { tuple });
        // named tuple with recursion
        let one = || (id() / tok(":")).err(|_, _, e| e.unwrap()) + this.clone();
        let named = (one() / tok(",")).err(|_, _, e| e.unwrap()) >> (
            |extra: &mut SQLSpace<'a>| (BVec::with_capacity_in(5, extra.bump), BVec::with_capacity_in(5, extra.bump)),
            |extra: &mut SQLSpace<'a>, mut v: (BVec<'a, &'a str>, BVec<'a, SQLSchema<'a>>), a: (&'a str, SQLSchema<'a>)| { v.0.push(a.0); v.1.push(a.1); v }
        );
        let named =  
            ((tok("(") % named.clone()).err(|_, _, e| e.unwrap()) / tok(")")).err(|_, _, e| e.unwrap()) ^
            ((tok("(") % named.clone()).err(|_, _, e| e.unwrap()) + (one()  / tok(")")).err(|_, _, e| e.unwrap())).out(|extra, (mut v, a)| { v.0.push(a.0); v.1.push(a.1); v });
        let named = named.out(|extra, named| SQLSchema::NamedTuple { name: named.0, tuple: named.1 });
        (tuple ^ named ^ simple).pad()
    })
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;
    use super::*;

    #[test]
    fn parse_tuple() {
        let input = "(a: i32, s: (i64, i64))";
        let bump = Bump::new();
        let mut space = SQLSpace::new(&bump, input);
        let parser = sql_parser_schema();
        println!("{:?}", parser.parse(input, 0, &mut space));
    }
}