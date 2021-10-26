use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

//fnonce() 不关心返回值
type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // let 语句结束后，临时变量的lockguard就被丢掉了，所以也释放了锁，这才能让下一个线程可以继续收到消息再取走任务
                let job = receiver.lock().unwrap().recv().unwrap();
                job();
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
            thread: thread,
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
        self.sender.send(job).unwrap();
    }
}
