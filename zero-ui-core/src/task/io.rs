//! IO tasks.

use std::{
    fmt,
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
    time::{Duration, Instant},
};

use crate::{task::AggegateWaker, units::*};

#[doc(no_inline)]
pub use futures_lite::io::*;
use parking_lot::Mutex;

/// Measure read/write of an async task.
///
/// Metrics are updated after each read/write, if you read/write all bytes in one call
/// the metrics will only update once.
pub struct Measure<T> {
    task: T,
    metrics: Metrics,
    start_time: Instant,
    last_write: Instant,
    last_read: Instant,
}
impl<T> Measure<T> {
    /// Start measuring a new read/write task.
    pub fn start(task: T, total_read: impl Into<ByteLength>, total_write: impl Into<ByteLength>) -> Self {
        Self::resume(task, (0, total_read), (0, total_write))
    }

    /// Continue measuring a read/write task.
    pub fn resume(
        task: T,
        read_progress: (impl Into<ByteLength>, impl Into<ByteLength>),
        write_progress: (impl Into<ByteLength>, impl Into<ByteLength>),
    ) -> Self {
        let now = Instant::now();
        Measure {
            task,
            metrics: Metrics {
                read_progress: (read_progress.0.into(), read_progress.1.into()),
                read_speed: 0.bytes(),
                write_progress: (write_progress.0.into(), write_progress.1.into()),
                write_speed: 0.bytes(),
                total_time: Duration::ZERO,
            },
            start_time: now,
            last_write: now,
            last_read: now,
        }
    }

    /// Current metrics.
    ///
    /// This value is updated after every read/write.
    pub fn metrics(&mut self) -> &Metrics {
        &self.metrics
    }

    /// Unwrap the inner task and final metrics.
    pub fn finish(mut self) -> (T, Metrics) {
        self.metrics.total_time = self.start_time.elapsed();
        (self.task, self.metrics)
    }
}

fn bytes_per_sec(bytes: ByteLength, elapsed: Duration) -> ByteLength {
    let bytes_per_sec = bytes.0 as u128 / elapsed.as_nanos() / Duration::from_secs(1).as_nanos();
    ByteLength(bytes_per_sec as usize)
}

impl<T: AsyncRead + Unpin> AsyncRead for Measure<T> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let self_ = self.get_mut();
        match Pin::new(&mut self_.task).poll_read(cx, buf) {
            Poll::Ready(Ok(bytes)) => {
                if bytes > 0 {
                    let bytes = bytes.bytes();
                    self_.metrics.read_progress.0 += bytes;

                    let now = Instant::now();
                    let elapsed = now - self_.last_read;

                    self_.last_read = now;
                    self_.metrics.read_speed = bytes_per_sec(bytes, elapsed);

                    self_.metrics.total_time = now - self_.start_time;
                }
                Poll::Ready(Ok(bytes))
            }
            p => p,
        }
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for Measure<T> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let self_ = self.get_mut();
        match Pin::new(&mut self_.task).poll_write(cx, buf) {
            Poll::Ready(Ok(bytes)) => {
                if bytes > 0 {
                    let bytes = bytes.bytes();
                    self_.metrics.write_progress.0 += bytes;

                    let now = Instant::now();
                    let elapsed = now - self_.last_write;

                    self_.last_write = now;
                    self_.metrics.write_speed = bytes_per_sec(bytes, elapsed);

                    self_.metrics.total_time = now - self_.start_time;
                }
                Poll::Ready(Ok(bytes))
            }
            p => p,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().task).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().task).poll_close(cx)
    }
}

