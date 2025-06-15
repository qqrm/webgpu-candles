use super::{WebGpuRenderer, with_global_renderer};
use futures::{
    StreamExt,
    channel::mpsc::{UnboundedSender, unbounded},
};
use std::cell::RefCell;

thread_local! {
    static RENDER_QUEUE: RefCell<Option<UnboundedSender<RenderTask>>> = const { RefCell::new(None) };
}

type RenderTask = Box<dyn FnOnce(&mut WebGpuRenderer) + 'static>;

#[cfg(not(target_arch = "wasm32"))]
fn spawn_async<F>(fut: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    futures::executor::block_on(fut);
}

#[cfg(target_arch = "wasm32")]
fn spawn_async<F>(fut: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    leptos::spawn_local(fut);
}

pub fn init_render_queue() {
    RENDER_QUEUE.with(|cell| {
        if cell.borrow().is_some() {
            return;
        }
        let (tx, mut rx) = unbounded::<RenderTask>();
        *cell.borrow_mut() = Some(tx);
        spawn_async(async move {
            while let Some(task) = rx.next().await {
                with_global_renderer(|r| {
                    task(r);
                });
            }
        });
    });
}

pub fn enqueue_render_task(task: RenderTask) {
    RENDER_QUEUE.with(|cell| {
        if let Some(tx) = &*cell.borrow() {
            let _ = tx.unbounded_send(task);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::rendering::renderer::{dummy_renderer, set_global_renderer};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn tasks_execute_in_order() {
        init_render_queue();
        let renderer = Rc::new(RefCell::new(dummy_renderer()));
        set_global_renderer(renderer);

        let result = Rc::new(RefCell::new(Vec::new()));
        let r1 = result.clone();
        enqueue_render_task(Box::new(move |_| r1.borrow_mut().push(1)));
        let r2 = result.clone();
        enqueue_render_task(Box::new(move |_| r2.borrow_mut().push(2)));

        assert_eq!(*result.borrow(), vec![1, 2]);
    }
}
