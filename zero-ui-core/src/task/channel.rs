//! Async channels.
//!
//! The channel can work across UI tasks and parallel tasks, it can be [`bounded`] or [`unbounded`] and is MPMC.
//!
//! This module is a thin wrapper around the [`flume`] crate's channel that just limits the API
//! surface to only `async` methods. You can convert from/into that [`flume`] channel.
//!
//! # Examples
//!
//! ```no_run
//! use zero_ui_core::{task::{self, channel}, units::*};
//!
//! let (sender, receiver) = channel::bounded(5);
//!
//! task::spawn(async move {
//!     task::timeout(5.secs()).await;
//!     if let Err(e) = sender.send("Data!").await {
//!         eprintln!("no receiver connected, did not send message: '{}'", e.0)
//!     }
//! });
//! task::spawn(async move {
//!     match receiver.recv().await {
//!         Ok(msg) => println!("{msg}"),
//!         Err(_) => eprintln!("no message in channel and no sender connected")
//!     }
//! });
//! ```
//!
//! [`flume`]: https://docs.rs/flume/0.10.7/flume/

use std::{
    any::type_name,
    convert::TryFrom,
    fmt,
    time::{Duration, Instant},
};

pub use flume::{RecvError, RecvTimeoutError, SendError, SendTimeoutError};

/// The transmitting end of an unbounded channel.
///
/// Use [`unbounded`] to create a channel.
pub struct UnboundSender<T>(flume::Sender<T>);
impl<T> fmt::Debug for UnboundSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnboundSender<{}>", type_name::<T>())
    }
}
impl<T> Clone for UnboundSender<T> {
    fn clone(&self) -> Self {
        UnboundSender(self.0.clone())
    }
}
impl<T> TryFrom<flume::Sender<T>> for UnboundSender<T> {
    type Error = flume::Sender<T>;

    /// Convert to [`UnboundSender`] if the flume sender is unbound.
    fn try_from(value: flume::Sender<T>) -> Result<Self, Self::Error> {
        if value.capacity().is_none() {
            Ok(UnboundSender(value))
        } else {
            Err(value)
        }
    }
}
impl<T> From<UnboundSender<T>> for flume::Sender<T> {
    fn from(s: UnboundSender<T>) -> Self {
        s.0
    }
}
impl<T> UnboundSender<T> {
    /// Send a value into the channel.
    ///
    /// If the messages are not received they accumulate in the channel buffer.
    ///
    /// Returns an error if all receivers have been dropped.
    pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.0.send(msg)
    }

    /// Returns `true` if all receivers for this channel have been dropped.
    #[inline]
    pub fn is_disconnected(&self) -> bool {
        self.0.is_disconnected()
    }

    /// Returns `true` if the channel is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of messages in the channel.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// The transmitting end of a channel.