/// Information about the state of an async IO task.
///
/// Use [`Measure`] to measure a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrics {
    /// Number of bytes read / estimated total.
    pub read_progress: (ByteLength, ByteLength),

    /// Average read speed in bytes/second.
    pub read_speed: ByteLength,

    /// Number of bytes written / estimated total.
    pub write_progress: (ByteLength, ByteLength),

    /// Average write speed in bytes/second.
    pub write_speed: ByteLength,

    /// Total time for the entire task. This will continuously increase until
    /// the task is finished.
    pub total_time: Duration,
}
impl Metrics {
    /// All zeros.
    pub fn zero() -> Self {
        Self {
            read_progress: (0.bytes(), 0.bytes()),
            read_speed: 0.bytes(),
            write_progress: (0.bytes(), 0.bytes()),
            write_speed: 0.bytes(),
            total_time: Duration::ZERO,
        }
    }
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut w = false;
        if self.read_progress.0 > 0.bytes() {
            w = true;
            if self.read_progress.0 != self.read_progress.1 {
                write!(
                    f,
                    "read: {} of {}, {}/s",
                    self.read_progress.0, self.read_progress.1, self.read_speed
                )?;
                w = true;
            } else {
                write!(f, "read {} in {:?}", self.read_progress.0, self.total_time)?;
            }
        }
        if self.write_progress.0 > 0.bytes() {
            if w {
                writeln!(f)?;
            }
            if self.write_progress.0 != self.write_progress.1 {
                write!(
                    f,
                    "write: {} of {}, {}/s",
                    self.write_progress.0, self.write_progress.1, self.write_speed
                )?;
            } else {
                write!(f, "written {} in {:?}", self.read_progress.0, self.total_time)?;
            }
        }

        Ok(())
    }
}

/// Cloneable buffered read.
///
/// Already read bytes stay in the buffer until all clones have read it, clones created after reading started
/// continue reading from the same offset as the reader they cloned but can also diverge on their own.
///
/// A single instance of this reader behaves like the default [`BufReader`].
///
/// # Fused Result
///
/// The result is *fused* when `EOF` or an [`Error`] occurs, unfortunately the IO error is not cloneable
/// so an error of the same [`ErrorKind`] is generated for the other reader clones and subsequent poll attempts.
///
/// The inner reader is dropped as soon as it finishes.
pub struct SharedBufReader<S: AsyncRead> {
    inner: Arc<Mutex<SharedBufReaderInner<S>>>,
    index: usize,
}
struct SharedBufReaderInner<S: AsyncRead> {
    source: Option<BufReader<S>>,
    waker: AggegateWaker,

    buf: Vec<u8>,
    len: usize, // buf.len without pending zeros.

    clones: Vec<Option<usize>>,

    result: FusedReadResult,
}
impl<S: AsyncRead> SharedBufReader<S> {
    /// Creates a buffered reader.
    pub fn new(source: S) -> Self {
        Self::from_reader(BufReader::new(source))
    }

