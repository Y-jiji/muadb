use std::{any::Any, cell::OnceCell, fmt::Debug, marker::PhantomData, ops::{Add, BitOr, BitXor, Shr, Rem}, os::unix::process, sync::{atomic::AtomicU64, Arc}};

// memorization buffer + output/error allocation buffer
pub trait Extra<O, E> {
    // mark a progress is visited by a parser
    fn mark(&self, progress: usize, tag: u64) {}
    // record execution result
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, &O), (usize, &E)>)   {  }
    // replay an expression
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, &O), (usize, &E)>> { None }
    // allocate output
    fn out(&self, o: O) -> &O;
    // allocate error
    fn err(&self, e: E) -> &E;
}

pub trait Parser<'p>: Sized {
    type O: 'p;
    type E: 'p;
    type X: Extra<Self::O, Self::E> + 'p;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)>;
    fn tag(&self) -> u64 { 0 }
    fn tagged(self) -> Tag<'p, Self> { Tag::new(self) }
}

#[derive(Debug, Clone, Copy)]
pub struct Tag<'p, P: Parser<'p>>{inner: P, tag: u64, phantom: PhantomData<fn(&'p())->()>}
impl<'p, P: Parser<'p>> Parser<'p> for Tag<'p, P> {
    type O = P::O;
    type E = P::E;
    type X = P::X;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
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
impl<'p, P: Parser<'p>> Tag<'p, P> {
    pub fn new(inner: P) -> Self {
        static COUNT: AtomicU64 = AtomicU64::new(1);
        use std::sync::atomic::Ordering::SeqCst;
        let tag = if inner.tag() != 0 { 0 } else {
            COUNT.fetch_add(1, SeqCst)
        };
        Tag{inner, tag, phantom: PhantomData}
    }
    pub fn out<Z: 'p, F>(self, f: F) -> Tag<'p, MapOut<'p, P, Z, F>>
        where P::X: Extra<Z, P::E>,
              F: Fn(&'p P::X, &'p P::O) -> &'p Z
    {
        Tag{inner:MapOut(self.inner, f, PhantomData), tag:self.tag, phantom: PhantomData}
    }
    pub fn err<Z: 'p>(self, f: fn(&'p P::X, &'p P::E) -> &'p Z) -> Tag<'p, MapErr<'p, P, Z>>
        where P::X: Extra<P::O, Z>
    {
        Tag{inner:MapErr(self.inner, f), tag:self.tag, phantom: PhantomData}
    }
}
impl<'p, P, Q> Add<Q> for Tag<'p, P>
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>,
          P::X: Extra<(&'p P::O, &'p Q::O), Either<&'p P::E, &'p Q::E>>,
{
    type Output = Tag<'p, Then<'p, Tag<'p, P>, Q>>;
    fn add(self, rhs: Q) -> Self::Output {
        Tag::new(Then { lhs: self, rhs, phantom: PhantomData })
    }
}
impl<'p, P, Q> BitOr<Q> for Tag<'p, P>
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>,
          P::X: Extra<Either<&'p P::O, &'p Q::O>, (&'p P::E, &'p Q::E)> 
{
    type Output = Tag<'p, Else<'p, Tag<'p, P>, Q>>;
    fn bitor(self, rhs: Q) -> Self::Output {
        Tag::new(Else { lhs: self, rhs, phantom: PhantomData })
    }
}
impl<'p, P, Q> BitXor<Q> for Tag<'p, P>
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X, O = P::O>,
          P::X: Extra<Either<&'p P::O, &'p Q::O>, (&'p P::E, &'p Q::E)>,
          P::X: Extra<P::O, (&'p P::E, &'p Q::E)>,
{
    type Output = Tag<'p, MapOut<'p, Else<'p, Tag<'p, P>, Q>, P::O, fn(&'p P::X, &'p Either<&'p Q::O, &'p P::O>) -> &'p P::O>>;
    fn bitxor(self, rhs: Q) -> Self::Output {
        use Either::*;
        fn unwrap<'q, O: 'q, A: 'q, B: 'q, X: Extra<O, (&'q A, &'q B)>>(x: &'q X, either: &'q Either<&'q O, &'q O>) -> &'q O {
            match either { L(l) => *l, R(r) => *r }
        }
        Tag::new(Else { lhs: self, rhs, phantom: PhantomData }).out(unwrap)
    }
}
impl<'p, P, Q> Rem<Q> for Tag<'p, P>
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>,
          P::X: Extra<(&'p P::O, &'p Q::O), Either<&'p P::E, &'p Q::E>>,
          P::X: Extra<Q::O, Either<&'p P::E, &'p Q::E>>,
{
    type Output = Tag<'p, MapOut<'p, Then<'p, Tag<'p, P>, Q>, Q::O, fn(&'p P::X, &'p (&'p P::O, &'p Q::O)) -> &'p Q::O>>;
    fn rem(self, rhs: Q) -> Self::Output {
        fn unwrap<'q, E: 'q, A: 'q, B: 'q, X: Extra<B, E>>(x: &'q X, (_, y): &'q (&'q A, &'q B)) -> &'q B {
            *y
        }
        Tag::new(Then { lhs: self, rhs, phantom: PhantomData }).out(unwrap)
    }
}
impl<'p, P, Z: 'p, FOLD, INI> Shr<(INI, FOLD)> for Tag<'p, P>
    where P: Parser<'p>, 
    P::X: Extra<Z, ()>,
    FOLD: Fn(&'p P::X, &'p Z, &'p P::O) -> &'p Z,
    INI: Fn(&'p P::X) -> &'p Z
{
    type Output = Tag<'p, Repeat<'p, Tag<'p, P>, Z, INI, FOLD>>;
    fn shr(self, (ini, fold): (INI, FOLD)) -> Self::Output {
        Tag::new(Repeat(self, ini, fold, PhantomData))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {L(L), R(R)}

#[derive(Debug, Clone, Copy)]
pub struct Then<'p, P, Q>
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>
{lhs: P, rhs: Q, phantom: PhantomData<fn(&'p())->()>}
impl<'p, P, Q> Parser<'p> for Then<'p, P, Q> 
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>,
          P::X: Extra<(&'p P::O, &'p Q::O), Either<&'p P::E, &'p Q::E>>
{
    type O = (&'p P::O, &'p Q::O);
    type E = Either<&'p P::E, &'p Q::E>;
    type X = P::X;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
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

#[derive(Debug, Clone, Copy)]
pub struct Else<'p, P, Q>
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>
{lhs: P, rhs: Q, phantom: PhantomData<fn(&'p())->()>}
impl<'p, P, Q> Parser<'p> for Else<'p, P, Q> 
    where P: Parser<'p>, 
          Q: Parser<'p, X = P::X>,
          P::X: Extra<Either<&'p P::O, &'p Q::O>, (&'p P::E, &'p Q::E)> 
{
    type E = (&'p P::E, &'p Q::E);
    type O = Either<&'p P::O, &'p Q::O>;
    type X = P::X;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
        let start = progress;
        let (progress, lhs) = match Parser::<'p>::parse(&self.lhs, input, progress, extra) {
            Err((progress, lhs)) => (start, lhs),
            Ok((progress, out)) => return Ok((progress, extra.out(Either::L(out))))
        };
        let (progress, rhs) = match Parser::<'p>::parse(&self.rhs, input, progress, extra) {
            Err((progress, rhs)) => (start, rhs),
            Ok((progress, out)) => return Ok((progress, extra.out(Either::R(out))))
        };
        Err((progress, extra.err((lhs, rhs))))
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
impl<'p, O: 'p, E: 'p, X> Parser<'p> for Recursive<'p, O, E, X> 
    where X: Extra<O, E>
{
    type E = E;
    type O = O;
    type X = X;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
        let this = self.clone();
        let Helper{that, parse, ..} = this.0.as_ref().get().unwrap();
        parse(*that, input, progress, extra)
    }
}
pub fn recurse<'p, O: 'p, E: 'p, X, P>(builder: impl FnOnce(Tag<'p, Recursive<'p, O, E, X>>) -> P) -> Tag<'p, Recursive<'p, O, E, X>>
    where X: Extra<O, E>,
          P: Parser<'p, E=E, O=O, X=X>,
{
    let this = Tag::new(Recursive(Arc::new(OnceCell::new())));
    let that = Box::leak(Box::new(Tag::new(builder(this.clone())))) as *const _ as *const u8;
    // make sure this "leak" operation is legit
    assert!(std::mem::size_of::<&mut Tag<P>>() == std::mem::size_of::<*const u8>());
    // cast parse and destruct function
    let parse = unsafe {std::mem::transmute(Tag::<P>::parse as fn(_, _, _, _) -> _)};
    let destruct = unsafe {std::mem::transmute::<_, fn(*const u8)>(drop::<Box<Tag<P>>> as fn(_) -> _)};
    // set self to be inner function
    this.inner.0.as_ref().set(Helper{that, parse, destruct});
    return this;
}
impl<'a, O, E, X> Debug for Recursive<'a, O, E, X> {
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
pub struct Repeat<'p, P, Z, INI, M>(P, INI, M, PhantomData<(fn(&'p ()) -> Z)>)
    where P: Parser<'p>, Z: 'p, 
          M: Fn(&'p P::X, &'p Z, &'p P::O) -> &'p Z,
          INI: Fn(&'p P::X) -> &'p Z;
impl<'p, P: Parser<'p>, Z: 'p, INI, M> Parser<'p> for Repeat<'p, P, Z, INI, M>
    where P::X: Extra<Z, ()>,
          P: Parser<'p>, Z: 'p,
          M: Fn(&'p P::X, &'p Z, &'p P::O) -> &'p Z,
          INI: Fn(&'p P::X) -> &'p Z
{
    type E = ();
    type O = Z;
    type X = P::X;
    fn parse(&self, input: &str, mut progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
        let mut ini = self.1(extra);
        loop {
            let (progress_new, out) = match self.0.parse(input, progress, extra) {
                Ok((progress, out)) => (progress, self.2(extra, ini, out)),
                Err(e) => return Ok((progress, ini)),
            };
            progress = progress_new;
            ini = out;
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapOut<'p, P, Z, M>(P, M, PhantomData<(fn(&'p ()) -> Z)>)
    where P: Parser<'p>, Z: 'p, M: Fn(&'p P::X, &'p P::O) -> &'p Z;
impl<'p, P: Parser<'p>, Z: 'p, M> Parser<'p> for MapOut<'p, P, Z, M>
    where P::X: Extra<Z, P::E>,
          P: Parser<'p>, Z: 'p, 
          M: Fn(&'p P::X, &'p P::O) -> &'p Z
{
    type E = P::E;
    type O = Z;
    type X = P::X;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
        match self.0.parse(input, progress, extra) {
            Ok((progress, out)) => Ok((progress, self.1(extra, out))),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MapErr<'p, P: Parser<'p>, Z: 'p>(P, fn(&'p P::X, &'p P::E) -> &'p Z);
impl<'p, P: Parser<'p>, Z: 'p> Parser<'p> for MapErr<'p, P, Z> 
    where P::X: Extra<P::O, Z>
{
    type E = Z;
    type O = P::O;
    type X = P::X;
    fn parse(&self, input: &str, progress: usize, extra: &'p Self::X) -> Result<(usize, &'p Self::O), (usize, &'p Self::E)> {
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
    #[derive(Debug)]
    pub struct NotMatch(&'static str);
    impl<'a> Parser<'a> for Token {
        type E = NotMatch;
        type O = &'static str;
        type X = Bump;
        fn parse(&self, input: &str, progress:usize, extra: &'a Self::X) -> Result<(usize, &'a Self::O), (usize, &'a Self::E)> {
            if input[progress..].starts_with(self.0) {
                Ok((progress + self.0.len(), extra.alloc(self.0)))
            }
            else {
                Err((progress, extra.alloc(NotMatch(self.0))))
            }
        }
    }

    #[test]
    fn alphabet() {
        let bump = Bump::new();
        let a = || Tag::new(Token("a"));
        let b = || Tag::new(Token("b"));
        fn take_left<A, B, C>(_: C, (a, b): (A, B)) -> A { a }
        fn unwrap<A, B, C>(_: C, a: Either<A, B>) -> () { () }
        let parser = recurse::<i64, (), Bump, _>(|this| {
            (a() + this.clone()).out(|extra, (lhs, rhs)| {
                extra.alloc(**rhs + 20)
            }) 
            ^ (b().out(|extra, _| extra.alloc(1)))
        }.err(|extra, _| extra.alloc(())));
        // let parser = a() + b();
        let example = "aaabb";
        let this = parser.parse(&example, 0, &bump);
        assert!(&61i64 == this.unwrap().1);
    }
}