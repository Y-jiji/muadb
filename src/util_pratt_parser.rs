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
    // record execution result
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, O), (usize, E)>)   {  }
    // replay an expression
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, O), (usize, E)>> { None }
}

pub trait Visited {
    fn visited() -> Self;
}
impl Visited for () {
    fn visited() -> Self { () }
}

// a general parser trait
#[auto_impl::auto_impl(&, Box)]
pub trait Parser<O, E, X>
    where
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, O), (usize, E)>;
}

#[derive(Debug)]
pub struct Tag<O, E, X, P>
    where
        P: Parser<O, E, X>,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    inner: P, 
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
        Self { inner: self.inner.clone(), phantom: PhantomData }
    }
}
impl<O, E, X, P> Parser<O, E, X> for Tag<O, E, X, P>
    where
        P: Parser<O, E, X>,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, O), (usize, E)> {
        self.inner.parse(input, progress, extra)
    }
}
impl<O, E, X, P> Tag<O, E, X, P>
    where
        P: Parser<O, E, X>,
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone
{
    pub fn new(inner: P) -> Self {
        Tag{inner, phantom: PhantomData}
    }
    pub fn pad(self) -> Tag<O, E, X, Pad<O, E, X, P>> {
        Tag{inner: Pad(self.inner, PhantomData), phantom: PhantomData}
    }
    pub fn erase(self) -> Tag<O, E, X, Box<dyn Parser<O, E, X> + 'static>> {
        let inner: Box<dyn Parser<O, E, X> + 'static> = unsafe{ std::mem::transmute(Box::new(self.inner) as Box<dyn Parser<O, E, X>>) };
        Tag { inner, phantom: PhantomData }
    }
    pub fn out<Z, FUNC>(self, map: FUNC) -> Tag<Z, E, X, MapOut<O, E, X, P, Z, FUNC>>
        where X: Extra<Z, E>,
              Z: Clone,
              FUNC: Fn(&mut X, O) -> Z,
    {
        Tag{
            inner: MapOut{map, inner: self.inner, phantom: PhantomData}, 
            phantom: PhantomData
        }
    }
    pub fn err<Z, FUNC>(self, map: FUNC) -> Tag<O, Z, X, MapErr<O, E, X, P, Z, FUNC>>
        where X: Extra<O, Z>,
              Z: Clone,
              FUNC: Fn(&mut X, usize, E) -> Z,
    {
        Tag{
            inner: MapErr{map, inner: self.inner, phantom: PhantomData}, 
            phantom: PhantomData
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {L(L), R(R)}

impl<L> Either<L, L> {
    pub fn unwrap(self) -> L {
        match self { Either::L(x) => x, Either::R(x) => x }
    }
}

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
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, (OP, OQ)), (usize, Either<EP, EQ>)> {
        let start = progress;
        let (progress, lhs) = match Parser::parse(&self.lhs, input, progress, extra) {
            Ok((progress, lhs)) => (progress, lhs),
            Err((progress, err)) => return Err((start, Either::L(err)))
        };
        let (progress, rhs) = match Parser::parse(&self.rhs, input, progress, extra) {
            Ok((progress, rhs)) => (progress, rhs),
            Err((progress, err)) => return Err((start, Either::R(err)))
        };
        Ok((progress, (lhs, rhs)))
    }
}
impl<OP, OQ, E, X, P, Q> Add<Tag<OQ, E, X, Q>> for Tag<OP, E, X, P>
    where 
          X: Extra<(OP, OQ), Either<E, E>> + Extra<(OP, OQ), E>,
          X: Extra<OP, E> + Extra<OQ, E>, 
          P: Parser<OP, E, X>, 
          Q: Parser<OQ, E, X>,
          E: Clone, OP: Clone, OQ: Clone,
{
    type Output = Tag<(OP, OQ), E, X, MapErr<(OP, OQ), Either<E, E>, X, Then<OP, E, OQ, E, X, Tag<OP, E, X, P>, Tag<OQ, E, X, Q>>, E, for<'a> fn(&'a mut X, usize, Either<E, E>) -> E>>;
    /// parse with self and rhs, merge the result
    fn add(self, rhs: Tag<OQ, E, X, Q>) -> Self::Output {
        fn map<'a, A, Z>(a: &'a mut A, _: usize, b: Either<Z, Z>) -> Z {
            match b { Either::L(x) => x, Either::R(x) => x }
        }
        Tag::new(Then { lhs: self, rhs, phant: PhantomData }).err(map as for<'a> fn(&'a mut _, _, _) -> _)
    }
}
impl<OP, EP, OQ, EQ, X, P, Q> Mul<Tag<OQ, EQ, X, Q>> for Tag<OP, EP, X, P>
    where 
          X: Extra<(OP, OQ), Either<EP, EQ>>,
          X: Extra<OP, EP> + Extra<OQ, EQ>, 
          P: Parser<OP, EP, X>, 
          Q: Parser<OQ, EQ, X>,
          OP: Clone, OQ: Clone,
          EP: Clone, EQ: Clone
{
    type Output = Tag<(OP, OQ), Either<EP, EQ>, X, Then<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>>;
    fn mul(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
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
    type Output = Tag<OQ, Either<EP, EQ>, X, MapOut<(OP, OQ), Either<EP, EQ>, X, Then<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>, OQ, fn(&mut X, (OP, OQ)) -> OQ>>;
    fn rem(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        fn unwrap<X, OP, OQ>(extra: &mut X, (p, q): (OP, OQ)) -> OQ { q }
        (self * rhs).out(unwrap)
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
    type Output = Tag<OP, Either<EP, EQ>, X, MapOut<(OP, OQ), Either<EP, EQ>, X, Then<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>, OP, fn(&mut X, (OP, OQ)) -> OP>>;
    fn div(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        fn unwrap<X, OP, OQ>(extra: &mut X, (p, q): (OP, OQ)) -> OP { p }
        (self * rhs).out(unwrap)
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
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, Either<OP, OQ>), (usize, (EP, EQ))> {
        let start = progress;
        let (progress, lhs) = match Parser::parse(&self.lhs, input, progress, extra) {
            Err((progress, lhs)) => (start, lhs),
            Ok((progress, err)) => return Ok((progress, Either::L(err)))
        };
        let (progress, rhs) = match Parser::parse(&self.rhs, input, progress, extra) {
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
    /// parse with self, or fallback to rhs
    type Output = Tag<Either<OP, OQ>, (EP, EQ), X, Else<OP, EP, OQ, EQ, X, Tag<OP, EP, X, P>, Tag<OQ, EQ, X, Q>>>;
    fn bitor(self, rhs: Tag<OQ, EQ, X, Q>) -> Self::Output {
        Tag::new(Else { lhs: self, rhs, phant: PhantomData })
    }
}

pub trait MergeIn<X> {
    fn merge(self, with: Self, x: &mut X) -> Self;
    fn merge_with(x: &mut X, _: usize, y: (Self, Self)) -> Self where Self: Sized {
        y.0.merge(y.1, x)
    }
}

impl<O, E, X, P, Q> BitXor<Tag<O, E, X, Q>> for Tag<O, E, X, P>
where 
    X: Extra<O, (E, E)> + Extra<O, E> + Extra<Either<O, O>, (E, E)>, 
    P: Parser<O, E, X>, 
    Q: Parser<O, E, X>,
    O: Clone,
    E: MergeIn<X> + Clone,
{
    type Output = Tag<O, E, X, MapErr<O, (E, E), X, MapOut<Either<O, O>, (E, E), X, Else<O, E, O, E, X, Tag<O, E, X, P>, Tag<O, E, X, Q>>, O, for<'a> fn(&'a mut X, Either<O, O>) -> O>, E, for<'a> fn(&'a mut X, usize, (E, E)) -> E>>;
    /// parse with self, or fallback to rhs
    /// when both self and rhs fail, we use MergeIn<X> trait to merge errors
    fn bitxor(self, rhs: Tag<O, E, X, Q>) -> Self::Output {
        fn map<'a, A, Z>(a: &'a mut A, b: Either<Z, Z>) -> Z {
            match b { Either::L(x) => x, Either::R(x) => x }
        }
        fn err<'b, A, Z: MergeIn<A>>(a: &'b mut A, _: usize, b: (Z, Z)) -> Z {
            b.0.merge(b.1, a)
        }
        // These length function signatures are necessary because rust cannot infer the for<'a> lifetime
        (self | rhs)
            .out(map as for<'a> fn(&'a mut _, _) -> _)
            .err(err as for<'a> fn(&'a mut _, _, _) -> _)
    }
}