    /// Convert the `reader` to a shareable reader.
    pub fn from_reader(reader: BufReader<S>) -> Self {
        SharedBufReader {
            inner: Arc::new(Mutex::new(SharedBufReaderInner {
                source: Some(reader),
                waker: AggegateWaker::empty(),

                buf: vec![],
                len: 0,

                clones: vec![Some(0)],

                result: FusedReadResult::Pending,
            })),
            index: 0,
        }
    }
}
impl<S: AsyncRead> Clone for SharedBufReader<S> {
    fn clone(&self) -> Self {
        let mut inner = self.inner.lock();

        if matches!(&inner.result, FusedReadResult::Pending) {
            let offset = inner.clones[self.index];
            let index = inner.clones.len();
            inner.clones.push(offset);
            Self {
                inner: self.inner.clone(),
                index,
            }
        } else {
            // already finished
            let index = inner.clones.len();
            inner.clones.push(None);
            Self {
                inner: self.inner.clone(),
                index,
            }
        }
    }
}
impl<S: AsyncRead> Drop for SharedBufReader<S> {
    fn drop(&mut self) {
        self.inner.lock().clones[self.index] = None;
    }
}
impl<S: AsyncRead> AsyncRead for SharedBufReader<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let self_ = self.as_ref();
        let mut inner = self_.inner.lock();
        let inner = &mut *inner;

        match &inner.result {
            FusedReadResult::Pending => {
                // normal execution, continue bellow.
            }
            FusedReadResult::Eof => {
                // inner reader has finished, but we may have pending data for `self`.
                if let Some(i) = inner.clones[self_.index] {
                    // data already read
                    let done = &inner.buf[i..inner.len];
                    let min = done.len().min(buf.len());

                    buf[..min].copy_from_slice(&done[..min]);

                    if done.len() <= buf.len() {
                        // fuse clone.
                        inner.clones[self_.index] = None;
                    } else {
                        // still did not request everything.
                        inner.clones[self_.index] = Some(i + min);
                    }
                    return Poll::Ready(Ok(min));
                } else {
                    // already finished this clone too.
                    return Poll::Ready(Ok(0));
                }
            }
            FusedReadResult::Err(e) => {
                // inner reader error, just return an "error clone".
                return Poll::Ready(e.err());
            }
        }

        if inner.buf.len() > 8.kilobytes().0 {
            // cleanup
            let used = inner.clones.iter().filter_map(|c| *c).min().unwrap();
            if used > 4.kilobytes().0 {
                inner.buf.copy_within(used.., 0);
                inner.buf.truncate(inner.buf.len() - used);

                for c in inner.clones.iter_mut().flatten() {
                    *c -= used;
                }
            }
        }

        // data already read
        let i = inner.clones[self_.index].unwrap();
        let done = &inner.buf[i..inner.len];

        // copy already read
        let min = done.len().min(buf.len());
        buf[..min].copy_from_slice(&done[..min]);

        if inner.waker.push(cx.waker().clone()) == 1 {
            // no pending request, read more data, even if we already fulfilled the request.
            let more = (buf.len() - min) + 1;

            let new_start = inner.len;
            let new_len = inner.len + more;
            inner.buf.resize(new_len, 0);
            inner.len = new_len;

            let waker = inner.waker.waker().unwrap();
            let mut cx = task::Context::from_waker(&waker);

            let source = inner.source.as_mut().unwrap();

            // SAFETY: we never move `source`.
            match unsafe { Pin::new_unchecked(source) }.poll_read(&mut cx, &mut inner.buf[new_start..]) {
                Poll::Ready(Ok(0)) => {
                    // finished EOF, return `done`
                    inner.result = FusedReadResult::Eof;
                    inner.clones[self_.index] = None;
                    inner.source = None;

                    return Poll::Ready(Ok(min));
                }
                Poll::Ready(Ok(l)) => {
                    inner.len += l;

                    // add more data if needed.
                    let rest = buf.len() - min;
                    let rest_min = rest.min(l);
                    if rest_min > 0 {
                        buf[min..min + rest_min].copy_from_slice(&inner.buf[new_start..new_start + rest_min]);
                    }
                    inner.clones[self_.index] = Some(new_start + rest_min);

                    return Poll::Ready(Ok(rest_min));
                }
                Poll::Ready(Err(e)) => {
                    // finished in error, fuse everything.
                    inner.result = FusedReadResult::Err(CloneableError::new(&e));
                    inner.buf = vec![];
                    inner.len = 0;
                    inner.source = None;

                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    // could not read anything else, but registered the waker.
                    // continue bellow..
                }
            }
        } else {
            // another clone already requested more data.
            // continue bellow..
        }

        // return what we have for now.
        if min == 0 {
            Poll::Pending
        } else {
            inner.clones[self_.index] = Some(i + min);
            Poll::Ready(Ok(min))
        }
    }
}

/// Duplicate `source` into two async buffered readers, if a reader advances more
/// then the other the difference is cached.
///
/// See [`SharedBufReader`] for more details.
pub fn duplicate_read<S: AsyncRead>(source: S) -> (SharedBufReader<S>, SharedBufReader<S>) {
    let a = SharedBufReader::new(source);
    let b = a.clone();
    (a, b)
}

