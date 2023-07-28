#![doc = include_str!("../../README.md")]

pub use async_trait::async_trait;

#[cfg(feature = "macros")]
pub use brug_macros::performer;

#[async_trait]
pub trait Performer<Command: Send> {
    /// Perform the given command, the command has an embedded callback path
    async fn perform(&mut self, command: Command);
}

#[async_trait]
pub trait Sender<T: Send>: Send {
    async fn send(self, data: T);
}

#[async_trait]
pub trait Receiver<T: Send>: Send {
    async fn receive(self) -> Option<T>;
}

/// Defines a type of transport for return values
/// this might e.g. be the `tokio::sync::oneshot` channel, but any different user implementation can used too
///
/// The indirection (e.g. the actual channel being stored in a child type of a `Transport`) was originally meant
/// to allow the `Command` enum's to contain a generic Sender, however it ended useful in being able to bundle both
/// Sender and Receiver types, and offer a method to create a paired set
pub trait Transport: 'static
{
    type Sender<T: Send>: Sender<T>;
    type Receiver<T: Send>: Receiver<T>;

    /// Create a new paired set of this transport method
    fn pair<T: Send>() -> (Self::Sender<T>, Self::Receiver<T>);
}

#[cfg(feature = "tokio")]
pub mod tokio {
    use async_trait::async_trait;
    use tokio::sync::oneshot;
    use crate::{Receiver, Sender, Transport};

    /// Implements [`Transport`](crate::Transport) for [`tokio::sync::oneshot`](tokio::sync::oneshot)
    pub struct OneShot;

    impl Transport for OneShot {
        type Sender<T: Send> = oneshot::Sender<T>;
        type Receiver<T: Send> = oneshot::Receiver<T>;

        fn pair<T: Send>() -> (Self::Sender<T>, Self::Receiver<T>) {
            oneshot::channel()
        }
    }

    #[async_trait]
    impl<T: Send> Sender<T> for oneshot::Sender<T> {
        async fn send(self, data: T) {
            let _ = oneshot::Sender::send(self, data);
        }
    }

    #[async_trait]
    impl<T: Send> Receiver<T> for oneshot::Receiver<T> {
        async fn receive(self) -> Option<T> {
            self.await.ok()
        }
    }
}