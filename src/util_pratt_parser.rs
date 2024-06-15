//! Another precedence parser known as Pratt parsing was first described by Vaughan Pratt 
//! in the 1973 paper "Top down operator precedence",[3] based on recursive descent. 
//! -- Wikipedia

//! TODO: record only on recursive entry points (implement an alternative tagging strategy)

use std::{any::Any, cell::OnceCell, fmt::Debug, marker::PhantomData, ops::{Add, BitAnd, BitOr, BitXor, Index, Mul, Range, Rem, Shr, Div}, os::unix::process, sync::{atomic::AtomicU64, Arc}};

// memorization buffer + output/error allocation buffer
pub trait Extra<O, E> {
    // mark a progress is visited by a parser
    fn mark(&self, progress: usize, tag: u64) { }
    // record execution result
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, &O), (usize, &E)>)   {  }
    // replay an expression
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, &O), (usize, &E)>> { None }
    // allocate output
    fn out(&self, o: O) -> &O;
    // allocate error
    fn err(&self, e: E) -> &E;
}

// a general parser trait
pub trait Parser<'p, O, E, X>: Sized 
    where O: 'p, E: 'p, X: Extra<O, E> + 'p,
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p O), (usize, &'p E)>;
    fn tag(&self) -> u64 { 0 }
    fn tagged(self) -> Tag<'p, O, E, X, Self> { Tag::new(self) }
}

