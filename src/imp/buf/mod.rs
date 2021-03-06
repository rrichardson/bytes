pub mod append;
pub mod block;
pub mod slice_buf;
pub mod ring;
pub mod take;

use {Bytes};
use buf::Take;
use byteorder::ByteOrder;
use std::{cmp, fmt, io, ptr, usize};

/// A trait for values that provide sequential read access to bytes.
pub trait Buf {

    /// Returns the number of bytes that can be accessed from the Buf
    fn remaining(&self) -> usize;

    /// Returns a slice starting at the current Buf position and of length
    /// between 0 and `Buf::remaining()`.
    fn bytes(&self) -> &[u8];

    /// Advance the internal cursor of the Buf
    fn advance(&mut self, cnt: usize);

    /// Returns true if there are any more bytes to consume
    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    fn copy_to<S: Sink + ?Sized>(&mut self, dst: &mut S) -> usize
            where Self: Sized {
        let rem = self.remaining();
        dst.sink(self);
        rem - self.remaining()
    }

    /// Read bytes from the `Buf` into the given slice and advance the cursor by
    /// the number of bytes read.
    /// Returns the number of bytes read.
    ///
    /// ```
    /// use std::io::Cursor;
    /// use bytes::Buf;
    ///
    /// let mut buf = Cursor::new(b"hello world");
    /// let mut dst = [0; 5];
    ///
    /// buf.read_slice(&mut dst);
    /// assert_eq!(b"hello", &dst);
    /// assert_eq!(6, buf.remaining());
    /// ```
    fn read_slice(&mut self, dst: &mut [u8]) {
        let mut off = 0;

        assert!(self.remaining() >= dst.len());

        while off < dst.len() {
            let cnt;

            unsafe {
                let src = self.bytes();
                cnt = cmp::min(src.len(), dst.len() - off);

                ptr::copy_nonoverlapping(
                    src.as_ptr(), dst[off..].as_mut_ptr(), cnt);

                off += src.len();
            }

            self.advance(cnt);
        }
    }

    /// Reads an unsigned 8 bit integer from the `Buf` without advancing the
    /// buffer cursor
    fn peek_u8(&self) -> Option<u8> {
        if self.has_remaining() {
            Some(self.bytes()[0])
        } else {
            None
        }
    }

    /// Reads an unsigned 8 bit integer from the `Buf`.
    fn read_u8(&mut self) -> u8 {
        let mut buf = [0; 1];
        self.read_slice(&mut buf);
        buf[0]
    }

    /// Reads a signed 8 bit integer from the `Buf`.
    fn read_i8(&mut self) -> i8 {
        let mut buf = [0; 1];
        self.read_slice(&mut buf);
        buf[0] as i8
    }

    /// Reads an unsigned 16 bit integer from the `Buf`
    fn read_u16<T: ByteOrder>(&mut self) -> u16 {
        let mut buf = [0; 2];
        self.read_slice(&mut buf);
        T::read_u16(&buf)
    }

    /// Reads a signed 16 bit integer from the `Buf`
    fn read_i16<T: ByteOrder>(&mut self) -> i16 {
        let mut buf = [0; 2];
        self.read_slice(&mut buf);
        T::read_i16(&buf)
    }

    /// Reads an unsigned 32 bit integer from the `Buf`
    fn read_u32<T: ByteOrder>(&mut self) -> u32 {
        let mut buf = [0; 4];
        self.read_slice(&mut buf);
        T::read_u32(&buf)
    }

    /// Reads a signed 32 bit integer from the `Buf`
    fn read_i32<T: ByteOrder>(&mut self) -> i32 {
        let mut buf = [0; 4];
        self.read_slice(&mut buf);
        T::read_i32(&buf)
    }

    /// Reads an unsigned 64 bit integer from the `Buf`
    fn read_u64<T: ByteOrder>(&mut self) -> u64 {
        let mut buf = [0; 8];
        self.read_slice(&mut buf);
        T::read_u64(&buf)
    }

