use crate::{Message, Result};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;

pub(crate) type CallerFn<T> = Box<
    dyn Fn(T) -> Pin<Box<dyn Future<Output = Result<<T as Message>::Result>> + Send + 'static>>
        + Send
        + 'static,
>;

/// Caller of a specific message type
///
/// Like `Sender<T>`, Caller has a weak reference to the recipient of the message type, and so will not prevent an actor from stopping if all Addr's have been dropped elsewhere.

pub struct Caller<T: Message> {
    pub actor_id: u64,
    pub(crate) caller_fn: CallerFn<T>,
}

impl<T: Message> Caller<T> {
    pub async fn call(&self, msg: T) -> Result<T::Result> {
        (self.caller_fn)(msg).await
    }
}

impl<T: Message<Result = ()>> PartialEq for Caller<T> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<T: Message<Result = ()>> Hash for Caller<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

// impl<T: Message> Clone for Caller<T> {
//     fn clone(&self) -> Caller<T> {
//         self.clone()
//     }
// }

/// Sender of a specific message type
///
/// Like `Caller<T>`, Sender has a weak reference to the recipient of the message type, and so will not prevent an actor from stopping if all Addr's have been dropped elsewhere.
/// This allows it to be used in `send_later` `send_interval` actor functions, and not keep the actor alive indefinitely even after all references to it have been dropped (unless `ctx.stop()` is called from within)

pub struct Sender<T: Message> {
    pub actor_id: u64,
    pub(crate) sender_fn: SenderFn<T>,
}

impl<T: Message<Result = ()>> Sender<T> {
    pub fn send(&self, msg: T) -> Result<()> {
        let func = self.sender_fn;
        let func2 = func.clone();
        func(msg)
        // (self.sender_fn)(msg)
        // Ok(())
    }
}

impl<T: Message<Result = ()>> PartialEq for Sender<T> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<T: Message<Result = ()>> Hash for Sender<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

impl<T: Message<Result = ()>> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender {
            actor_id: self.actor_id.clone(),
            sender_fn: self.sender_fn.clone(),
        }
    }
}

// SENDER FN

type SenderFn<T: Message<Result = ()>> = Box<dyn SenderClosure<T>>;

impl<T: Message<Result = ()>> Clone for SenderFn<T> {
    fn clone(&self) -> SenderFn<T> {
        *dyn_clone::clone_box(&*self)
    }
}

use dyn_clone::DynClone;
pub trait SenderClosure<T>: DynClone + 'static + Send {}

impl<T, F> SenderClosure<T> for F where F: Fn(T) -> Result<()> + 'static + Send + Clone {}
