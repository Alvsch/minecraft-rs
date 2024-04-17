use std::{future::Future, sync::OnceLock};

use tokio::runtime::Runtime;

pub trait BlockOn: Future + Sized {
    fn block(self) -> Self::Output {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        let rt = RUNTIME.get_or_init(|| { Runtime::new().unwrap() });
        rt.block_on(self)
    }
}

impl<T: Future> BlockOn for T {}