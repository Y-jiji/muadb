use std::{any::Any, cell::OnceCell, fmt::Debug, marker::PhantomData, ops::{Add, BitOr}, os::unix::process, sync::{atomic::AtomicU64, Arc}};

#[allow(unused)]
pub trait Extra<O, E> {
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, &O), (usize, &E)>)   {  }
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, &O), (usize, &E)>> { None }
    fn out(&self, o: O) -> &O;
    fn err(&self, e: E) -> &E;
}

#[auto_impl::auto_impl(Box, &)]
pub trait Parser<'p>: Sized {
    type O: 'p;
    type E: 'p;
    type X: Extra<Self::O, Self::E>;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)>;
    fn tag(&self) -> u64 { 0 }
}

#[derive(Debug, Clone, Copy)]
pub struct Memorized<P: for<'p> Parser<'p>>{inner: P, tag: u64}
impl<'q, P: for<'p> Parser<'p>> Parser<'q> for Memorized<P> {
    type O = <P as Parser<'q>>::O;
    type E = <P as Parser<'q>>::E;
    type X = <P as Parser<'q>>::X;
    fn parse(&self, input: &str, progress: usize, extra: &'q Self::X) -> Result<(usize, &'q Self::O), (usize, &'q Self::E)> {
        if let Some(result) = extra.replay(progress, self.tag) {
            return result;
        }
        let result = self.inner.parse(input, progress, extra);
        extra.record(progress, self.tag, result.clone());
        return result;
    }
    fn tag(&self) -> u64 { self.tag }
}
impl<'q, P: for<'p> Parser<'p>> Memorized<P> {
    pub fn new(inner: P) -> Self {
        static COUNT: AtomicU64 = AtomicU64::new(1);
        use std::sync::atomic::Ordering::SeqCst;
        let tag = if inner.tag() != 0 { inner.tag() } else { COUNT.fetch_add(1, SeqCst) };
        Memorized{inner, tag}
    }
}
impl<P: for<'p> Parser<'p>, Q: for<'q> Parser<'q, X=<P as Parser<'q>>::X>> Add<Memorized<Q>> for Memorized<P> 
    where P: for<'p> Parser<'p>, 
          Q: for<'q> Parser<'q, X = <P as Parser<'q>>::X>,
          for<'r> <P as Parser<'r>>::X: Extra<(&'r <P as Parser<'r>>::O, &'r <Q as Parser<'r>>::O), Either<&'r <P as Parser<'r>>::E, &'r <Q as Parser<'r>>::E>> 
{
    type Output = Memorized<Then<Memorized<P>, Memorized<Q>>>;
    fn add(self, rhs: Memorized<Q>) -> Self::Output {
        Memorized::new(Then { lhs: self, rhs })
    }
}
impl<P: for<'p> Parser<'p>, Q: for<'q> Parser<'q, X=<P as Parser<'q>>::X>> BitOr<Memorized<Q>> for Memorized<P> 
    where P: for<'p> Parser<'p>, 
          Q: for<'q> Parser<'q, X = <P as Parser<'q>>::X>,
          for<'r> <P as Parser<'r>>::X: Extra<Either<&'r <P as Parser<'r>>::O, &'r <Q as Parser<'r>>::O>, (&'r <P as Parser<'r>>::E, &'r <Q as Parser<'r>>::E)> 
{
    type Output = Memorized<Else<Memorized<P>, Memorized<Q>>>;
    fn bitor(self, rhs: Memorized<Q>) -> Self::Output {
        Memorized::new(Else { lhs: self, rhs })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {L(L), R(R)}

#[derive(Debug, Clone, Copy)]
pub struct Then<P, Q>
    where P: for<'p> Parser<'p>, 
          Q: for<'q> Parser<'q, X = <P as Parser<'q>>::X>
{lhs: P, rhs: Q}
impl<'r, P, Q> Parser<'r> for Then<P, Q> 
    where P: for<'p> Parser<'p>, 
          Q: for<'q> Parser<'q, X = <P as Parser<'q>>::X>,
          <P as Parser<'r>>::X: Extra<(&'r <P as Parser<'r>>::O, &'r <Q as Parser<'r>>::O), Either<&'r <P as Parser<'r>>::E, &'r <Q as Parser<'r>>::E>> 
{
    type O = (&'r <P as Parser<'r>>::O, &'r <Q as Parser<'r>>::O);
    type E = Either<&'r <P as Parser<'r>>::E, &'r <Q as Parser<'r>>::E>;
    type X = <P as Parser<'r>>::X;
    fn parse(&self, input: &str, progress: usize, extra: &'r Self::X) -> Result<(usize, &'r Self::O), (usize, &'r Self::E)> {
        let (progress, lhs) = match Parser::<'r>::parse(&self.lhs, input, progress, extra) {
            Ok((progress, lhs)) => (progress, lhs),
            Err((progress, err)) => return Err((progress, extra.err(Either::L(err))))
        };
        let (progress, rhs) = match Parser::<'r>::parse(&self.rhs, input, progress, extra) {
            Ok((progress, rhs)) => (progress, rhs),
            Err((progress, err)) => return Err((progress, extra.err(Either::R(err))))
        };
        Ok((progress, extra.out((lhs, rhs))))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Else<P, Q>
    where P: for<'p> Parser<'p>, 
          Q: for<'q> Parser<'q, X = <P as Parser<'q>>::X>
{lhs: P, rhs: Q}
impl<'r, P, Q> Parser<'r> for Else<P, Q> 
    where P: for<'p> Parser<'p>, 
          Q: for<'q> Parser<'q, X = <P as Parser<'q>>::X>,
          <P as Parser<'r>>::X: Extra<Either<&'r <P as Parser<'r>>::O, &'r <Q as Parser<'r>>::O>, (&'r <P as Parser<'r>>::E, &'r <Q as Parser<'r>>::E)> 
{
    type E = (&'r <P as Parser<'r>>::E, &'r <Q as Parser<'r>>::E);
    type O = Either<&'r <P as Parser<'r>>::O, &'r <Q as Parser<'r>>::O>;
    type X = <P as Parser<'r>>::X;
    fn parse(&self, input: &str, progress: usize, extra: &'r Self::X) -> Result<(usize, &'r Self::O), (usize, &'r Self::E)> {
        let (progress, lhs) = match Parser::<'r>::parse(&self.lhs, input, progress, extra) {
            Err((progress, lhs)) => (progress, lhs),
            Ok((progress, out)) => return Ok((progress, extra.out(Either::L(out))))
        };
        let (progress, rhs) = match Parser::<'r>::parse(&self.rhs, input, progress, extra) {
            Err((progress, rhs)) => (progress, rhs),
            Ok((progress, out)) => return Ok((progress, extra.out(Either::R(out))))
        };
        Err((progress, extra.err((lhs, rhs))))
    }
}

