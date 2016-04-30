use ::error::Result;
use std::thread;
use std::marker;


pub enum Command {
    Continue,
    Quit,
}

pub trait Recver<T> { // Recvs messages from mpsc, tcp, udp or other protocols
    fn recv(&mut self) -> Result<T>;
}

pub trait Handler<T> {
    fn handle(&mut self, T) -> Result<Command>;
}

pub struct Spinner<T, R, H>
        where R: Recver<T>,
              H: Handler<T> {
    recver: R,
    handler: H,
    phantom: marker::PhantomData<T>,
}

impl<T, R, H> Spinner<T, R, H>
        where R: Recver<T>,
              H: Handler<T> {
    pub fn new(recver: R, handler: H) -> Spinner<T, R, H> {
        Spinner {
            recver: recver,
            handler: handler,
            phantom: marker::PhantomData {}
        }
    }

    pub fn spin(&mut self) -> Result<()> {
        loop {
            let command = try!(self.recver.recv());
            match try!(self.handler.handle(command)) {
                Command::Quit => break,
                Command::Continue => (),
            };
        }
        Ok(())
    }
}

pub fn spawn<T, R, H>(recver: R,
                      handler: H) -> thread::JoinHandle<()>
        where T: 'static,
              R: 'static + Send + Recver<T>,
              H: 'static + Send + Handler<T> {
    thread::spawn(move || {
        if let Err(error) = Spinner::new(recver, handler).spin() {
            println!("#sirver spin_forever: {:#?}", error);
        }
    })
}
