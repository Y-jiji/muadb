//! Another precedence parser known as Pratt parsing was first described by Vaughan Pratt 
//! in the 1973 paper "Top down operator precedence",[3] based on recursive descent. 
//! -- Wikipedia

//! TODO: record only on recursive entry points (implement an alternative tagging strategy)

use std::{any::Any, cell::OnceCell, fmt::Debug, marker::PhantomData, ops::{Add, BitAnd, BitOr, BitXor, Index, Mul, Range, Rem, Shr, Div}, os::unix::process, sync::{atomic::AtomicU64, Arc}};

// memorization buffer + output/error allocation buffer
pub trait Extra<O, E>: Clone 
    where
        O: Clone,
        E: Clone
{
    // mark a progress is visited by a parser
    fn mark(&self, progress: usize, tag: u64) { }
    // record execution result
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, O), (usize, E)>)   {  }
    // replay an expression
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, O), (usize, E)>> { None }
}

// a general parser trait
pub trait Parser<O, E, X>: Sized 
    where
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, O), (usize, E)>;
    fn tag(&self) -> u64 { 0 }
}

#[derive(Debug)]
pub struct Tag<O, E, X, P>
    where
        P: Parser<O, E, X>,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    inner: P, tag: u64, 
    phantom: PhantomData<(O, E, X)>
}
impl<O, E, X, P> Clone for Tag<O, E, X, P>
    where
        P: Parser<O, E, X> + Clone,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), tag: self.tag, phantom: PhantomData }
    }
}
impl<O, E, X, P> Parser<O, E, X> for Tag<O, E, X, P>
    where
        P: Parser<O, E, X>,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, O), (usize, E)> {
        if self.tag() == 0 {
            return self.inner.parse(input, progress, extra);
        }
        if let Some(result) = extra.replay(progress, self.tag) {
            return result.clone();
        }
        extra.mark(progress, self.tag);
        let result = self.inner.parse(input, progress, extra.clone());
        extra.record(progress, self.tag, result.clone());
        return result;
    }
    fn tag(&self) -> u64 { self.tag }
}
impl<O, E, X, P> Tag<O, E, X, P>
    where
        P: Parser<O, E, X>,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    pub fn new(inner: P) -> Self {
        static COUNT: AtomicU64 = AtomicU64::new(1);
        use std::sync::atomic::Ordering::SeqCst;
        let tag = if inner.tag() != 0 { 0 } else {
            COUNT.fetch_add(1, SeqCst)
        };
        Tag{inner, tag, phantom: PhantomData}
    }
    pub fn out<Z, FUNC>(self, map: FUNC) -> Tag<Z, E, X, MapOut<O, E, X, P, Z, FUNC>>
        where X: Extra<Z, E>,
              Z: Clone,
              FUNC: Fn(X, O) -> Z,
    {
        Tag{
            inner: MapOut{map, inner: self.inner, phantom: PhantomData}, 
            tag:self.tag, phantom: PhantomData
        }
    }
    pub fn err<Z, FUNC>(self, map: FUNC) -> Tag<O, Z, X, MapErr<O, E, X, P, Z, FUNC>>
        where X: Extra<O, Z>,
              Z: Clone,
              FUNC: Fn(X, E) -> Z,
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
pub struct Then<OP, EP, OQ, EQ, X, P, Q>
where 
    X: Extra<(OP, OQ), Either<EP, EQ>> + Extra<OP, EP> + Extra<OQ, EQ>, 
    P: Parser<OP, EP, X>, 
    Q: Parser<OQ, EQ, X>,
    OP: Clone, OQ: Clone,
    EP: Clone, EQ: Clone
{
    lhs: P,
    rhs: Q,
    phant: PhantomData<(OP, EP, OQ, EQ, X, P, Q)>
}
impl<OP, EP, OQ, EQ, X, P, Q> Parser<(OP, OQ), Either<EP, EQ>, X>  for Then<OP, EP, OQ, EQ, X, P, Q>
    where 
        X: Extra<(OP, OQ), Either<EP, EQ>>,
        X: Extra<OP, EP> + Extra<OQ, EQ>, 
        P: Parser<OP, EP, X>, 
        Q: Parser<OQ, EQ, X>,
        OP: Clone, OQ: Clone,
        EP: Clone, EQ: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, (OP, OQ)), (usize, Either<EP, EQ>)> {
        let start = progress;
        let (progress, lhs) = match Parser::parse(&self.lhs, input, progress, extra.clone()) {
            Ok((progress, lhs)) => (progress, lhs),
            Err((progress, err)) => return Err((start, Either::L(err)))
        };
        let (progress, rhs) = match Parser::parse(&self.rhs, input, progress, extra.clone()) {
            Ok((progress, rhs)) => (progress, rhs),
            Err((progress, err)) => return Err((start, Either::R(err)))
        };
        Ok((progress, (lhs, rhs)))
    }
}
impl<OP, EP, OQ, EQ, X, P, Q> Add<Tag<OQ, EQ, X, Q>> for Tag<OP, EP, X, P>
    where 
          X: Extra<(OP, OQ), Either<EP, EQ>>,
          X: Extra<OP, EP> + Extra<OQ, EQ>, 
          P: Parser<OP, EP, X>, 
          Q: Parser<OQ, EQ, X>,
          OP: Clone, OQ: Clone,
          EP: Clone, EQ: Clone
{
    type Output = Tag<(OP, OQ), Either<EP, EQ>, X, Then<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>>;
    fn add(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        Tag::new(Then { lhs: self, rhs, phant: PhantomData })
    }
}
impl<OP, EP, OQ, EQ, X, P, Q> Rem<Tag<OQ, EQ, X, Q>> for Tag<OP, EP, X, P> 
where 
      X: Extra<(OP, OQ), Either<EP, EQ>> + Extra<OQ, Either<EP, EQ>> + Extra<OP, EP> + Extra<OQ, EQ>, 
      P: Parser<OP, EP, X>, 
      Q: Parser<OQ, EQ, X>,
      OP: Clone, OQ: Clone,
      EP: Clone, EQ: Clone
{
    type Output = Tag<OQ, Either<EP, EQ>, X, MapOut<(OP, OQ), Either<EP, EQ>, X, Then<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>, OQ, fn(X, (OP, OQ)) -> OQ>>;
    fn rem(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        fn unwrap<X, OP, OQ>(extra: X, (p, q): (OP, OQ)) -> OQ { q }
        (self + rhs).out(unwrap)
    }
}
impl<OP, EP, OQ, EQ, X, P, Q> Div<Tag<OQ, EQ, X, Q>> for Tag<OP, EP, X, P> 
where 
      X: Extra<(OP, OQ), Either<EP, EQ>> + Extra<OP, Either<EP, EQ>> + Extra<OP, EP> + Extra<OQ, EQ>, 
      P: Parser<OP, EP, X>, 
      Q: Parser<OQ, EQ, X>,
      OP: Clone, OQ: Clone,
      EP: Clone, EQ: Clone
{
    type Output = Tag<OP, Either<EP, EQ>, X, MapOut<(OP, OQ), Either<EP, EQ>, X, Then<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>, OP, fn(X, (OP, OQ)) -> OP>>;
    fn div(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        fn unwrap<X, OP, OQ>(extra: X, (p, q): (OP, OQ)) -> OP { p }
        (self + rhs).out(unwrap)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Else<OP, EP, OQ, EQ, X, P, Q>
    where 
          X: Extra<Either<OP, OQ>, (EP, EQ)> + Extra<OP, EP> + Extra<OQ, EQ>, 
          P: Parser<OP, EP, X>, 
          Q: Parser<OQ, EQ, X>,
          OP: Clone, OQ: Clone,
          EP: Clone, EQ: Clone
{
    lhs: P,
    rhs: Q,
    phant: PhantomData<(OP, EP, OQ, EQ, X, P, Q)>
}
impl<OP, EP, OQ, EQ, X, P, Q> Parser<Either<OP, OQ>, (EP, EQ), X> for Else<OP, EP, OQ, EQ, X, P, Q>
    where 
          X: Extra<Either<OP, OQ>, (EP, EQ)> + Extra<OP, EP> + Extra<OQ, EQ>, 
          P: Parser<OP, EP, X>, 
          Q: Parser<OQ, EQ, X>,
          OP: Clone, OQ: Clone,
          EP: Clone, EQ: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, Either<OP, OQ>), (usize, (EP, EQ))> {
        let start = progress;
        let (progress, lhs) = match Parser::parse(&self.lhs, input, progress, extra.clone()) {
            Err((progress, lhs)) => (start, lhs),
            Ok((progress, err)) => return Ok((progress, Either::L(err)))
        };
        let (progress, rhs) = match Parser::parse(&self.rhs, input, progress, extra.clone()) {
            Err((progress, rhs)) => (start, rhs),
            Ok((progress, err)) => return Ok((progress, Either::R(err)))
        };
        Err((start, (lhs, rhs)))
    }
}
impl<OP, EP, OQ, EQ, X, P, Q> BitOr<Tag<OQ, EQ, X, Q>> for Tag<OP, EP, X, P>
    where 
          X: Extra<Either<OP, OQ>, (EP, EQ)> + Extra<OP, EP> + Extra<OQ, EQ>, 
          P: Parser<OP, EP, X>, 
          Q: Parser<OQ, EQ, X>,
          OP: Clone, OQ: Clone,
          EP: Clone, EQ: Clone
{
    type Output = Tag<Either<OP, OQ>, (EP, EQ), X, Else<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>>;
    fn bitor(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        Tag::new(Else { lhs: self, rhs, phant: PhantomData })
    }
}
impl<O, EP, EQ, X, P, Q> BitXor<Tag<O, EQ, X, Q>> for Tag<O, EP, X, P>
where 
    X: Extra<Either<O, O>, (EP, EQ)> + Extra<O, (EP, EQ)> + Extra<O, EP> + Extra<O, EQ>, 
    P: Parser<O, EP, X>, 
    Q: Parser<O, EQ, X>,
    O: Clone,
    EP: Clone, EQ: Clone
{
    type Output = Tag<O, (EP, EQ), X, MapOut<Either<O, O>, (EP, EQ), X, Else<O, EP, O, EQ, X, Tag<O, EP, X, P>, Tag<O, EQ, X, Q>>, O, fn( X,  Either< O,  O>) -> O>>;
    fn bitxor(self, rhs: Tag<O, EQ, X, Q>) -> Self::Output {
        fn map<A, Z>(a: A, b: Either<Z, Z>) -> Z {
            match b { Either::L(x) => x, Either::R(x) => x }
        }
        (self | rhs).out(map)
    }
}

