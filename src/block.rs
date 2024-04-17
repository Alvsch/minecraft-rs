use futures::Future;

pub trait BlockOn: Future + Sized {
    fn block(self) -> Self::Output {
        futures::executor::block_on(self)
    }
}

impl<T: Future> BlockOn for T {}