/// Represents the cloneable parts of an [`Error`].
///
/// Unfortunately [`Error`] does not implement clone, this is needed to implemented
/// *fused* IO futures, where an error may be returned more than one time. This type partially
/// works around the issue by copying enough information to recreate an error that is still useful.
///
/// The OS error code, [`ErrorKind`] and display message are preserved. Note that this not an error type,
/// it must be converted to [`Error`] using `into` or [`err`].
///
/// [`err`]: Self::err
#[derive(Clone)]
pub struct CloneableError {
    info: ErrorInfo,
}
#[derive(Clone)]
enum ErrorInfo {
    OsError(i32),
    Other(ErrorKind, String),
}
impl CloneableError {
    /// Copy the cloneable information from the [`Error`].
    pub fn new(e: &Error) -> Self {
        let info = if let Some(code) = e.raw_os_error() {
            ErrorInfo::OsError(code)
        } else {
            ErrorInfo::Other(e.kind(), format!("{}", e))
        };

        Self { info }
    }

    /// Returns an `Err(Error)` generated from the cloneable information.
    pub fn err<T>(&self) -> Result<T> {
        Err(self.clone().into())
    }
}
impl From<CloneableError> for Error {
    fn from(e: CloneableError) -> Self {
        match e.info {
            ErrorInfo::OsError(code) => Error::from_raw_os_error(code),
            ErrorInfo::Other(kind, msg) => Error::new(kind, msg),
        }
    }
}

enum FusedReadResult {
    Pending,
    Eof,
    Err(CloneableError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task;

    #[test]
    pub fn shared_buf_read() {
        let data = Data::new(60.kilobytes().0);

        let mut expected = vec![0; data.len];
        data.clone().blocking_read(&mut expected[..]);
    
        let (mut a, mut b) = duplicate_read(data);

        let (a, b) = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let b = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                b.read_to_end(&mut buf).await.unwrap();
                buf
            });

            task::all!(a, b).await
        });

        crate::assert_vec_eq!(expected, a);
        crate::assert_vec_eq!(expected, b);
    }

    #[test]
    pub fn shared_buf_read_not_shared() {
        let data = Data::new(60.kilobytes().0);

        let mut expected = vec![0; data.len];
        data.clone().blocking_read(&mut expected[..]);
    
        let mut a = SharedBufReader::new(data);

        let a = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap();
                buf
            });           

            a.await
        });

        crate::assert_vec_eq!(expected, a);
    }

    #[derive(Clone)]
    struct Data {
        b: u8,
        len: usize,
    }
    impl Data {
        pub fn new(len: usize) -> Self {
            Self { b: 0, len }
        }
        pub fn blocking_read(&mut self, buf: &mut [u8]) -> usize {
            let len = self.len;
            for b in buf.iter_mut().take(len) {
                *b = self.b;
                self.b = self.b.wrapping_add(1);
            }
            buf.len().min(len)
        }
    }
    impl AsyncRead for Data {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
            let _ = cx;

            let l = self.as_mut().blocking_read(buf);

            Poll::Ready(Ok(l))
        }
    }

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: std::future::Future,
    {
        task::block_on(task::with_timeout(test, 5.secs())).unwrap()
    }

    #[macro_export]
    macro_rules! assert_vec_eq {
        ($a:expr, $b: expr) => {
            match (&$a, &$b) {
                (ref a, ref b) => {
                    let len_not_eq = a.len() != b.len();
                    let mut data_not_eq = None;
                    for (i, (a, b)) in a.iter().zip(b.iter()).enumerate() {
                        if a != b {
                            data_not_eq = Some(i);
                            break;
                        }
                    }

                    if len_not_eq || data_not_eq.is_some() {
                        use std::fmt::*;

                        let mut error = format!("`{}` != `{}`", stringify!($a), stringify!($b));
                        if len_not_eq {
                            let _ = write!(&mut error, "\n  lengths not equal: {} != {}", a.len(), b.len());    
                        }
                        if let Some(i) = data_not_eq {
                            let _ = write!(&mut error, "\n  data not equal at index {}: {} != {:?}", i, a[i], b[i]);
                        }
                        panic!("{}", error)
                    }                    
                }
            }
        }
    }
}
