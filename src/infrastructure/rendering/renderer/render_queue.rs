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
                if with_global_renderer(|r| {
                    task(r);
                })
                .is_none()
                {
                    // renderer not available
                }
            }
        });
    });
}

pub fn enqueue_render_task(task: RenderTask) {
    let owner = leptos::Owner::current();
    let task: RenderTask = Box::new(move |renderer| {
        if let Some(owner) = owner {
            let _ = leptos::try_with_owner(owner, || task(renderer));
        } else {
            task(renderer);
        }
    });
    RENDER_QUEUE.with(|cell| {
        if let Some(tx) = &*cell.borrow() {
            let _ = tx.unbounded_send(task);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::rendering::renderer::{
        GLOBAL_RENDERER, dummy_renderer, set_global_renderer,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    struct RestoreGlobals {
        renderer: Option<Rc<RefCell<WebGpuRenderer>>>,
        queue: Option<UnboundedSender<RenderTask>>,
    }

    impl Drop for RestoreGlobals {
        fn drop(&mut self) {
            RENDER_QUEUE.with(|cell| *cell.borrow_mut() = self.queue.take());
            GLOBAL_RENDERER.with(|cell| *cell.borrow_mut() = self.renderer.take());
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test(async)]
    async fn tasks_execute_in_order() {
        let _restore = RestoreGlobals {
            renderer: GLOBAL_RENDERER.with(|cell| cell.borrow_mut().take()),
            queue: RENDER_QUEUE.with(|cell| cell.borrow_mut().take()),
        };
        let scope = crate::test_support::AppTestScope::new();
        let result = Rc::new(RefCell::new(Vec::new()));
        let (completed, completion) = futures::channel::oneshot::channel();
        scope.run(|| {
            set_global_renderer(Rc::new(RefCell::new(dummy_renderer())));
            init_render_queue();
            let r1 = result.clone();
            enqueue_render_task(Box::new(move |_| r1.borrow_mut().push(1)));
            let r2 = result.clone();
            enqueue_render_task(Box::new(move |_| r2.borrow_mut().push(2)));
            enqueue_render_task(Box::new(move |_| completed.send(()).unwrap()));
        });
        // The browser schedules spawn_local work on a later microtask. Keep its
        // owner alive and wait for a trailing queue task instead of sleeping.
        completion.await.expect("queue completion");
        assert_eq!(*result.borrow(), vec![1, 2]);
    }
}