#[derive(Debug)]
pub struct Tag<'p, O, E, X, P>
    where O: 'p, E: 'p, X: Extra<O, E> + 'p, P: Parser<'p, O, E, X>,
{
    inner: P, tag: u64, 
    phantom: PhantomData<(fn(&'p())->(), O, E, X)>
}
impl<'p, O, E, X, P> Clone for Tag<'p, O, E, X, P>
    where O: 'p, E: 'p, X: Extra<O, E> + 'p, P: Parser<'p, O, E, X> + Clone,
{
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), tag: self.tag, phantom: PhantomData }
    }
}
impl<'p, O, E, X, P> Parser<'p, O, E, X> for Tag<'p, O, E, X, P>
    where O: 'p, E: 'p, X: Extra<O, E> + 'p, P: Parser<'p, O, E, X>,
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p O), (usize, &'p E)> {
        if self.tag() == 0 {
            return self.inner.parse(input, progress, extra);
        }
        if let Some(result) = extra.replay(progress, self.tag) {
            return result;
        }
        extra.mark(progress, self.tag);
        let result = self.inner.parse(input, progress, extra);
        extra.record(progress, self.tag, result.clone());
        return result;
    }
    fn tag(&self) -> u64 { self.tag }
}
impl<'p, O, E, X, P> Tag<'p, O, E, X, P>
where
    P: Parser<'p, O, E, X>, 
    O: 'p, E: 'p, X: 'p, 
    X: Extra<O, E>
{
    pub fn new(inner: P) -> Self {
        static COUNT: AtomicU64 = AtomicU64::new(1);
        use std::sync::atomic::Ordering::SeqCst;
        let tag = if inner.tag() != 0 { 0 } else {
            COUNT.fetch_add(1, SeqCst)
        };
        Tag{inner, tag, phantom: PhantomData}
    }
    pub fn out<Z, FUNC>(self, map: FUNC) -> Tag<'p, Z, E, X, MapOut<'p, O, E, X, P, Z, FUNC>>
        where X: Extra<Z, E>,
              Z: 'p,
              FUNC: Fn(&'p X, &'p O) -> &'p Z,
    {
        Tag{
            inner: MapOut{map, inner: self.inner, phantom: PhantomData}, 
            tag:self.tag, phantom: PhantomData
        }
    }
    pub fn err<Z, FUNC>(self, map: FUNC) -> Tag<'p, O, Z, X, MapErr<'p, O, E, X, P, Z, FUNC>>
        where X: Extra<O, Z>,
              Z: 'p,
              FUNC: Fn(&'p X, &'p E) -> &'p Z,
    {
        Tag{
            inner: MapErr{map, inner: self.inner, phantom: PhantomData}, 
            tag:self.tag, phantom: PhantomData
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {L(L), R(R)}

#[derive(Debug, Clone, Copy)]
pub struct Then<'p, OP, EP, OQ, EQ, X, P, Q>
    where OP: 'p, EP: 'p,
          OQ: 'p, EQ: 'p, 
          X: Extra<(&'p OP, &'p OQ), Either<&'p EP, &'p EQ>> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
          P: Parser<'p, OP, EP, X>, 
          Q: Parser<'p, OQ, EQ, X>
{
    lhs: P,
    rhs: Q,
    phant: PhantomData<(fn(&'p()), OP, EP, OQ, EQ, X, P, Q)>
}
impl<'p, OP, EP, OQ, EQ, X, P, Q> Parser<'p, (&'p OP, &'p OQ), Either<&'p EP, &'p EQ>, X>  for Then<'p, OP, EP, OQ, EQ, X, P, Q>
    where OP: 'p, EP: 'p,
          OQ: 'p, EQ: 'p, 
          X: Extra<(&'p OP, &'p OQ), Either<&'p EP, &'p EQ>> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
          P: Parser<'p, OP, EP, X>, 
          Q: Parser<'p, OQ, EQ, X>
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p (&'p OP, &'p OQ)), (usize, &'p Either<&'p EP, &'p EQ>)> {
        let start = progress;
        let (progress, lhs) = match Parser::<'p>::parse(&self.lhs, input, progress, extra) {
            Ok((progress, lhs)) => (progress, lhs),
            Err((progress, err)) => return Err((start, extra.err(Either::L(err))))
        };
        let (progress, rhs) = match Parser::<'p>::parse(&self.rhs, input, progress, extra) {
            Ok((progress, rhs)) => (progress, rhs),
            Err((progress, err)) => return Err((start, extra.err(Either::R(err))))
        };
        Ok((progress, extra.out((lhs, rhs))))
    }
}
impl<'p, OP, EP, OQ, EQ, X, P, Q> Add<Tag<'p, OQ, EQ, X, Q>> for Tag<'p, OP, EP, X, P>
    where OP: 'p, EP: 'p,
          OQ: 'p, EQ: 'p, 
          X: Extra<(&'p OP, &'p OQ), Either<&'p EP, &'p EQ>> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
          P: Parser<'p, OP, EP, X>, 
          Q: Parser<'p, OQ, EQ, X>
{
    type Output = Tag<'p, (&'p OP, &'p OQ), Either<&'p EP, &'p EQ>, X, Then<'p, OP, EP, OQ, EQ, X, Tag<'p, OP, EP, X, P>, Tag<'p, OQ, EQ, X, Q>>>;
    fn add(self, rhs: Tag<'p, OQ, EQ, X, Q>) -> Self::Output {
        Tag::new(Then { lhs: self, rhs, phant: PhantomData })
    }
}
impl<'p, OP, EP, OQ, EQ, X, P, Q> Rem<Tag<'p, OQ, EQ, X, Q>> for Tag<'p, OP, EP, X, P> 
where OP: 'p, EP: 'p,
      OQ: 'p, EQ: 'p, 
      X: Extra<(&'p OP, &'p OQ), Either<&'p EP, &'p EQ>> + Extra<OQ, Either<&'p EP, &'p EQ>> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
      P: Parser<'p, OP, EP, X>, 
      Q: Parser<'p, OQ, EQ, X>
{
    type Output = Tag<'p, OQ, Either<&'p EP, &'p EQ>, X, MapOut<'p, (&'p OP, &'p OQ), Either<&'p EP, &'p EQ>, X, Then<'p, OP, EP, OQ, EQ, X, Tag<'p, OP, EP, X, P>, Tag<'p, OQ, EQ, X, Q>>, OQ, fn(&'p X, &'p (&'p OP, &'p OQ)) -> &'p OQ>>;
    fn rem(self, rhs: Tag<'p, OQ, EQ, X, Q>) -> Self::Output {
        fn unwrap<'p, X, OP, OQ>(extra: &'p X, (p, q): &'p (&'p OP, &'p OQ)) -> &'p OQ { q }
        (self + rhs).out(unwrap)
    }
}
impl<'p, OP, EP, OQ, EQ, X, P, Q> Div<Tag<'p, OQ, EQ, X, Q>> for Tag<'p, OP, EP, X, P> 
where OP: 'p, EP: 'p,
      OQ: 'p, EQ: 'p, 
      X: Extra<(&'p OP, &'p OQ), Either<&'p EP, &'p EQ>> + Extra<OP, Either<&'p EP, &'p EQ>> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
      P: Parser<'p, OP, EP, X>, 
      Q: Parser<'p, OQ, EQ, X>
{
    type Output = Tag<'p, OP, Either<&'p EP, &'p EQ>, X, MapOut<'p, (&'p OP, &'p OQ), Either<&'p EP, &'p EQ>, X, Then<'p, OP, EP, OQ, EQ, X, Tag<'p, OP, EP, X, P>, Tag<'p, OQ, EQ, X, Q>>, OP, fn(&'p X, &'p (&'p OP, &'p OQ)) -> &'p OP>>;
    fn div(self, rhs: Tag<'p, OQ, EQ, X, Q>) -> Self::Output {
        fn unwrap<'p, X, OP, OQ>(extra: &'p X, (p, q): &'p (&'p OP, &'p OQ)) -> &'p OP { p }
        (self + rhs).out(unwrap)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Else<'p, OP, EP, OQ, EQ, X, P, Q>
    where OP: 'p, EP: 'p,
          OQ: 'p, EQ: 'p, 
          X: Extra<Either<&'p OP, &'p OQ>, (&'p EP, &'p EQ)> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
          P: Parser<'p, OP, EP, X>, 
          Q: Parser<'p, OQ, EQ, X>
{
    lhs: P,
    rhs: Q,
    phant: PhantomData<(fn(&'p()), OP, EP, OQ, EQ, X, P, Q)>
}
impl<'p, OP, EP, OQ, EQ, X, P, Q> Parser<'p, Either<&'p OP, &'p OQ>, (&'p EP, &'p EQ), X> for Else<'p, OP, EP, OQ, EQ, X, P, Q>
    where OP: 'p, EP: 'p,
          OQ: 'p, EQ: 'p, 
          X: Extra<Either<&'p OP, &'p OQ>, (&'p EP, &'p EQ)> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
          P: Parser<'p, OP, EP, X>, 
          Q: Parser<'p, OQ, EQ, X>
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p Either<&'p OP, &'p OQ>), (usize, &'p (&'p EP, &'p EQ))> {
        let start = progress;
        let (progress, lhs) = match Parser::<'p>::parse(&self.lhs, input, progress, extra) {
            Err((progress, lhs)) => (start, lhs),
            Ok((progress, err)) => return Ok((progress, extra.out(Either::L(err))))
        };
        let (progress, rhs) = match Parser::<'p>::parse(&self.rhs, input, progress, extra) {
            Err((progress, rhs)) => (start, rhs),
            Ok((progress, err)) => return Ok((progress, extra.out(Either::R(err))))
        };
        Err((start, extra.err((lhs, rhs))))
    }
}
impl<'p, OP, EP, OQ, EQ, X, P, Q> BitOr<Tag<'p, OQ, EQ, X, Q>> for Tag<'p, OP, EP, X, P>
    where OP: 'p, EP: 'p,
          OQ: 'p, EQ: 'p, 
          X: Extra<Either<&'p OP, &'p OQ>, (&'p EP, &'p EQ)> + Extra<OP, EP> + Extra<OQ, EQ> + 'p, 
          P: Parser<'p, OP, EP, X>, 
          Q: Parser<'p, OQ, EQ, X>
{
    type Output = Tag<'p, Either<&'p OP, &'p OQ>, (&'p EP, &'p EQ), X, Else<'p, OP, EP, OQ, EQ, X, Tag<'p, OP, EP, X, P>, Tag<'p, OQ, EQ, X, Q>>>;
    fn bitor(self, rhs: Tag<'p, OQ, EQ, X, Q>) -> Self::Output {
        Tag::new(Else { lhs: self, rhs, phant: PhantomData })
    }
}
impl<'p, O, EP, EQ, X, P, Q> BitXor<Tag<'p, O, EQ, X, Q>> for Tag<'p, O, EP, X, P>
where 
    O: 'p, EP: 'p, EQ: 'p, 
    X: Extra<Either<&'p O, &'p O>, (&'p EP, &'p EQ)> + Extra<O, (&'p EP, &'p EQ)> + Extra<O, EP> + Extra<O, EQ> + 'p, 
    P: Parser<'p, O, EP, X>, 
    Q: Parser<'p, O, EQ, X>
{
    type Output = Tag<'p, O, (&'p EP, &'p EQ), X, MapOut<'p, Either<&'p O, &'p O>, (&'p EP, &'p EQ), X, Else<'p, O, EP, O, EQ, X, Tag<'p, O, EP, X, P>, Tag<'p, O, EQ, X, Q>>, O, fn(&'p  X, &'p  Either<&'p  O, &'p  O>) -> &'p O>>;
    fn bitxor(self, rhs: Tag<'p, O, EQ, X, Q>) -> Self::Output {
        fn map<'p, A, Z>(a: &'p A, b: &'p Either<&'p Z, &'p Z>) -> &'p Z {
            match b { Either::L(x) => x, Either::R(x) => x }
        }
        (self | rhs).out(map)
    }
}

struct Helper<'p, O: 'p, E: 'p, X> {
    // pointer to external parser object
    that: *const u8, 
    // Safety: since parse don't mutate self, this Helper can be viewed as a Fn(...) -> ... object
    parse: fn(*const u8, &str, usize, &'p X) -> Result<(usize, &'p O), (usize, &'p E)>,
    // destruct self
    destruct: fn(*const u8)
}
impl<'p, O, E, X> Drop for Helper<'p, O, E, X> {
    fn drop(&mut self) {
        (self.destruct)(self.that);
    }
}

pub struct Recursive<'p, O: 'p, E: 'p, X>(Arc<OnceCell<Helper<'p, O, E, X>>>);
impl<'p, O: 'p, E: 'p, X> Parser<'p, O, E, X> for Recursive<'p, O, E, X> 
    where X: Extra<O, E>
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p O), (usize, &'p E)> {
        let this = self.clone();
        let Helper{that, parse, ..} = this.0.as_ref().get().unwrap();
        parse(*that, input, progress, extra)
    }
}
pub fn recurse<'p, O: 'p, E: 'p, X, P>(builder: impl FnOnce(Tag<'p, O, E, X, Recursive<'p, O, E, X>>) -> P) -> Tag<'p, O, E, X, Recursive<'p, O, E, X>>
    where X: Extra<O, E>,
          P: Parser<'p, O, E, X>,
{
    let this: Tag<O, E, X, Recursive<O, E, X>> = Tag::new(Recursive(Arc::new(OnceCell::new())));
    let that = Box::leak(Box::new(Tag::new(builder(this.clone())))) as *const _ as *const u8;
    // make sure this "leak" operation is legit
    assert!(std::mem::size_of::<&mut Tag<O, E, X, P>>() == std::mem::size_of::<*const u8>());
    // cast parse and destruct function
    let parse = unsafe {std::mem::transmute(Tag::<O, E, X, P>::parse as fn(_, _, _, _) -> _)};
    let destruct = unsafe {std::mem::transmute::<_, fn(*const u8)>(drop::<Box<Tag::<O, E, X, P>>> as fn(_) -> _)};
    // set self to be inner function
    this.inner.0.as_ref().set(Helper{that, parse, destruct});
    return this;
}
impl<'a, O, E, X> std::fmt::Debug for Recursive<'a, O, E, X> 
    where X: Extra<O, E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Recursive(inner: {:?})", &self.0 as *const _)
    }
}
impl<'a, O, E, X> Clone for Recursive<'a, O, E, X> {
    fn clone(&self) -> Self {
        Recursive(Arc::clone(&self.0))
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Repeat<'p, O, E, X, P, Z, INIT, FOLD>
    where P: Parser<'p, O, E, X>, 
          O: 'p, E: 'p, X: 'p, Z: 'p,
          FOLD: Fn(&'p X, &'p mut Z, &'p O) -> &'p mut Z,
          INIT: Fn(&'p X) -> &'p mut Z,
          X: Extra<O, E> + Extra<Z, E>,
{
    inner: P,
    fold: FOLD,
    init: INIT,
    phantom: PhantomData<(fn(&'p()), O, E, X, Z)>
}
impl<'p, O, E, X, P, Z, INIT, FOLD> Parser<'p, Z, E, X> for Repeat<'p, O, E, X, P, Z, INIT, FOLD>
where P: Parser<'p, O, E, X>, 
      O: 'p, E: 'p, X: 'p, Z: 'p,
      FOLD: Fn(&'p X, &'p mut Z, &'p O) -> &'p mut Z,
      INIT: Fn(&'p X) -> &'p mut Z,
      X: Extra<O, E> + Extra<Z, E>,
{
    fn parse(&self, input: &str, mut progress: usize, extra: &'p X) -> Result<(usize, &'p Z), (usize, &'p E)> {
        let mut ini = (self.init)(extra);
        loop {
            let (progress_new, out) = match self.inner.parse(input, progress, extra) {
                Ok((progress, out)) => (progress, (self.fold)(extra, ini, out)),
                Err(e) => return Ok((progress, ini)),
            };
            progress = progress_new;
            ini = out;
        }
    }
}
impl<'p, O, E, X, P, Z, INIT, FOLD> Shr<(INIT, FOLD)> for Tag<'p, O, E, X, P>
where P: Parser<'p, O, E, X>, 
      O: 'p, E: 'p, X: 'p, Z: 'p,
      FOLD: Fn(&'p X, &'p mut Z, &'p O) -> &'p mut Z,
      INIT: Fn(&'p X) -> &'p mut Z,
      X: Extra<O, E> + Extra<Z, E>,
{
    type Output = Tag<'p, Z, E, X, Repeat<'p, O, E, X, Tag<'p, O, E, X, P>, Z, INIT, FOLD>>;
    fn shr(self, (init, fold): (INIT, FOLD)) -> Self::Output {
        Tag::new(Repeat{
            inner: self,
            fold, init,
            phantom: PhantomData
        })
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapOut<'p, O, E, X, P, Z, FUNC>
where 
    P: Parser<'p, O, E, X>, 
    O: 'p, E: 'p, X: 'p, Z: 'p,
    FUNC: Fn(&'p X, &'p O) -> &'p Z,
    X: Extra<O, E> + Extra<Z, E>
{
    map: FUNC,
    inner: P,
    phantom: PhantomData<(fn(&'p()), O, E, X, Z)>
}
impl<'p, O, E, X, P, Z, FUNC> Parser<'p, Z, E, X> for MapOut<'p, O, E, X, P, Z, FUNC>
where 
    P: Parser<'p, O, E, X>, 
    O: 'p, E: 'p, X: 'p, Z: 'p,
    FUNC: Fn(&'p X, &'p O) -> &'p Z,
    X: Extra<O, E> + Extra<Z, E>
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p Z), (usize, &'p E)> {
        match self.inner.parse(input, progress, extra) {
            Ok((progress, out)) => Ok((progress, (self.map)(extra, out))),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapErr<'p, O, E, X, P, Z, FUNC>
where 
    P: Parser<'p, O, E, X>, 
    O: 'p, E: 'p, X: 'p, Z: 'p,
    FUNC: Fn(&'p X, &'p E) -> &'p Z,
    X: Extra<O, E> + Extra<O, Z>
{
    map: FUNC,
    inner: P,
    phantom: PhantomData<(fn(&'p()), O, E, X, Z)>
}
impl<'p, O, E, X, P, Z, FUNC> Parser<'p, O, Z, X> for MapErr<'p, O, E, X, P, Z, FUNC>
where 
    P: Parser<'p, O, E, X>, 
    O: 'p, E: 'p, X: 'p, Z: 'p,
    FUNC: Fn(&'p X, &'p E) -> &'p Z,
    X: Extra<O, E> + Extra<O, Z>
{
    fn parse(&self, input: &str, progress: usize, extra: &'p X) -> Result<(usize, &'p O), (usize, &'p Z)> {
        match self.inner.parse(input, progress, extra) {
            Err((progress, err)) => Err((progress, (self.map)(extra, err))),
            Ok(o) => Ok(o),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Token<X: Extra<(), ()>> {
    token: &'static str, 
    alloc: PhantomData<X>,
}
impl<'a, X> Token<X>
    where X: Extra<(), ()> + 'a
{
    pub fn new(token: &'static str) -> Tag<'a, (), (), X, Self> {
        Tag::new(Token { token, alloc: PhantomData })
    }
}
impl<'a, X> Parser<'a, (), (), X> for Token<X>
    where X: Extra<(), ()> + 'a
{
    fn parse(&self, input: &str, progress: usize, extra: &'a X) -> Result<(usize, &'a ()), (usize, &'a ())> {
        if input[progress..].starts_with(self.token) {
            log::debug!("TOKEN={:?} MATCHED", self.token);
            Ok((progress + self.token.len(), extra.out(())))
        }
        else {
            log::debug!("TOKEN={:?} SEEN={:?}", self.token, &input[progress..(progress+self.token.len()).min(input.len())]);
            Err((progress, extra.err(())))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pad<X>(PhantomData<X>);
impl<'a, X> Pad<X>
    where X: Extra<(), ()> + 'a
{
    pub fn new() -> Tag<'a, (), (), X, Self> {
        Tag::new(Pad(PhantomData))
    }
}
impl<'a, X> Parser<'a, (), (), X> for Pad<X>
    where X: Extra<(), ()> + 'a
{
    fn parse(&self, input: &str, progress: usize, extra: &'a X) -> Result<(usize, &'a ()), (usize, &'a ())> {
        let mut cut = 0;
        let mut last = true;
        for (i, c) in input[progress..].char_indices() {
            cut = i;
            if c.is_whitespace() { continue }
            last = false;
            break;
        }
        if last { cut = input[progress..].len() }
        log::debug!("EAT SPACE={cut}");
        Ok((progress + cut, extra.out(())))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bumpalo::Bump;

    impl<O, E> Extra<O, E> for Bump {
        fn err(&self, e: E) -> &E {
            self.alloc(e)
        }
        fn out(&self, o: O) -> &O {
            self.alloc(o)   
        }
    }

    #[test]
    fn alphabet() {
        let bump = Bump::new();
        let a = || Token::new("a");
        let b = || Token::new("b");
        fn take_left<A, B, C>(_: C, (a, b): (A, B)) -> A { a }
        fn unwrap<A, B, C>(_: C, a: Either<A, B>) -> () { () }
        let parser = recurse::<i64, (), Bump, _>(|this| {
            (a() + this.clone()).out(|extra, (lhs, rhs)| {
                extra.alloc(**rhs + 20)
            }) 
            ^ (b().out(|extra: &Bump, _| extra.alloc(1)))
        }.err(|extra, _| extra.alloc(())));
        let example = "aaabb";
        let this = parser.parse(&example, 0, &bump);
        assert!(&61 == this.unwrap().1);
    }
}