///
/// Use [`bounded`] or [`rendezvous`] to create a channel. You can also convert an [`UnboundSender`] into this one.
pub struct Sender<T>(flume::Sender<T>);
impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sender<{}>", type_name::<T>())
    }
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender(self.0.clone())
    }
}
impl<T> From<flume::Sender<T>> for Sender<T> {
    fn from(s: flume::Sender<T>) -> Self {
        Sender(s)
    }
}
impl<T> From<Sender<T>> for flume::Sender<T> {
    fn from(s: Sender<T>) -> Self {
        s.0
    }
}
impl<T> Sender<T> {
    /// Send a value into the channel.
    ///
    /// Waits until there is space in the channel buffer.
    ///
    /// Returns an error if all receivers have been dropped.
    #[inline]
    pub async fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.0.send_async(msg).await
    }

    /// Send a value into the channel.
    ///
    /// Waits until there is space in the channel buffer or the `timeout` is reached.
    ///
    /// Returns an error if all receivers have been dropped or the `timeout` is reached.
    pub async fn send_timeout(&self, msg: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
        match super::with_timeout(self.send(msg), timeout).await {
            Ok(r) => match r {
                Ok(_) => Ok(()),
                Err(e) => Err(SendTimeoutError::Disconnected(e.0)),
            },
            Err(_t) => {
                // TODO: wait for https://github.com/zesterer/flume/pull/84
                //
                todo!("wait for send_timeout_async impl")
            }
        }
    }

    /// Send a value into the channel.
    ///
    /// Waits until there is space in the channel buffer or the `deadline` has passed.
    ///
    /// Returns an error if all receivers have been dropped or the `deadline` has passed.
    pub async fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), SendTimeoutError<T>> {
        let now = Instant::now();
        if deadline < now {
            Err(SendTimeoutError::Timeout(msg))
        } else {
            self.send_timeout(msg, deadline - now).await
        }
    }

    /// Returns `true` if all receivers for this channel have been dropped.
    #[inline]
    pub fn is_disconnected(&self) -> bool {
        self.0.is_disconnected()
    }

    /// Returns `true` if the channel is empty.
    ///
    /// Note: [`rendezvous`] channels are always empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the channel is full.
    ///
    /// Note: [`rendezvous`] channels are always full and [`unbounded`] channels are never full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Returns the number of messages in the channel.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If the channel is bounded, returns its capacity.
    #[inline]
    pub fn capacity(&self) -> Option<usize> {
        self.0.capacity()
    }
}

/// The receiving end of a channel.
///
/// Use [`bounded`],[`unbounded`] or [`rendezvous`] to create a channel.
///
/// # Work Stealing
///
/// Cloning the receiver **does not** turn this channel into a broadcast channel.
/// Each message will only be received by a single receiver. You can use this to
/// to implement work stealing.
pub struct Receiver<T>(flume::Receiver<T>);
impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Receiver<{}>", type_name::<T>())
    }
}
impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Receiver(self.0.clone())
    }
}
impl<T> Receiver<T> {
    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped.
    pub async fn recv(&self) -> Result<T, RecvError> {
        self.0.recv_async().await
    }

    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `timeout` is reached.
    pub async fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        match super::with_timeout(self.recv(), timeout).await {
            Ok(r) => match r {
                Ok(m) => Ok(m),
                Err(_) => Err(RecvTimeoutError::Disconnected),
            },
            Err(_) => Err(RecvTimeoutError::Timeout),
        }
    }

    /// Wait for an incoming value from the channel associated with this receiver.
    ///  
    /// Returns an error if all senders have been dropped or the `deadline` has passed.
    pub async fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {
        let now = Instant::now();
        if deadline < now {
            Err(RecvTimeoutError::Timeout)
        } else {
            self.recv_timeout(now - deadline).await
        }
    }

    /// Returns `true` if all senders for this channel have been dropped.
    #[inline]
    pub fn is_disconnected(&self) -> bool {
        self.0.is_disconnected()
    }

    /// Returns `true` if the channel is empty.
    ///
    /// Note: [`rendezvous`] channels are always empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the channel is full.
    ///
    /// Note: [`rendezvous`] channels are always full and [`unbounded`] channels are never full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Returns the number of messages in the channel.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If the channel is bounded, returns its capacity.
    #[inline]
    pub fn capacity(&self) -> Option<usize> {
        self.0.capacity()
    }

    /// Takes all sitting in the channel.
    #[inline]
    pub fn drain(&self) -> flume::Drain<T> {
        self.0.drain()
    }
}

