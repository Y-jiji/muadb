use std::ptr::null_mut;
use std::alloc::*;

/// bytes, but aligned as u64, so it can hold u64
pub struct Bytes(*mut u8);
const HEAD: usize = 16;
const UNIT: usize = 16;
const ALIGN: usize = 16;

impl Bytes {
    pub fn new() -> Self { Bytes(null_mut()) }
    pub fn new_as_usize() -> Self {unsafe { Bytes(null_mut::<u8>().add(1)) }}
    fn initialize(&mut self) {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        if self.0 != null_mut() { return }
        let layout = Layout::from_size_align(HEAD + UNIT, ALIGN).unwrap();
        unsafe {
            let ptr = std::alloc::alloc_zeroed(layout);
            *(ptr as *mut [u64;2]) = [0, UNIT as u64];
            *self = Bytes(ptr);
        }
    }
    pub fn as_u64(&self) -> usize {
        debug_assert!(self.0 as usize % 2 == 1);
        self.0 as usize / 2
    }
    pub fn add(&mut self, x: usize) {
        debug_assert!(self.0 as usize % 2 == 1);
        unsafe {
            self.0 = self.0.add(x * 2);
        }
    }
    pub fn len(&self) -> usize {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        let Bytes(ptr) = self;
        if ptr.is_null() { return 0 }
        unsafe { (*(*ptr as *mut [u64;2]))[0] as usize }
    }
    pub fn capacity(&self) -> usize {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        let Bytes(ptr) = self;
        if ptr.is_null() { return 0 }
        unsafe { (*(*ptr as *mut [u64;2]))[1] as usize }
    }
    pub fn pad(&mut self, align: usize) {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        if self.len() % align == 0 { return }
        let rest = align - self.len() % align;
        let zero = [0; 128];
        self.extend(&zero[..rest]);
    }
    pub fn push(&mut self, byte: u8) {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        self.initialize();
        let len = self.len();
        let cap = self.capacity();
        let Bytes(ptr) = self;
        if len + 1 > cap {unsafe {
            let layout = Layout::from_size_align(cap + HEAD, ALIGN).unwrap();
            *ptr = std::alloc::realloc(*ptr, layout, cap * 2 + HEAD);
            (*(*ptr as *mut [u64;2]))[1] *= 2;
        }}
        unsafe {
            *(*ptr as *mut u8).byte_add(len + HEAD) = byte;
            (*(*ptr as *mut [u64;2]))[0] += 1;
        }
    }
    pub fn pop(&mut self) -> Option<u8> {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        if self.len() == 0 { return None }
        let len = self.len();
        let cap = self.capacity();
        let Bytes(ptr) = self;
        let last = unsafe { *(*ptr as *mut u8).byte_add(len + HEAD - 1) };
        if len * 2 + HEAD < cap {unsafe{
            let layout = Layout::from_size_align(cap + HEAD, ALIGN).unwrap();
            *ptr = std::alloc::realloc(*ptr, layout, cap / 2 + HEAD);
            (*(*ptr as *mut [u64;2]))[1] /= 2;
        }}
        unsafe {
            (*(*ptr as *mut [u64;2]))[0] -= 1;
            Some(last)
        }
    }
    pub fn extend(&mut self, slice: &[u8]) {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        self.initialize();
        let len = self.len();
        let mut cap = self.capacity();
        let Bytes(ptr) = self;
        while len + slice.len() > cap {unsafe {
            let layout = Layout::from_size_align(cap + HEAD, ALIGN).unwrap();
            *ptr = std::alloc::realloc(*ptr, layout, cap * 2 + HEAD);
            cap *= 2;
            (*(*ptr as *mut [u64;2]))[1] = cap as u64;
        }}
        unsafe {
            std::ptr::copy(slice.as_ptr(), ptr.add(len + HEAD), slice.len());
            (*(*ptr as *mut [u64;2]))[0] += slice.len() as u64;
        }
    }
    pub fn filled(len: usize, byte: u8) -> Self {
        let layout = Layout::from_size_align(HEAD + (len + ALIGN - 1) & !(ALIGN - 1), ALIGN).unwrap();
        println!("{layout:?}");
        unsafe {
            let ptr = std::alloc::alloc(layout);
            std::ptr::write_bytes(ptr.add(HEAD), byte, len);
            (*(ptr as *mut [u64;2]))[0] = len as u64;
            (*(ptr as *mut [u64;2]))[1] = ((len + ALIGN - 1) & !(ALIGN - 1)) as u64;
            Bytes(ptr)
        }
    }
    pub fn slice(&self, range: impl std::ops::RangeBounds<usize>) -> &'_ [u8] {
        debug_assert!(self.0 as usize % 2 == 0, "pointer not aligned to 4 means the value cannnot be used as a pointer's address");
        use std::ops::Bound::*;
        let Bytes(ptr) = self;
        if ptr.is_null() { return &[] }
        let start = 16 + match range.start_bound() {
            Included(x) => *x,
            Excluded(x) => *x+1,
            Unbounded => 0,
        };
        let end = 16 + match range.end_bound() {
            Included(x) => *x+1,
            Excluded(x) => *x,
            Unbounded => self.len()
        };
        debug_assert!(end >= start);
        unsafe {
            std::slice::from_raw_parts(ptr.add(start), end - start)
        }
    }
}

impl Drop for Bytes {
    fn drop(&mut self) {
        if self.0 as usize % 2 != 0 { return }
        let cap = self.capacity();
        let Bytes(ptr) = self;
        let Ok(layout) = Layout::from_size_align(cap + HEAD, ALIGN) else {
            panic!("invalid layout SIZE:{} ALIGN:{}", cap + HEAD, ALIGN);
        };
        unsafe { std::alloc::dealloc(*ptr, layout); }
    }
}

#[cfg(test)]
mod test {
    use rand::*;
    use rand_xoshiro::*;
    use super::*;

    #[test]
    fn push_pop_repeat() {
        let mut rng = Xoroshiro128Plus::seed_from_u64(7788);
        let mut x = vec![];
        let mut y = Bytes::new();
        for _j in 0..100 {
            for _i in 0..rng.gen_range(0..4096) {
                assert!(&x[..] == y.slice(..), "{:?} != {:?}", &x[..], y.slice(..));
                let x = x.pop();
                let y = y.pop();
                assert!(x == y, "{x:?} != {y:?}");
            }
            for _i in 0..rng.gen_range(0..4096) {
                let n = rng.gen::<u8>();
                x.push(n);
                y.push(n);
                assert!(&x[..] == y.slice(..), "{:?} != {:?}", &x[..], y.slice(..));
            }
        }
    }

    #[test]
    fn fill_push_pop_repeat() {
        let mut rng = Xoroshiro128Plus::seed_from_u64(7788);
        let mut x = vec![7; 1049];
        let mut y = Bytes::filled(1049, 7u8);
        for _j in 0..100 {
            for _i in 0..rng.gen_range(0..4096) {
                assert!(&x[..] == y.slice(..), "{:?} != {:?}", &x[..], y.slice(..));
                let x = x.pop();
                let y = y.pop();
                assert!(x == y, "{x:?} != {y:?}");
            }
            for _i in 0..rng.gen_range(0..4096) {
                let n = rng.gen::<u8>();
                x.push(n);
                y.push(n);
                assert!(&x[..] == y.slice(..), "{:?} != {:?}", &x[..], y.slice(..));
            }
        }
    }
}