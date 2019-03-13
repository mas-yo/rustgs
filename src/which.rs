use futures::prelude::*;

pub(crate) struct Which<F,I,E> where F: Future<Item=I,Error=E> {
    value: Option<F::Item>,
    future: Option<F>,
}
impl<F,I,E> Future for Which<F,I,E> where F: Future<Item=I,Error=E> {
    type Item = I;
    type Error = E;
    fn poll(&mut self) -> Poll<I,E> {
        match &mut self.future {
            Some(f) => {
                f.poll()
            }
            None => {
                if self.value.is_some() {
                    Ok(Async::Ready(self.value.take().unwrap()))
                }
                else {
                    Ok(Async::NotReady)
                }
            }
        }
    }
}

impl<F,I,E> Which<F,I,E> where F: Future<Item=I,Error=E> {
    pub fn from_value(v: F::Item) -> Self {
        Self {value: Some(v), future:None}
    }
    pub fn from_future(f: F) -> Self {
        Self {value: None, future:Some(f)}
    }
}