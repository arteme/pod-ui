use std::future::Future;
use std::sync::{Arc, Mutex};
use log::warn;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use unicycle::FuturesUnordered;
use crate::controller::Controller;
use crate::store::{Event, Signal, Store, StoreSetIm};

pub struct ControllerStack {
    controllers: Vec<Arc<Mutex<Controller>>>,
    rxs: Vec<broadcast::Receiver<Event<String>>>,
    interrupt_tx: broadcast::Sender<()>,
    interrupt_rx: broadcast::Receiver<()>,
}

impl ControllerStack {
    fn new() -> Self {
        let (interrupt_tx, interrupt_rx) = broadcast::channel(16);
        Self {
            controllers: vec![],
            rxs: vec![],
            interrupt_tx,
            interrupt_rx
        }
    }

    fn add(&mut self, controller: Arc<Mutex<Controller>>) {
        let rx = controller.subscribe();
        self.controllers.push(controller);
        self.rxs.push(rx);

        self.interrupt_tx.send(())
            .map_err(|e| warn!("Failed to send interrupt signal"))
            .unwrap_or_default();
    }

    fn remove(&mut self, controller: Arc<Mutex<Controller>>) -> bool {
        let i = self.controllers.iter().enumerate()
            .find(|(i, c)| Arc::ptr_eq(*c, &controller) )
            .map(|(i,_)| i);
        if let Some(i) = i {
            self.controllers.remove(i);
            self.rxs.remove(i);

            self.interrupt_tx.send(())
                .map_err(|e| warn!("Failed to send interrupt signal"))
                .unwrap_or_default();

            true
        } else {
            false
        }
    }

    fn controller_for(&self, key: &str) -> Option<&Arc<Mutex<Controller>>> {
        self.controllers.iter().find(|c| c.has(key))
    }

    async fn recv_opt(&mut self) -> Result<Option<Event<String>>, RecvError> {
        let mut futures = FuturesUnordered::new();
        for rx in self.rxs.iter_mut() {
            futures.push(rx.recv());
        }

        loop {
            tokio::select! {
                msg = futures.next() => {
                    if let Some(msg) = msg {
                        match msg {
                            Err(Lagged) => continue,
                            Err(Closed) => return Ok(None),
                            Ok(v) => return Ok(Some(v)),
                        }
                    } else {
                        return Ok(None)
                    }
                }
                msg = self.interrupt_rx.recv() => {
                    match msg {
                        Err(Lagged) => continue,
                        Err(Closed) => return Err(Closed),
                        Ok(_) => return Ok(None),
                    }
                }
            }
        }
    }

    async fn recv(&mut self) -> Result<Event<String>, RecvError> {
        loop {
            match self.recv_opt().await {
                Ok(None) => continue,
                Ok(Some(v)) => return Ok(v),
                Err(e) => return Err(e)
            }
        }
    }
}

impl Store<&str, u16, String> for ControllerStack {
    fn has(&self, key: &str) -> bool {
        self.controller_for(key).is_some()
    }

    fn get(&self, key: &str) -> Option<u16> {
        self.controller_for(key)
            .and_then(|c| c.get(key))
    }

    fn set_full(&mut self, key: &str, value: u16, origin: u8, signal: Signal) -> bool {
        self.controller_for(key)
            .map(|c| c.set_full(key, value, origin, signal))
            .unwrap_or(false)
    }

    fn subscribe(&self) -> broadcast::Receiver<Event<String>> {
        todo!()
    }
}