    /// Reads a signed 64 bit integer from the `Buf`
    fn read_i64<T: ByteOrder>(&mut self) -> i64 {
        let mut buf = [0; 8];
        self.read_slice(&mut buf);
        T::read_i64(&buf)
    }

    /// Reads an unsigned n-bytes integer from the `Buf`
    fn read_uint<T: ByteOrder>(&mut self, nbytes: usize) -> u64 {
        let mut buf = [0; 8];
        self.read_slice(&mut buf[..nbytes]);
        T::read_uint(&buf[..nbytes], nbytes)
    }

    /// Reads a signed n-bytes integer from the `Buf`
    fn read_int<T: ByteOrder>(&mut self, nbytes: usize) -> i64 {
        let mut buf = [0; 8];
        self.read_slice(&mut buf[..nbytes]);
        T::read_int(&buf[..nbytes], nbytes)
    }

    /// Reads a IEEE754 single-precision (4 bytes) floating point number from
    /// the `Buf`
    fn read_f32<T: ByteOrder>(&mut self) -> f32 {
        let mut buf = [0; 4];
        self.read_slice(&mut buf);
        T::read_f32(&buf)
    }

    /// Reads a IEEE754 double-precision (8 bytes) floating point number from
    /// the `Buf`
    fn read_f64<T: ByteOrder>(&mut self) -> f64 {
        let mut buf = [0; 8];
        self.read_slice(&mut buf);
        T::read_f64(&buf)
    }

    /// Creates a "by reference" adaptor for this instance of Buf
    fn by_ref(&mut self) -> &mut Self where Self: Sized {
        self
    }

    /// Create an adapter which will limit at most `limit` bytes from it.
    fn take(self, limit: usize) -> Take<Self> where Self: Sized {
        Take::new(self, limit)
    }

    /// Return a `Reader` for the value. Allows using a `Buf` as an `io::Read`
    fn reader(self) -> Reader<Self> where Self: Sized {
        Reader::new(self)
    }
}

/// A trait for values that provide sequential write access to bytes.
pub trait MutBuf {

    /// Returns the number of bytes that can be written to the MutBuf
    fn remaining(&self) -> usize;

    /// Advance the internal cursor of the MutBuf
    unsafe fn advance(&mut self, cnt: usize);

    /// Returns true iff there is any more space for bytes to be written
    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    /// Returns a mutable slice starting at the current MutBuf position and of
    /// length between 0 and `MutBuf::remaining()`.
    ///
    /// The returned byte slice may represent uninitialized memory.
    unsafe fn mut_bytes(&mut self) -> &mut [u8];

    fn copy_from<S: Source>(&mut self, src: S) -> usize
            where Self: Sized {
        let rem = self.remaining();
        src.source(self);
        rem - self.remaining()
    }

    /// Write bytes from the given slice into the `MutBuf` and advance the
    /// cursor by the number of bytes written.
    /// Returns the number of bytes written.
    ///
    /// ```
    /// use bytes::MutBuf;
    /// use std::io::Cursor;
    ///
    /// let mut dst = [0; 6];
    ///
    /// {
    ///     let mut buf = Cursor::new(&mut dst);
    ///     buf.write_slice(b"hello");
    ///
    ///     assert_eq!(1, buf.remaining());
    /// }
    ///
    /// assert_eq!(b"hello\0", &dst);
    /// ```
    fn write_slice(&mut self, src: &[u8]) {
        let mut off = 0;

        assert!(self.remaining() >= src.len(), "buffer overflow");

        while off < src.len() {
            let cnt;

            unsafe {
                let dst = self.mut_bytes();
                cnt = cmp::min(dst.len(), src.len() - off);

                ptr::copy_nonoverlapping(
                    src[off..].as_ptr(),
                    dst.as_mut_ptr(),
                    cnt);

                off += cnt;

            }

            unsafe { self.advance(cnt); }
        }
    }

    fn write_str(&mut self, src: &str) {
        self.write_slice(src.as_bytes());
    }

    /// Writes an unsigned 8 bit integer to the MutBuf.
    fn write_u8(&mut self, n: u8) {
        self.write_slice(&[n])
    }

