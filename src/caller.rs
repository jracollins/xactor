use crate::{Actor, Addr, Message, Result};
use std::future::Future;
use std::pin::Pin;
use std::hash::{Hash, Hasher};


pub(crate) type CallerFn<T> = Box<
    dyn Fn(T) -> Pin<Box<dyn Future<Output = Result<<T as Message>::Result>> + Send + 'static>>
        + 'static,
>;

pub(crate) type SenderFn<T> = Box<dyn Fn(T) -> Result<()> + 'static + Send>;

/// Caller of a specific message type
pub struct Caller<T: Message>(pub(crate) CallerFn<T>);

impl<T: Message> Caller<T> {
    pub async fn call(&self, msg: T) -> Result<T::Result> {
        self.0(msg).await
    }
}

/// Sender of a specific message type
// pub struct Sender<T: Message>(pub(crate) SenderFn<T>);

// impl<T: Message<Result = ()>> Sender<T> {
//     pub fn send(&self, msg: T) -> Result<()> {
//         self.0(msg)
//     }
// }

/// Sender of a specific message type
pub struct Sender<T: Message>  {
    pub actor_id: u64,
    pub(crate) sender_fn: SenderFn<T>,
}

impl<T: Message<Result = ()>> Sender<T> {
    pub fn send(&self, msg: T) -> Result<()> {
        (self.sender_fn)(msg)
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



// /// Sender of a specific message type
// pub struct WeakSender<T: Message, A: Actor>  {
//     weak_addr: Addr<A>,
//     actor_id: u64,
//     pub(crate) sender_fn: SenderFn<T>,
// }

// impl<T, A> Clone for WeakSender<T: Message, A: Actor> {
//     fn clone(&self) -> Self {
//         Self {
//             weak_addr: self.weak_addr.clone(),
//             actor_id: self.actor_id.clone(),
//             sender_fn: self.weak_addr.sender(),
//         }
//     }

// }

// impl<A> PartialEq for Addr<A> {
//     fn eq(&self, other: &Self) -> bool {
//         self.actor_id == other.actor_id
//     }
// }

// impl<A> Hash for Addr<A> {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.actor_id.hash(state)
//     }
// }

// impl WeakSender<T: Message, A: Actor> {
//     pub fn new(addr: Addr<A>) -> Self {
//         let weak_tx = Arc::downgrade(&self.tx);

//         WeakSender {

//         }
//     }
// }

// impl<T: Message<Result = ()>> WeakSender<T> {
//     pub fn send(&self, msg: T) -> Result<()> {
//         (self.sender_fn)(msg)
//     }
// }