/// Create a channel with no maximum capacity.
///
/// Unbound channels always [`send`] messages immediately, never yielding on await.
/// If the messages are no [received] they accumulate in the channel buffer.
///
/// # Examples
///
/// The example [spawns] two parallel tasks, the receiver task takes a while to start receiving but then
/// rapidly consumes all messages in the buffer and new messages as they are send.
///
/// ```no_run
/// use zero_ui_core::{task::{self, channel}, units::*};
///
/// let (sender, receiver) = channel::unbounded();
///
/// task::spawn(async move {
///     for msg in ["Hello!", "Are you still there?"].into_iter().cycle() {
///         task::timeout(300.ms());
///         if let Err(e) = sender.send(msg) {
///             eprintln!("no receiver connected, the message `{}` was not send", e.0);
///             break;
///         }
///     }
/// });
/// task::spawn(async move {
///     task::timeout(5.secs()).await;
///     
///     loop {
///         match receiver.recv().await {
///             Ok(msg) => println!("{msg}"),
///             Err(_) => {
///                 eprintln!("no message in channel and no sender connected");
///                 break;
///             }
///         }
///     }
/// });
/// ```
///
/// Note that you don't need to `.await` on [`send`] as there is always space in the channel buffer.
///
/// [`send`]: UnboundSender::send
/// [received]: Receiver::recv
/// [spawns]: crate::task::spawn
#[inline]
pub fn unbounded<T>() -> (UnboundSender<T>, Receiver<T>) {
    let (s, r) = flume::unbounded();
    (UnboundSender(s), Receiver(r))
}

/// Create a channel with a maximum capacity.
///
/// Bounded channels [`send`] until the channel reaches its capacity then it awaits until a message
/// is [received] before sending another message.
///
/// # Examples
///
/// The example [spawns] two parallel tasks, the receiver task takes a while to start receiving but then
/// rapidly consumes the 2 messages in the buffer and unblocks the sender to send more messages.
///
/// ```no_run
/// use zero_ui_core::{task::{self, channel}, units::*};
///
/// let (sender, receiver) = channel::bounded(2);
///
/// task::spawn(async move {
///     for msg in ["Hello!", "Data!"].into_iter().cycle() {
///         task::timeout(300.ms());
///         if let Err(e) = sender.send(msg).await {
///             eprintln!("no receiver connected, the message `{}` was not send", e.0);
///             break;
///         }
///     }
/// });
/// task::spawn(async move {
///     task::timeout(5.secs()).await;
///     
///     loop {
///         match receiver.recv().await {
///             Ok(msg) => println!("{msg}"),
///             Err(_) => {
///                 eprintln!("no message in channel and no sender connected");
///                 break;
///             }
///         }
///     }
/// });
/// ```
///
/// [`send`]: UnboundSender::send
/// [received]: Receiver::recv
/// [spawns]: crate::task::spawn
#[inline]
pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (s, r) = flume::bounded(capacity);
    (Sender(s), Receiver(r))
}

/// Create a [`bounded`] channel with `0` capacity.
///
/// Rendezvous channels always awaits until the message is [received] to *return* from [`send`], there is no buffer.
///
/// # Examples
///
/// The example [spawns] two parallel tasks, the sender and receiver *handshake* when transferring the message, the
/// receiver takes 2 seconds to receive, so the sender takes 2 seconds to send.
///
/// ```no_run
/// use zero_ui_core::{task::{self, channel}, units::*};
/// use std::time::*;
///
/// let (sender, receiver) = channel::rendezvous();
///
/// task::spawn(async move {
///     loop {
///         let t = Instant::now();
///
///         if let Err(e) = sender.send("the stuff").await {
///             eprintln!(r#"failed to send "{}", no receiver connected"#, e.0);
///             break;
///         }
///
///         assert!(Instant::now().duration_since(t) >= 2.secs());
///     }
/// });
/// task::spawn(async move {
///     loop {
///         task::timeout(2.secs()).await;
///
///         match receiver.recv().await {
///             Ok(msg) => println!(r#"got "{msg}""#),
///             Err(_) => {
///                 eprintln!("no sender connected");
///                 break;
///             }
///         }
///     }
/// });
/// ```
///
/// [`send`]: UnboundSender::send
/// [received]: Receiver::recv
/// [spawns]: crate::task::spawn
#[inline]
pub fn rendezvous<T>() -> (Sender<T>, Receiver<T>) {
    bounded::<T>(0)
}
