// Implementation of a mini tokio

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use futures::task::{self, ArcWake};
use std::thread;
use crossbeam::channel;
use std::sync::{Arc, Mutex};

struct MiniTokio
{
    scheduled: channel::Receiver<Arc<Task>>,
    sender: channel::Sender<Arc<Task>>,
}

struct Task
{
    // The mutex is to make the task implement sync. Only one
    // thread accesses future at any given time. The Mutex is not
    // required for correctness.Mutex

    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    executor: channel::Sender<Arc<Task>>,
}

impl Task {
    fn schedule(self: &Arc<Self>) {
        self.executor.send(self.clone());
    }

    fn poll(self: Arc<Self>) {
        // Create a waker from the Task instance. 
        // This uses the ArcWake impl from above.
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        // No other thread ever tries to lock the future
        let mut future = self.future.try_lock().unwrap();

        // Poll the future
        let _ = future.as_mut().poll(&mut cx);
    }


    // Spawns a new task with the given future.
    //
    // Initializes a new Task harness containing the given future and pushes it
    // onto `sender`. The receiver half of the channel will get the task and
    // execute it.

    fn spawn<F>(future: F, sender: &channel::Sender<Arc<Task>>)
    where 
        F: Future<Output = ()> + Send + 'static,

    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}

struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<&'static str>
    {
        if Instant::now() >= self.when {
            println!("Hello world");
            Poll::Ready("done")
        } else {
            // Get a handle to the waker for the current task
            let waker = cx.waker().clone();
            let when = self.when;


            // Spawn a timer thread
            thread::spawn(move || {
                let now = Instant::now();
                
                if now < when {
                    thread::sleep(when - now);
                }

                waker.wake();
            });

            // cx.waker().wake_by_ref();
            
            Poll::Pending
        }
    }
}

impl MiniTokio {
    fn new() -> MiniTokio {
        let (sender, scheduled) = channel::unbounded();
        MiniTokio {sender, scheduled}
    }

    // Spawn a future onto the mini-tokio instance
    fn spawn<F>(&mut self, future: F) 
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender);
    }

    fn run(&mut self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }
}


fn main() {
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {
        let when = Instant::now() + Duration::from_millis(10);
        let future = Delay {when};
        let out = future.await;
        assert_eq!(out, "done");
    });

    mini_tokio.run();
}