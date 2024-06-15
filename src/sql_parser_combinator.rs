use std::{cell::OnceCell, marker::PhantomData, ops::{Add, BitOr}, sync::atomic::AtomicU64};

#[allow(unused)]
pub trait Extra<'a, O, E> {
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, O), (usize, E)>) {  }
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, O), (usize, E)>> { None }
}

#[allow(type_alias_bounds)]
pub type PResult<'a, P: Parser> = Result<(usize, P::O<'a>), (usize, P::E<'a>)>;
static COUNT: AtomicU64 = AtomicU64::new(1);

#[auto_impl::auto_impl(Box, &)]
pub trait Parser: Sized {
    type O<'a>: Clone;
    type E<'a>: Clone;
    type X<'a>: Extra<'a, Self::O<'a>, Self::E<'a>> + Clone;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self>;
}

#[derive(Debug, Clone, Copy)]
pub struct Memorized<P: Parser>(P, u64);
impl<P: Parser> Parser for Memorized<P> {
    type O<'a> = P::O<'a>;
    type E<'a> = P::E<'a>;
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        if let Some(result) = extra.clone().replay(progress, self.1) {
            return result;
        }
        let result = self.0.parse(input, progress, extra.clone());
        extra.record(progress, self.1, result.clone());
        return result;
    }
}
impl<P: Parser> Memorized<P> {
    pub fn new(inner: P) -> Self {
        Memorized(inner, COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

#[derive(Debug, Clone)]
pub struct Recursive<P: Parser>(OnceCell<P>);
impl<P: Parser> Parser for Recursive<P> {
    type O<'a> = P::O<'a>;
    type E<'a> = P::E<'a>;
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        self.0.get().unwrap().parse(input, progress, extra)
    }
}
pub fn recursive<P: Parser>(f: impl FnOnce(&Memorized<Recursive<P>>) -> P) -> Memorized<Recursive<P>> {
    let this = Memorized::new(Recursive(OnceCell::new()));
    this.0.0.set(f(&this)).unwrap_or_else(|_| unreachable!());
    return this;
}

#[derive(Debug, Clone, Copy)]
pub enum Either<A, B> { A(A), B(B) }

#[derive(Debug, Clone, Copy)]
pub struct Concat<P, Q>(P, Q)
where P: Parser, 
      Q: for<'a> Parser<X<'a> = P::X<'a>>;
impl<P, Q> Parser for Concat<P, Q>
where P: Parser, 
    Q: for<'a> Parser<X<'a> = P::X<'a>>,
    for<'a> P::X<'a>: Extra<'a, (P::O<'a>, Q::O<'a>), Either<P::E<'a>, Q::E<'a>>>
{
    type E<'a> = Either<P::E<'a>, Q::E<'a>>;
    type O<'a> = (P::O<'a>, Q::O<'a>);
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        let start = progress;
        let (progress, a) = match self.0.parse(input, progress, extra.clone()) {
            Ok(e) => e,
            Err((_, output)) => return Err((start, Either::A(output)))
        };
        let (progress, b) = match self.1.parse(input, progress, extra) {
            Ok(e) => e,
            Err((_, output)) => return Err((start, Either::B(output)))
        };
        Ok((progress, (a, b)))
    }
}
impl<P, Q> Add<Memorized<Q>> for Memorized<P>
where P: Parser, 
      Q: for<'a> Parser<X<'a> = P::X<'a>>,
      for<'a> P::X<'a>: Extra<'a, (P::O<'a>, Q::O<'a>), Either<P::E<'a>, Q::E<'a>>>
{
    type Output = Memorized<Concat<Memorized<P>, Memorized<Q>>>;
    fn add(self, rhs: Memorized<Q>) -> Memorized<Concat<Memorized<P>, Memorized<Q>>> {
        Memorized::new(Concat(self, rhs))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Choice<P, Q>(P, Q)
where P: Parser, 
      Q: for<'a> Parser<X<'a> = P::X<'a>>,
      for<'a> P::X<'a>: Extra<'a, Either<P::O<'a>, Q::O<'a>>, (P::E<'a>, Q::E<'a>)>;
impl<P, Q> Parser for Choice<P, Q> 
where P: Parser, 
      Q: for<'a> Parser<X<'a> = P::X<'a>>,
      for<'a> P::X<'a>: Extra<'a, Either<P::O<'a>, Q::O<'a>>, (P::E<'a>, Q::E<'a>)>
{
    type E<'a> = (P::E<'a>, Q::E<'a>);
    type O<'a> = Either<P::O<'a>, Q::O<'a>>;
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        let (_, a) = match self.0.parse(input, progress, extra.clone()) {
            Err(e) => e,
            Ok((progress, output)) => return Ok((progress, Either::A(output)))
        };
        let (_, b) = match self.1.parse(input, progress, extra) {
            Err(e) => e,
            Ok((progress, output)) => return Ok((progress, Either::B(output)))
        };
        Err((progress, (a, b)))
    }
}
impl<P, Q> BitOr<Memorized<Q>> for Memorized<P>
where P: Parser, 
      Q: for<'a> Parser<X<'a> = P::X<'a>>,
      for<'a> P::X<'a>: Extra<'a, Either<P::O<'a>, Q::O<'a>>, (P::E<'a>, Q::E<'a>)>
{
    type Output = Memorized<Choice<Memorized<P>, Memorized<Q>>>;
    fn bitor(self, rhs: Memorized<Q>) -> Self::Output {
        Memorized::new(Choice(self, rhs))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Lookahead<P>(P);
impl<P: Parser> Parser for Lookahead<P> 
where for<'a> P::X<'a>: Extra<'a, (), P::E<'a>>
{
    type O<'a> = ();
    type E<'a> = P::E<'a>;
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        match self.0.parse(input, progress, extra) {
            Ok(_) => Ok((progress, ())),
            Err(e) => Err(e),
        }
    }
}

pub struct Map<P: Parser, M: for<'a> Mapper<X<'a>=P::X<'a>, I<'a>=P::O<'a>>>(P, PhantomData<M>);
pub trait Mapper {
    type X<'a>;
    type I<'a>;
    type O<'a>: Clone;
    fn map<'a>(_: Self::X<'a>, _: Self::I<'a>) -> Self::O<'a>;
}
impl<P: Parser, M: for<'a> Mapper<X<'a>=P::X<'a>, I<'a>=P::O<'a>>> Parser for Map<P, M> 
    where for<'a> P::X<'a>: Extra<'a, M::O<'a>, P::E<'a>>
{
    type O<'a> = M::O<'a>;
    type E<'a> = P::E<'a>;
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        match self.0.parse(input, progress, extra.clone()) {
            Err(e) => Err(e),
            Ok((progress, output)) => Ok((progress, M::map(extra, output)))
        }
    }
}

pub struct MapErr<P: Parser, M: for<'a> Mapper<X<'a>=P::X<'a>, I<'a>=P::E<'a>>>(P, PhantomData<M>);
impl<P: Parser, M: for<'a> Mapper<X<'a>=P::X<'a>, I<'a>=P::E<'a>>> Parser for MapErr<P, M> 
    where for<'a> P::X<'a>: Extra<'a, P::O<'a>, M::O<'a>>
{
    type O<'a> = P::O<'a>;
    type E<'a> = M::O<'a>;
    type X<'a> = P::X<'a>;
    fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
        match self.0.parse(input, progress, extra.clone()) {
            Ok(e) => Ok(e),
            Err((progress, err)) => Err((progress, M::map(extra, err)))
        }
    }
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;
    use super::*;
    use thiserror::*;

    impl<'a, O, E> Extra<'a, O, E> for &'a Bump {}
    #[derive(Debug, Clone, Copy, Error)]
    pub enum Err {
        #[error("not a number")]
        NotNumber,
        #[error("expect {0}, found {1:?}")]
        Expect(char, Option<char>),
    }

    pub struct Number;
    impl Parser for Number {
        type O<'a> = i64;
        type E<'a> = Err;
        type X<'a> = &'a Bump;
        fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
            let len = input.len() - progress - input[progress..].trim_start_matches(|c: char| char::is_numeric(c)).len();
            if len == 0 { Err((progress, Err::NotNumber))? }
            let num = &input[..len].parse().unwrap();
            return Ok((progress + len, *num))
        }
    }

    pub struct Punct(char);
    impl Parser for Punct {
        type O<'a> = char;
        type E<'a> = Err;
        type X<'a> = &'a Bump;
        fn parse<'a>(&self, input: &str, progress: usize, extra: Self::X<'a>) -> PResult<'a, Self> {
            if input[progress..].starts_with(self.0) {
                Ok((progress + input[progress..].len() - input[progress..].strip_prefix(self.0).unwrap().len(), self.0))
            }
            else {
                Err((progress, Err::Expect(self.0, input[progress..].chars().next())))
            }
        }
    }

    #[test]
    fn arithmetic() {
        let bump = Bump::new();
        let num = || Memorized::new(Number);
        let pun = || Memorized::new(Punct('+'));
        let add = recursive(|rec| {
            let rec = || rec.clone();
            (num() + pun() + rec()) | num()
        });
        println!("{:?}", add.parse("1+1", 0, &bump));
    }

}