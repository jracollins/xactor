use crate::{Actor, Caller, Context, Error, Handler, Message, Result, Sender};
use futures::channel::{mpsc, oneshot};
use futures::future::Shared;
use futures::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;

type ExecFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub(crate) type ExecFn<A> =
    Box<dyn for<'a> FnOnce(&'a mut A, &'a mut Context<A>) -> ExecFuture<'a> + Send + 'static>;

pub(crate) enum ActorEvent<A> {
    Exec(ExecFn<A>),
    Stop(Option<Error>),
    RemoveStream(usize),
}

/// The address of an actor.
///
/// When all references to `Addr<A>` are dropped, the actor ends.
/// You can use `Clone` trait to create multiple copies of `Addr<A>`.
pub struct Addr<A> {
    pub(crate) actor_id: u64,
    pub(crate) tx: Arc<mpsc::UnboundedSender<ActorEvent<A>>>,
    pub(crate) rx_exit: Option<Shared<oneshot::Receiver<()>>>,
}

impl<A> Clone for Addr<A> {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id,
            tx: self.tx.clone(),
            rx_exit: self.rx_exit.clone(),
        }
    }
}

impl<A> PartialEq for Addr<A> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<A> Hash for Addr<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

impl<A: Actor> Addr<A> {
    /// Returns the id of the actor.
    pub fn actor_id(&self) -> u64 {
        self.actor_id
    }

    /// Stop the actor.
    pub fn stop(&mut self, err: Option<Error>) -> Result<()> {
        mpsc::UnboundedSender::clone(&*self.tx).start_send(ActorEvent::Stop(err))?;
        Ok(())
    }

    /// Send a message `msg` to the actor and wait for the return value.
    pub async fn call<T: Message>(&self, msg: T) -> Result<T::Result>
    where
        A: Handler<T>,
    {
        let (tx, rx) = oneshot::channel();
        mpsc::UnboundedSender::clone(&*self.tx).start_send(ActorEvent::Exec(Box::new(
            move |actor, ctx| {
                Box::pin(async move {
                    let res = Handler::handle(actor, ctx, msg).await;
                    let _ = tx.send(res);
                })
            },
        )))?;

        Ok(rx.await?)
    }

    /// Send a message `msg` to the actor without waiting for the return value.
    pub fn send<T: Message<Result = ()>>(&self, msg: T) -> Result<()>
    where
        A: Handler<T>,
    {
        mpsc::UnboundedSender::clone(&*self.tx).start_send(ActorEvent::Exec(Box::new(
            move |actor, ctx| {
                Box::pin(async move {
                    Handler::handle(actor, ctx, msg).await;
                })
            },
        )))?;
        Ok(())
    }

    /// Create a `Caller<T>` for a specific message type
    pub fn caller<T: Message>(&self) -> Caller<T>
    where
        A: Handler<T>,
    {
        let addr = self.clone();
        Caller(Box::new(move |msg| {
            let addr = addr.clone();
            Box::pin(async move { addr.call(msg).await })
        }))
    }

    /// Create a `Sender<T>` for a specific message type
    pub fn sender<T: Message<Result = ()>>(&self) -> Sender<T>
    where
        A: Handler<T>,
    {
        let weak_tx = Arc::downgrade(&self.tx);
        Sender(Box::new(move |msg| match weak_tx.upgrade() {
            Some(tx) => {
                mpsc::UnboundedSender::clone(&tx).start_send(ActorEvent::Exec(Box::new(
                    move |actor, ctx| {
                        Box::pin(async move {
                            Handler::handle(&mut *actor, ctx, msg).await;
                        })
                    },
                )))?;
                Ok(())
            }
            None => Ok(()),
        }))
    }

    //  /// Create a `Sender<T>` for a specific message type
    //  pub fn sender<T: Message<Result = ()>>(&self) -> Sender<T>
    //  where
    //      A: Handler<T>,
    //  {
    //      let weak_tx = Arc::downgrade(&self.tx);
    //      Sender(Box::new(move |msg| {
    //          // FFS
    //          match weak_tx.upgrade() {
    //              Some(tx) => {
    //                  mpsc::UnboundedSender::clone(&*tx).start_send(ActorEvent::Exec(Box::new(
    //                      move |actor, ctx| {
    //                          Box::pin(async move {
    //                              let mut actor = actor.lock().await;
    //                              Handler::handle(&mut *actor, &ctx, msg).await;
    //                          })
    //                      },
    //                  )))?;
    //                  // Arc::downgrade(&tx);
    //                  Ok(())
    //              }
    //              None => Ok(()),
    //          }
    //      }))
    //  }

    /// Wait for an actor to finish, and if the actor has finished, the function returns immediately.
    pub async fn wait_for_stop(self) {
        if let Some(rx_exit) = self.rx_exit {
            rx_exit.await.ok();
        } else {
            futures::future::pending::<()>().await;
        }
    }
}
