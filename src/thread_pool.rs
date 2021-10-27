use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

enum Message {
    NewJob(Job),
    Terminate,
}

//fnonce() 不关心返回值
type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // let 语句结束后，临时变量的lockguard就被丢掉了，所以也释放了锁，这才能让下一个线程可以继续收到消息再取走任务
                let message = receiver.lock().unwrap().recv().unwrap();
                match message {
                    Message::NewJob(job) => {
                        println!("worker {} receive job", id);
                        job();
                    }
                    Message::Terminate => {
                        println!("worker {} will Terminate", id);
                        break;
                    }
                }
                // 显示的例子，这样guard的生命期覆盖了job，job中又有read阻塞住了,job不执行完那mutex就不会释放，其他线程也无法拿到receiver的job，就必须显示的drop掉guard来释放锁。
                // let guard = receiver.lock().unwrap();
                // let job = guard.recv().unwrap();
                // std::mem::drop(guard);
                // job();
            }
            // 更傻逼了，while let是表达式，临时变量gurad在整个表达式期间都有效所以没法释放
            // while let Ok(job) = receiver.lock().unwrap().recv() {
            //     println!("Worker {} got a job; executing.", id);
            //     job();
            // }
        });
        Worker {
            id: id,
            thread: Some(thread),
        }
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        let (tx, rx) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(rx));
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool {
            workers,
            sender: tx,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("beg to shutdown all workers!");
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(t) = worker.thread.take() {
                println!("shutdown work{} ---ing", worker.id);
                t.join().unwrap();
                println!("shutdown work{} completed", worker.id);
            }
        }
    }
}
