use futures::prelude::*;
use std::sync::Arc;
use std::sync::RwLock;

pub(crate) trait Update {
    fn update(&mut self);
}

pub(crate) trait PushTask {
    fn push_task<F>(&mut self, f: F)
    where
        F: 'static + Future<Item = (), Error = ()> + Send + Sync;
}

pub(crate) type SharedTasks = Arc<RwLock<Tasks>>;
pub(crate) type Tasks = Vec<Box<dyn Future<Item = (), Error = ()> + Send + Sync>>;

pub(crate) fn new_tasks() -> Tasks {
    Vec::new()
}
pub(crate) fn new_shared_tasks() -> SharedTasks {
    Arc::new(RwLock::new(new_tasks()))
}

impl Update for Tasks {
    fn update(&mut self) {
        let mut i = 0;
        while i != self.len() {
            if let Ok(Async::Ready(_)) = self[i].poll() {
                self.remove(i);
            } else {
                //TODO error
                i += 1;
            }
        }
    }
}

impl PushTask for Tasks {
    fn push_task<F>(&mut self, f: F)
    where
        F: 'static + Future<Item = (), Error = ()> + Send + Sync,
    {
        self.push(Box::new(f));
    }
}