pub struct Recursive<O, E, X>(Arc<OnceCell<Box<dyn for<'a, 'r> Fn(&'a str, usize, &'r X) -> Result<(usize, &'r O), (usize, &'r E)>>>>);
impl<'r, O: 'r, E: 'r, X> Parser<'r> for Recursive<O, E, X> 
    where X: Extra<O, E>
{
    type E = E;
    type O = O;
    type X = X;
    fn parse(&self, input: &str, progress: usize, extra: &'r Self::X) -> Result<(usize, &'r Self::O), (usize, &'r Self::E)> {
        (self.0.as_ref().get().unwrap())(input, progress, extra)
    }
}
pub fn recurse<O, E, X, P: for<'a> Parser<'a, E=E, O=O, X=X> + 'static>(builder: impl FnOnce(Recursive<O, E, X>) -> P) -> Recursive<O, E, X> {
    let this = Recursive(Arc::new(OnceCell::new()));
    let that = builder(this.clone());
    this.0.as_ref().set(Box::new(move |input, progress, extra| that.parse(input, progress, extra)));
    return this;
}
impl<'a, O, E, X> Debug for Recursive<O, E, X> {
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
pub struct MapOut<P: for<'p> Parser<'p>, Z>(P, for<'a> fn(&'a <P as Parser<'a>>::X, &'a <P as Parser<'a>>::O) -> &'a Z);
impl<'q, P: for<'p> Parser<'p>, Z: 'q> Parser<'q> for MapOut<P, Z> 
    where <P as Parser<'q>>::X: Extra<Z, <P as Parser<'q>>::E>
{
    type E = <P as Parser<'q>>::E;
    type O = Z;
    type X = <P as Parser<'q>>::X;
    fn parse(&self, input: &str, progress: usize, extra: &'q Self::X) -> Result<(usize, &'q Self::O), (usize, &'q Self::E)> {
        match self.0.parse(input, progress, extra) {
            Ok((progress, out)) => Ok((progress, self.1(extra, out))),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapErr<P: for<'p> Parser<'p>, Z>(P, for<'a> fn(&'a <P as Parser<'a>>::X, &'a <P as Parser<'a>>::E) -> &'a Z);
impl<'q, P: for<'p> Parser<'p>, Z: 'q> Parser<'q> for MapErr<P, Z> 
    where <P as Parser<'q>>::X: Extra<<P as Parser<'q>>::O, Z>
{
    type E = Z;
    type O = <P as Parser<'q>>::O;
    type X = <P as Parser<'q>>::X;
    fn parse(&self, input: &str, progress: usize, extra: &'q Self::X) -> Result<(usize, &'q Self::O), (usize, &'q Self::E)> {
        match self.0.parse(input, progress, extra) {
            Err((progress, out)) => Err((progress, self.1(extra, out))),
            Ok(e) => Ok(e),
        }
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

    pub struct Token(&'static str);
    pub struct NotMatch(&'static str);
    impl<'a> Parser<'a> for Token {
        type E = NotMatch;
        type O = &'static str;
        type X = Bump;
        fn parse(&self, input: &str, progress:usize, extra: &'a Self::X) -> Result<(usize, &'a Self::O),(usize, &'a Self::E)> {
            if input[progress..].starts_with(self.0) {
                Ok((progress + self.0.len(), extra.alloc(self.0)))
            }
            else {
                Err((progress, extra.alloc(NotMatch(self.0))))
            }
        }
    }

    #[test]
    fn arithemetic() {
        let bump = Bump::new();
        let a = || Token("a");
        let b = || Token("b");
        fn take_left<A, B, C>(_: C, (a, b): (A, B)) -> A { a }
        fn unwrap<A, B, C>(_: C, a: Either<A, B>) -> () { () }
        let parser = recurse::<usize, (), Bump, _>(|this| {
            MapOut(MapErr(Else { lhs: Then { lhs: a(), rhs: this }, rhs: b() }, |extra, _| extra.alloc(())), |extra, _| extra.alloc(0usize))
        });
        let example = "aaabb";
        println!("{:?}", parser.parse(&example, 0, &bump));
    }
}