use std::sync::Arc;
use std::thread;
use std::time::Duration;

use iced::futures::Stream;
use iced::futures::channel::{mpsc, oneshot};

use crate::scan::progress::{Progress, Snapshot};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub enum Update<T> {
    Progress(Snapshot),
    Finished(T, Snapshot),
}

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

pub fn with_progress<T, F>(job: F) -> impl Stream<Item = Update<T>>
where
    T: Send + 'static,
    F: FnOnce(&Progress) -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::unbounded();

    thread::spawn(move || {
        let progress = Arc::new(Progress::default());

        let worker = {
            let progress = Arc::clone(&progress);
            let reporter = thread::current();

            thread::spawn(move || {
                let result = job(&progress);
                let finished = progress.snapshot();
                reporter.unpark();
                (result, finished)
            })
        };

        while !worker.is_finished() {
            let _ = sender.unbounded_send(Update::Progress(progress.snapshot()));
            thread::park_timeout(PROGRESS_INTERVAL);
        }

        if let Ok((result, finished)) = worker.join() {
            let _ = sender.unbounded_send(Update::Finished(result, finished));
        }
    });

    receiver
}