    /// Writes a signed 8 bit integer to the MutBuf.
    fn write_i8(&mut self, n: i8) {
        self.write_slice(&[n as u8])
    }

    /// Writes an unsigned 16 bit integer to the MutBuf.
    fn write_u16<T: ByteOrder>(&mut self, n: u16) {
        let mut buf = [0; 2];
        T::write_u16(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes a signed 16 bit integer to the MutBuf.
    fn write_i16<T: ByteOrder>(&mut self, n: i16) {
        let mut buf = [0; 2];
        T::write_i16(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes an unsigned 32 bit integer to the MutBuf.
    fn write_u32<T: ByteOrder>(&mut self, n: u32) {
        let mut buf = [0; 4];
        T::write_u32(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes a signed 32 bit integer to the MutBuf.
    fn write_i32<T: ByteOrder>(&mut self, n: i32) {
        let mut buf = [0; 4];
        T::write_i32(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes an unsigned 64 bit integer to the MutBuf.
    fn write_u64<T: ByteOrder>(&mut self, n: u64) {
        let mut buf = [0; 8];
        T::write_u64(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes a signed 64 bit integer to the MutBuf.
    fn write_i64<T: ByteOrder>(&mut self, n: i64) {
        let mut buf = [0; 8];
        T::write_i64(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes an unsigned n-bytes integer to the MutBuf.
    ///
    /// If the given integer is not representable in the given number of bytes,
    /// this method panics. If `nbytes > 8`, this method panics.
    fn write_uint<T: ByteOrder>(&mut self, n: u64, nbytes: usize) {
        let mut buf = [0; 8];
        T::write_uint(&mut buf, n, nbytes);
        self.write_slice(&buf[0..nbytes])
    }

    /// Writes a signed n-bytes integer to the MutBuf.
    ///
    /// If the given integer is not representable in the given number of bytes,
    /// this method panics. If `nbytes > 8`, this method panics.
    fn write_int<T: ByteOrder>(&mut self, n: i64, nbytes: usize) {
        let mut buf = [0; 8];
        T::write_int(&mut buf, n, nbytes);
        self.write_slice(&buf[0..nbytes])
    }

    /// Writes a IEEE754 single-precision (4 bytes) floating point number to
    /// the MutBuf.
    fn write_f32<T: ByteOrder>(&mut self, n: f32) {
        let mut buf = [0; 4];
        T::write_f32(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Writes a IEEE754 double-precision (8 bytes) floating point number to
    /// the MutBuf.
    fn write_f64<T: ByteOrder>(&mut self, n: f64) {
        let mut buf = [0; 8];
        T::write_f64(&mut buf, n);
        self.write_slice(&buf)
    }

    /// Creates a "by reference" adaptor for this instance of MutBuf
    fn by_ref(&mut self) -> &mut Self where Self: Sized {
        self
    }

    /// Create an adapter which will limit at most `limit` bytes from it.
    fn take(self, limit: usize) -> Take<Self> where Self: Sized {
        Take::new(self, limit)
    }

    /// Return a `Write` for the value. Allows using a `MutBuf` as an
    /// `io::Write`
    fn writer(self) -> Writer<Self> where Self: Sized {
        Writer::new(self)
    }
}

/*
 *
 * ===== IntoBuf =====
 *
 */

/// Conversion into a `Buf`
///
/// Usually, `IntoBuf` is implemented on references of types and not directly
/// on the types themselves. For example, `IntoBuf` is implemented for `&'a
/// Vec<u8>` and not `Vec<u8>` directly.
pub trait IntoBuf {
    type Buf: Buf;

    fn into_buf(self) -> Self::Buf;
}

impl<'a> IntoBuf for &'a [u8] {
    type Buf = io::Cursor<&'a [u8]>;

    /// Creates a buffer from a value
    fn into_buf(self) -> Self::Buf {
        io::Cursor::new(self)
    }
}

// Kind of annoying...
impl<'a> IntoBuf for &'a &'static [u8] {
    type Buf = io::Cursor<&'static [u8]>;

    /// Creates a buffer from a value
    fn into_buf(self) -> Self::Buf {
        io::Cursor::new(self)
    }
}

impl IntoBuf for Vec<u8> {
    type Buf = io::Cursor<Vec<u8>>;

    fn into_buf(self) -> Self::Buf {
        io::Cursor::new(self)
    }
}

impl<'a> IntoBuf for &'a Vec<u8> {
    type Buf = io::Cursor<&'a [u8]>;

    fn into_buf(self) -> Self::Buf {
        io::Cursor::new(&self[..])
    }
}

impl IntoBuf for () {
    type Buf = io::Cursor<&'static [u8]>;

    fn into_buf(self) -> Self::Buf {
        io::Cursor::new(&[])
    }
}

impl<'a> IntoBuf for &'a () {
    type Buf = io::Cursor<&'static [u8]>;

    fn into_buf(self) -> Self::Buf {
        io::Cursor::new(&[])
    }
}

/*
 *
 * ===== Sink / Source =====
 *
 */


/// A value that writes bytes from itself into a `MutBuf`.
pub trait Source {
    /// Copy data from self into destination buffer
    fn source<B: MutBuf>(self, buf: &mut B);
}

impl<'a> Source for &'a [u8] {
    fn source<B: MutBuf>(self, buf: &mut B) {
        buf.write_slice(self);
    }
}

impl Source for u8 {
    fn source<B: MutBuf>(self, buf: &mut B) {
        let src = [self];
        buf.write_slice(&src);
    }
}

impl Source for Bytes {
    fn source<B: MutBuf>(self, buf: &mut B) {
        Source::source(&self, buf);
    }
}

impl<'a> Source for &'a Bytes {
    fn source<B: MutBuf>(self, buf: &mut B) {
        Source::source(&mut self.buf(), buf);
    }
}

impl<'a, T: Buf> Source for &'a mut T {
    fn source<B: MutBuf>(mut self, buf: &mut B) {
        while self.has_remaining() && buf.has_remaining() {
            let l;

            unsafe {
                let s = self.bytes();
                let d = buf.mut_bytes();
                l = cmp::min(s.len(), d.len());

                ptr::copy_nonoverlapping(
                    s.as_ptr(),
                    d.as_mut_ptr(),
                    l);
            }

            self.advance(l);
            unsafe { buf.advance(l); }
        }
    }
}

pub trait Sink {
    fn sink<B: Buf>(&mut self, buf: &mut B);
}

impl Sink for [u8] {
    fn sink<B: Buf>(&mut self, buf: &mut B) {
        buf.read_slice(self);
    }
}

impl<T: MutBuf> Sink for T {
    fn sink<B: Buf>(&mut self, buf: &mut B) {
        Source::source(buf, self)
    }
}

/*
 *
 * ===== Read / Write =====
 *
 */

/// Adapts a `Buf` to the `io::Read` trait
pub struct Reader<B> {
    buf: B,
}

impl<B: Buf> Reader<B> {
    /// Return a `Reader` for the given `buf`
    pub fn new(buf: B) -> Reader<B> {
        Reader { buf: buf }
    }

    /// Gets a reference to the underlying buf.
    pub fn get_ref(&self) -> &B {
        &self.buf
    }

    /// Gets a mutable reference to the underlying buf.
    pub fn get_mut(&mut self) -> &mut B {
        &mut self.buf
    }

    /// Unwraps this `Reader`, returning the underlying `Buf`
    pub fn into_inner(self) -> B {
        self.buf
    }
}

impl<B: Buf + Sized> io::Read for Reader<B> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(self.buf.remaining(), dst.len());

        Buf::copy_to(&mut self.buf, &mut dst[0..len]);
        Ok(len)
    }
}

/// Buffer related extension for `io::Read`
pub trait ReadExt {
    fn read_buf<B: MutBuf>(&mut self, buf: &mut B) -> io::Result<usize>;
}

impl<T: io::Read> ReadExt for T {
    fn read_buf<B: MutBuf>(&mut self, buf: &mut B) -> io::Result<usize> {
        if !buf.has_remaining() {
            return Ok(0);
        }

        unsafe {
            let i = try!(self.read(buf.mut_bytes()));

            buf.advance(i);
            Ok(i)
        }
    }
}

/// Adapts a `MutBuf` to the `io::Write` trait
pub struct Writer<B> {
    buf: B,
}

impl<B: MutBuf> Writer<B> {
    /// Return a `Writer` for teh given `buf`
    pub fn new(buf: B) -> Writer<B> {
        Writer { buf: buf }
    }

    /// Gets a reference to the underlying buf.
    pub fn get_ref(&self) -> &B {
        &self.buf
    }

    /// Gets a mutable reference to the underlying buf.
    pub fn get_mut(&mut self) -> &mut B {
        &mut self.buf
    }

    /// Unwraps this `Writer`, returning the underlying `MutBuf`
    pub fn into_inner(self) -> B {
        self.buf
    }
}

impl<B: MutBuf + Sized> io::Write for Writer<B> {
    fn write(&mut self, src: &[u8]) -> io::Result<usize> {
        let n = cmp::min(self.buf.remaining(), src.len());

        self.buf.copy_from(&src[0..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Buffer related extension for `io::Write`
pub trait WriteExt {
    fn write_buf<B: Buf>(&mut self, buf: &mut B) -> io::Result<usize>;
}

impl<T: io::Write> WriteExt for T {
    fn write_buf<B: Buf>(&mut self, buf: &mut B) -> io::Result<usize> {
        if !buf.has_remaining() {
            return Ok(0);
        }

        let i = try!(self.write(buf.bytes()));
        buf.advance(i);
        Ok(i)
    }
}

/*
 *
 * ===== Buf impls =====
 *
 */

impl<T: AsRef<[u8]>> Buf for io::Cursor<T> {
    fn remaining(&self) -> usize {
        self.get_ref().as_ref().len() - self.position() as usize
    }

    fn bytes(&self) -> &[u8] {
        let pos = self.position() as usize;
        &(self.get_ref().as_ref())[pos..]
    }

    fn advance(&mut self, cnt: usize) {
        let pos = self.position() as usize;
        let pos = cmp::min(self.get_ref().as_ref().len(), pos + cnt);
        self.set_position(pos as u64);
    }
}

impl<T: AsMut<[u8]> + AsRef<[u8]>> MutBuf for io::Cursor<T> {

    fn remaining(&self) -> usize {
        self.get_ref().as_ref().len() - self.position() as usize
    }

    /// Advance the internal cursor of the MutBuf
    unsafe fn advance(&mut self, cnt: usize) {
        let pos = self.position() as usize;
        let pos = cmp::min(self.get_mut().as_mut().len(), pos + cnt);
        self.set_position(pos as u64);
    }

    /// Returns a mutable slice starting at the current MutBuf position and of
    /// length between 0 and `MutBuf::remaining()`.
    ///
    /// The returned byte slice may represent uninitialized memory.
    unsafe fn mut_bytes(&mut self) -> &mut [u8] {
        let pos = self.position() as usize;
        &mut (self.get_mut().as_mut())[pos..]
    }
}

impl MutBuf for Vec<u8> {
    fn remaining(&self) -> usize {
        usize::MAX - self.len()
    }

    unsafe fn advance(&mut self, cnt: usize) {
        let len = self.len() + cnt;

        if len > self.capacity() {
            // Reserve additional
            // TODO: Should this case panic?
            let cap = self.capacity();
            self.reserve(cap - len);
        }

        self.set_len(len);
    }

    unsafe fn mut_bytes(&mut self) -> &mut [u8] {
        use std::slice;

        if self.capacity() == self.len() {
            self.reserve(64); // Grow the vec
        }

        let cap = self.capacity();
        let len = self.len();

        let ptr = self.as_mut_ptr();
        &mut slice::from_raw_parts_mut(ptr, cap)[len..]
    }
}

/*
 *
 * ===== fmt impls =====
 *
 */

pub struct Fmt<'a, B: 'a>(pub &'a mut B);

impl<'a, B: MutBuf> fmt::Write for Fmt<'a, B> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s);
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        fmt::write(self, args)
    }
}
