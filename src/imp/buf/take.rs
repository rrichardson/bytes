use {Buf, MutBuf};
use std::{cmp};

#[derive(Debug)]
pub struct Take<T> {
    inner: T,
    limit: usize,
}

impl<T> Take<T> {
    pub fn new(inner: T, limit: usize) -> Take<T> {
        Take {
            inner: inner,
            limit: limit,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn set_limit(&mut self, lim: usize) {
        self.limit = lim
    }
}

impl<T: Buf> Buf for Take<T> {
    fn remaining(&self) -> usize {
        cmp::min(self.inner.remaining(), self.limit)
    }

    fn bytes(&self) -> &[u8] {
        &self.inner.bytes()[..self.limit]
    }

    fn advance(&mut self, cnt: usize) {
        let cnt = cmp::min(cnt, self.limit);
        self.limit -= cnt;
        self.inner.advance(cnt);
    }
}

impl<T: MutBuf> MutBuf for Take<T> {
    fn remaining(&self) -> usize {
        cmp::min(self.inner.remaining(), self.limit)
    }

    unsafe fn mut_bytes(&mut self) -> &mut [u8] {
        &mut self.inner.mut_bytes()[..self.limit]
    }

    unsafe fn advance(&mut self, cnt: usize) {
        let cnt = cmp::min(cnt, self.limit);
        self.limit -= cnt;
        self.inner.advance(cnt);
    }
}
