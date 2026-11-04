use std::thread;

use iced::futures::channel::oneshot;

pub fn background<T, F>(job: F) -> impl Future<Output = T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = oneshot::channel();

    thread::spawn(move || {
        let _ = sender.send(job());
    });

    async move { receiver.await.expect("background job panicked") }
}