struct Helper<O, E, X> {
    // pointer to external parser object
    that: *const u8, 
    // Safety: since parse don't mutate self, this Helper can be viewed as a Fn(...) -> ... object
    parse: fn(*const u8, &str, usize, X) -> Result<(usize, O), (usize, E)>,
    // destruct self
    destruct: fn(*const u8)
}
impl<O, E, X> Drop for Helper<O, E, X> {
    fn drop(&mut self) {
        (self.destruct)(self.that);
    }
}

pub struct Recursive<O, E, X>(Arc<OnceCell<Helper<O, E, X>>>);
impl<O, E, X> Parser<O, E, X> for Recursive<O, E, X> 
    where
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, O), (usize, E)> {
        let this = self.clone();
        let Helper{that, parse, ..} = this.0.as_ref().get().unwrap();
        parse(*that, input, progress, extra)
    }
}
pub fn recurse<O, E, X, P>(builder: impl FnOnce(Tag<O, E, X, Recursive<O, E, X>>) -> P) -> Tag<O, E, X, Recursive<O, E, X>>
where
    X: Extra<O, E> + Clone, 
    O: Clone, 
    E: Clone,
    P: Parser<O, E, X>
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
impl<O, E, X> std::fmt::Debug for Recursive<O, E, X> 
    where X: Extra<O, E>,
          O: Clone,
          E: Clone
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Recursive(inner: {:?})", &self.0 as *const _)
    }
}
impl<O, E, X> Clone for Recursive<O, E, X> {
    fn clone(&self) -> Self {
        Recursive(Arc::clone(&self.0))
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Repeat<O, E, X, P, Z, INIT, FOLD>
    where P: Parser<O, E, X>, 
          O: Clone, 
          E: Clone,
          Z: Clone,
          FOLD: Fn(X, Z, O) -> Z,
          INIT: Fn(X) -> Z,
          X: Extra<O, E> + Extra<Z, E>,
{
    inner: P,
    fold: FOLD,
    init: INIT,
    phantom: PhantomData<(O, E, X, Z)>
}
impl<O, E, X, P, Z, INIT, FOLD> Parser<Z, E, X> for Repeat<O, E, X, P, Z, INIT, FOLD>
    where
        P: Parser<O, E, X>, 
        O: Clone, 
        E: Clone,
        Z: Clone,
        FOLD: Fn(X, Z, O) -> Z,
        INIT: Fn(X) -> Z,
        X: Extra<O, E> + Extra<Z, E> + Clone,
{
    fn parse(&self, input: &str, mut progress: usize, extra: X) -> Result<(usize, Z), (usize, E)> {
        let mut ini = (self.init)(extra.clone());
        loop {
            let (progress_new, out) = match self.inner.parse(input, progress, extra.clone()) {
                Ok((progress, out)) => (progress, (self.fold)(extra.clone(), ini, out)),
                Err(e) => return Ok((progress, ini)),
            };
            progress = progress_new;
            ini = out;
        }
    }
}
impl<O, E, X, P, Z, INIT, FOLD> Shr<(INIT, FOLD)> for Tag<O, E, X, P>
    where
        P: Parser<O, E, X>, 
        O: Clone, 
        E: Clone,
        Z: Clone,
        FOLD: Fn(X, Z, O) -> Z,
        INIT: Fn(X) -> Z,
        X: Extra<O, E> + Extra<Z, E> + Clone,
{
    type Output = Tag<Z, E, X, Repeat<O, E, X, Tag<O, E, X, P>, Z, INIT, FOLD>>;
    fn shr(self, (init, fold): (INIT, FOLD)) -> Self::Output {
        Tag::new(Repeat{
            inner: self,
            fold, init,
            phantom: PhantomData
        })
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapOut<O, E, X, P, Z, FUNC>
where 
    P: Parser<O, E, X>, 
    O: Clone,
    E: Clone,
    Z: Clone,
    FUNC: Fn(X, O) -> Z,
    X: Extra<O, E> + Extra<Z, E>
{
    map: FUNC,
    inner: P,
    phantom: PhantomData<(O, E, X, Z)>
}
impl<O, E, X, P, Z, FUNC> Parser<Z, E, X> for MapOut<O, E, X, P, Z, FUNC>
where 
    P: Parser<O, E, X>, 
    O: Clone,
    E: Clone,
    Z: Clone,
    FUNC: Fn(X, O) -> Z,
    X: Extra<O, E> + Extra<Z, E>
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, Z), (usize, E)> {
        match self.inner.parse(input, progress, extra.clone()) {
            Ok((progress, out)) => Ok((progress, (self.map)(extra, out))),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapErr<O, E, X, P, Z, FUNC>
where 
    P: Parser<O, E, X>, 
    O: Clone,
    E: Clone,
    Z: Clone,
    FUNC: Fn(X, E) -> Z,
    X: Extra<O, E> + Extra<O, Z>
{
    map: FUNC,
    inner: P,
    phantom: PhantomData<(O, E, X, Z)>
}
impl<O, E, X, P, Z, FUNC> Parser<O, Z, X> for MapErr<O, E, X, P, Z, FUNC>
where 
    P: Parser<O, E, X>, 
    O: Clone,
    E: Clone,
    Z: Clone,
    FUNC: Fn(X, E) -> Z,
    X: Extra<O, E> + Extra<O, Z>
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, O), (usize, Z)> {
        match self.inner.parse(input, progress, extra.clone()) {
            Err((progress, err)) => Err((progress, (self.map)(extra.clone(), err))),
            Ok(o) => Ok(o),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Token<X: Extra<(), ()>> {
    token: &'static str, 
    alloc: PhantomData<X>,
}
impl<X> Token<X>
    where X: Extra<(), ()>
{
    pub fn new(token: &'static str) -> Tag<(), (), X, Self> {
        Tag::new(Token { token, alloc: PhantomData })
    }
}
impl<X> Parser<(), (), X> for Token<X>
    where X: Extra<(), ()>
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, ()), (usize, ())> {
        if input[progress..].starts_with(self.token) {
            log::debug!("TOKEN={:?} MATCHED", self.token);
            Ok((progress + self.token.len(), ()))
        }
        else {
            log::debug!("TOKEN={:?} SEEN={:?}", self.token, &input[progress..(progress+self.token.len()).min(input.len())]);
            Err((progress, ()))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pad<X>(PhantomData<X>);
impl<X> Pad<X>
    where X: Extra<(), ()>
{
    pub fn new() -> Tag<(), (), X, Self> {
        Tag::new(Pad(PhantomData))
    }
}
impl<X> Parser<(), (), X> for Pad<X>
    where X: Extra<(), ()>
{
    fn parse(&self, input: &str, progress: usize, extra: X) -> Result<(usize, ()), (usize, ())> {
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
        Ok((progress + cut, ()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bumpalo::Bump;

    impl<'a, O: Clone + 'a, E: Clone + 'a> Extra<O, E> for &'a Bump {}

    #[test]
    fn alphabet() {
        let bump = Bump::new();
        let a = || Token::new("a");
        let b = || Token::new("b");
        fn take_left<A, B, C>(_: C, (a, b): (A, B)) -> A { a }
        fn unwrap<A, B, C>(_: C, a: Either<A, B>) -> () { () }
        let parser = recurse::<i64, (), &Bump, _>(|this| {
            (a() + this.clone()).out(|extra, (lhs, rhs)| {
                rhs + 20
            })
            ^ 
            (b().out(|extra: &Bump, _| 1))
        }.err(|extra, _| ()));
        let example = "aaabb";
        let this = parser.parse(&example, 0, &bump);
        assert!(61 == this.unwrap().1);
    }
}