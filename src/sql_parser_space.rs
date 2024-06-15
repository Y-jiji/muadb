use std::{cell::{Cell, RefCell}, ops::Add};
use bumpalo::{Bump, collections::Vec as BVec};
use crate::util_pratt_parser::*;

// Each SQL is allocated in this holder structure. 
// We also use it for symbol table and as parsing cache. 
pub struct SQLSpace<'a> {
    pub bump: &'a Bump,
    tag_slice: RefCell<&'a mut [BVec<'a, u16>]>,
    res_slice: RefCell<&'a mut [BVec<'a, Option<&'a u8>>]>,
}

fn check(x: u64) -> u16 {
    assert!(x != 0, "tag should never be zero");
    if x > u16::MAX as u64 { panic!("oh! your parser is so big that tag(={x}) > {}!", u16::MAX); }
    return x as u16;
}

impl<'a, O, E> Extra<O, E> for SQLSpace<'a> {
    fn mark(&self, progress: usize, tag: u64) {}
    // Safety: allocation is managed by bumpalo (without *external heap allocation*)
    // Therefore, transmuting &u8 <-> &Result is safe as each reference is only used as one &Result type. 
    fn record(&self, progress: usize, tag: u64, result: Result<(usize, &O), (usize, &E)>) {
        self.tag_slice.borrow_mut()[progress].push(check(tag));
        let result = unsafe { std::mem::transmute(self.bump.alloc(result)) };
        self.res_slice.borrow_mut()[progress].push(Some(result));
    }
    fn replay(&self, progress: usize, tag: u64) -> Option<Result<(usize, &O), (usize, &E)>> {
        for (i, x) in self.tag_slice.borrow()[progress].iter().enumerate() {
            if *x != check(tag) { continue }
            let res = self.res_slice.borrow()[progress][i]?;
            let res: &Result<(usize, &O), (usize, &E)> = unsafe { std::mem::transmute(res) };
            return Some(*res);
        }
        return None
    }
    // easy!
    fn out(&self, o: O) -> &O { self.bump.alloc(o) }
    fn err(&self, e: E) -> &E { self.bump.alloc(e) }
}

impl<'a> SQLSpace<'a> {
    pub fn new(bump: &'a Bump, input: &str) -> Self {
        Self {
            bump,
            tag_slice: RefCell::new(bump.alloc_slice_fill_with(input.len()+1, |_| BVec::with_capacity_in(10, bump))),
            res_slice: RefCell::new(bump.alloc_slice_fill_with(input.len()+1, |_|BVec::with_capacity_in(10, bump)))
        }
    }
}
