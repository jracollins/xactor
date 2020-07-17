use crate::addr::ActorEvent;
use crate::runtime::spawn;
use crate::{Actor, Addr, Context};
use anyhow::Result;
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::oneshot;
use futures::{FutureExt, StreamExt};

/// Actor supervisor
///
/// Supervisor gives the actor the ability to restart after failure.
/// When the actor fails, recreate a new actor instance and replace it.
pub struct Supervisor;

impl Supervisor {
    /// Start a supervisor
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xactor::*;
    /// use std::time::Duration;
    ///
    /// #[message]
    /// struct Die;
    ///
    /// #[message]
    /// struct Add;
    ///
    /// #[message(result = "i32")]
    /// struct Get;
    ///
    /// struct MyActor(i32);
    ///
    /// impl Actor for MyActor {}
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Add> for MyActor {
    ///     async fn handle(&mut self, ctx: &Context<Self>, _: Add) {
    ///         self.0 += 1;
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Get> for MyActor {
    ///     async fn handle(&mut self, ctx: &Context<Self>, _: Get) -> i32 {
    ///         self.0
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Die> for MyActor {
    ///     async fn handle(&mut self, ctx: &Context<Self>, _: Die) {
    ///         ctx.stop(None);
    ///     }
    /// }
    ///
    /// #[xactor::main]
    /// async fn main() -> Result<()> {
    ///     let mut addr = Supervisor::start(|| MyActor(0)).await?;
    ///
    ///     addr.send(Add)?;
    ///     assert_eq!(addr.call(Get).await?, 1);
    ///
    ///     addr.send(Add)?;
    ///     assert_eq!(addr.call(Get).await?, 2);
    ///
    ///     addr.send(Die)?;
    ///     sleep(Duration::from_secs(1)).await; // Wait for actor restart
    ///
    ///     assert_eq!(addr.call(Get).await?, 0);
    ///     Ok(())
    /// }
    /// ```
    pub async fn start<A, F>(f: F) -> Result<Addr<A>>
    where
        A: Actor,
        F: Fn() -> A + Send + 'static,
    {
        let (tx_exit, rx_exit) = oneshot::channel();
        let rx_exit = rx_exit.shared();
        let (mut ctx, mut rx, tx) = Context::new(Some(rx_exit));
        let addr = Addr {
            actor_id: ctx.actor_id(),
            tx: tx.clone(),
            rx_exit: ctx.rx_exit.clone(),
        };

        // Create the actor
        let mut actor = f();

        // Call started
        actor.started(&mut ctx).await?;

        spawn({
            async move {
                loop {
                    while let Some(event) = rx.next().await {
                        match event {
                            ActorEvent::Exec(f) => f(&mut actor, &mut ctx).await,
                            ActorEvent::Stop(_err) => break,
                            ActorEvent::RemoveStream(id) => {
                                if ctx.streams.contains(id) {
                                    ctx.streams.remove(id);
                                }
                            }
                        }
                    }

                    actor.stopped(&mut ctx).await;
                    for (_, handle) in ctx.streams.iter() {
                        handle.abort();
                    }

                    actor.restarted(&mut ctx).await.ok();
                }
            }
        });

        Ok(addr)
    }
}

// while let Some(event) = rx.next().await {
//     match event {
//         ActorEvent::Exec(f) => f(actor.clone(), ctx.clone()).await,
//         ActorEvent::Stop(_err) => {
//             actor.lock().await.stopped(&ctx).await;
//             actor.lock().await.started(&ctx).await.ok();
//         }
//     }
// }

// actor.lock().await.stopped(&ctx).await;
