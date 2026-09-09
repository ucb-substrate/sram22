//! Design scripts executed and cached by a Substrate 2 context.
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use substrate::context::{Context, PrivateInstallation};

pub trait Script: 'static {
    type Params: Clone + Eq + Hash + Send + Sync + 'static;
    type Output: Send + Sync + 'static;
    fn run(params: &Self::Params, ctx: &Context) -> anyhow::Result<Self::Output>;
}
struct ScriptCache<S: Script>(Mutex<HashMap<S::Params, Arc<S::Output>>>);
impl<S: Script> Default for ScriptCache<S> {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}
impl<S: Script> PrivateInstallation for ScriptCache<S> {}
pub trait DesignContext {
    fn run_script<S: Script>(&self, params: &S::Params) -> anyhow::Result<Arc<S::Output>>;
}
impl DesignContext for Context {
    fn run_script<S: Script>(&self, params: &S::Params) -> anyhow::Result<Arc<S::Output>> {
        let cache = self.get_or_install(ScriptCache::<S>::default());
        if let Some(value) = cache.0.lock().unwrap().get(params).cloned() {
            return Ok(value);
        }
        // Scripts can call other scripts. Never hold the cache lock during execution.
        let value = Arc::new(S::run(params, self)?);
        let value = cache
            .0
            .lock()
            .unwrap()
            .entry(params.clone())
            .or_insert(value)
            .clone();
        Ok(value)
    }
}

/// Runs a Substrate 2 design script while servicing a legacy layout generator.
/// Layout layer keys are resolved against the calling layout context on this thread.
pub fn run_for_layout<S: Script>(
    layout: &substrate1::data::SubstrateCtx,
    params: &S::Params,
) -> substrate1::error::Result<Arc<S::Output>> {
    crate::with_layout_context(layout, |ctx| ctx.run_script::<S>(params)).map_err(|e| {
        substrate1::error::ErrorSource::from(std::io::Error::other(e.to_string())).into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static RUNS: AtomicUsize = AtomicUsize::new(0);
    struct Inner;
    impl Script for Inner {
        type Params = usize;
        type Output = usize;
        fn run(p: &usize, _: &Context) -> anyhow::Result<usize> {
            RUNS.fetch_add(1, Ordering::SeqCst);
            Ok(p * 2)
        }
    }
    struct Outer;
    impl Script for Outer {
        type Params = usize;
        type Output = usize;
        fn run(p: &usize, ctx: &Context) -> anyhow::Result<usize> {
            Ok(*ctx.run_script::<Inner>(p)? + 1)
        }
    }
    #[test]
    fn scripts_cache_results_and_allow_nested_calls() {
        let ctx = Context::builder().build();
        assert_eq!(*ctx.run_script::<Outer>(&3).unwrap(), 7);
        assert_eq!(*ctx.run_script::<Inner>(&3).unwrap(), 6);
        assert_eq!(*ctx.run_script::<Outer>(&3).unwrap(), 7);
        assert_eq!(RUNS.load(Ordering::SeqCst), 1);
        assert_eq!(*ctx.run_script::<Inner>(&4).unwrap(), 8);
        assert_eq!(RUNS.load(Ordering::SeqCst), 2);
    }
}