static COUNT: AtomicU64 = AtomicU64::new(1);
use std::sync::atomic::Ordering::SeqCst;
pub struct Recursive<O, E, X> {
    this: Arc<OnceCell<Box<dyn Parser<O, E, X>>>>,
    tag: u64,
}
impl<O, E, X> Parser<O, E, X> for Recursive<O, E, X> 
    where
        X: Extra<O, E> + Clone, 
        O: Clone, 
        E: Clone + Visited
{
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, O), (usize, E)> {
        log::debug!("{}", &input[progress..]);
        if let Some(result) = extra.replay(progress, self.tag) { return result }
        let result = 
            self.this.get().expect("UNINITIALIZED RECURSIVE PARSER").parse(input, progress, extra);
        extra.record(progress, self.tag, result.clone());
        return result;
    }
}
pub fn recurse<O, E, X, P>(builder: impl FnOnce(Tag<O, E, X, Recursive<O, E, X>>) -> P) -> Tag<O, E, X, Recursive<O, E, X>>
where
    X: Extra<O, E> + Clone, 
    O: Clone, 
    E: Clone + Visited,
    P: Parser<O, E, X>
{
    //! Safety: types that implements Parser<O, E, X> are constitutes functions (input, X) -> Result<(O, usize), (E, usize)>
    //! Therefore, if X, O, E don't change, the function signature don't change. 
    //! Therefore, we don't require X, O, E to have 'static lifetime. 
    let tag = COUNT.fetch_add(1, SeqCst);
    let this: Tag<O, E, X, Recursive<O, E, X>> = Tag::new(Recursive{
        tag, this: Arc::new(OnceCell::new())
    });
    let that = Box::new(Tag::new(builder(this.clone())));
    // UNSAFE HERE
    this.inner.this.as_ref().set(unsafe{ std::mem::transmute(that as Box<dyn Parser<O, E, X>>) });
    return this;
}
impl<O, E, X> std::fmt::Debug for Recursive<O, E, X> 
    where X: Extra<O, E>,
          O: Clone,
          E: Clone
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Recursive(tag: {})", &self.tag)
    }
}
impl<O, E, X> Clone for Recursive<O, E, X> {
    fn clone(&self) -> Self {
        Recursive{
            this: Arc::clone(&self.this),
            tag: self.tag
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Repeat<O, E, X, P, Z, INIT, FOLD>
    where P: Parser<O, E, X>, 
          O: Clone, 
          E: Clone,
          Z: Clone,
          FOLD: Fn(&mut X, Z, O) -> Z,
          INIT: Fn(&mut X) -> Z,
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
        FOLD: Fn(&mut X, Z, O) -> Z,
        INIT: Fn(&mut X) -> Z,
        X: Extra<O, E> + Extra<Z, E> + Clone,
{
    fn parse(&self, input: &str, mut progress: usize, extra: &mut X) -> Result<(usize, Z), (usize, E)> {
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
impl<O, E, X, P, Z, INIT, FOLD> Shr<(INIT, FOLD)> for Tag<O, E, X, P>
    where
        P: Parser<O, E, X>, 
        O: Clone, 
        E: Clone,
        Z: Clone,
        FOLD: Fn(&mut X, Z, O) -> Z,
        INIT: Fn(&mut X) -> Z,
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
    FUNC: Fn(&mut X, O) -> Z,
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
    FUNC: Fn(&mut X, O) -> Z,
    X: Extra<O, E> + Extra<Z, E>
{
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, Z), (usize, E)> {
        match self.inner.parse(input, progress, extra) {
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
    FUNC: Fn(&mut X, usize, E) -> Z,
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
    FUNC: Fn(&mut X, usize, E) -> Z,
    X: Extra<O, E> + Extra<O, Z>
{
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, O), (usize, Z)> {
        match self.inner.parse(input, progress, extra) {
            Err((_, err)) => Err((progress, (self.map)(extra, progress, err))),
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
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, ()), (usize, ())> {
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
pub struct Pad<O, E, X, P>(P, PhantomData<(O, E, X)>);
impl<O, E, X, P> Parser<O, E, X> for Pad<O, E, X, P>
    where P: Parser<O, E, X>,
          X: Extra<O, E> + Clone, 
          O: Clone, 
          E: Clone
          
{
    fn parse(&self, input: &str, progress: usize, extra: &mut X) -> Result<(usize, O), (usize, E)> {
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
        // I know it is bad to copy a piece of code...
        let progress = cut + progress;
        let (progress, out) =self.0.parse(input, progress, extra)?;
        let mut last = true;
        for (i, c) in input[progress..].char_indices() {
            cut = i;
            if c.is_whitespace() { continue }
            last = false;
            break;
        }
        if last { cut = input[progress..].len() }
        log::debug!("EAT SPACE={cut}");
        let progress = cut + progress;
        Ok((progress, out))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bumpalo::Bump;

    impl<'a, O: Clone + 'a, E: Clone + 'a> Extra<O, E> for &'a Bump {}
    impl<'a> MergeIn<&'a Bump> for () {
        fn merge(self, with: Self, x: &mut &'a Bump) -> Self { () }
    }

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
            (b().out(|extra, _| 1))
        }.err(|extra, _, _| ()));
        let example = "aaabb";
        let this = parser.parse(&example, 0, &mut &bump);
        assert!(61 == this.unwrap().1);
    }
}