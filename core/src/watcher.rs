use anyhow::*;
use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    task::JoinHandle,
};

use crate::objects::Objects;

const POLL_SECONDS: u64 = 1;

pub enum ChangeEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
}

/// A naive approach to a single threaded file watcher. Uses polling to deduce what files have been
/// changed, this has a major drawback in if we need collaborative environments. A solution to this
/// is to create a trait where we can either use `notify` or our low resource usage solution.
struct Watcher<H: Hasher + Clone + Send + 'static> {
    // todo: roots can be a hashmap of Path -> last_state, we'll also store a mpsc queue here for
    // profit and pleasure. We should only need one reader so we expose that as an &mut
    // root: &'a Path,
    // last_state: Objects,
    watching: HashMap<PathBuf, JoinHandle<Result<()>>>,
    sender: Sender<ChangeEvent>,
    receiver: Receiver<ChangeEvent>,

    hasher: H,
}

impl<H: Hasher + Clone + Send + 'static> Watcher<H> {
    async fn from_dir(hasher: H) -> Result<Self> {
        // todo: How much capacity is appropriate here?
        let (sender, receiver) = channel(1000);
        Ok(Self {
            watching: HashMap::new(),
            hasher,
            sender,
            receiver,
        })
    }

    // will return a Receiver, we should only allow for one Receiver.
    // let's the compiler handle not allowing multiple subscribers
    pub async fn listen(&mut self) -> &mut Receiver<ChangeEvent> {
        &mut self.receiver
    }

    pub async fn unwatch(&mut self, path: &Path) -> Result<()> {
        if let Some(joiner) = self.watching.get_mut(path) {
            // attempts to abort the task, will need to look into how valid this is
            // a oneshot can also be used in a tokio::select! to handle this
            joiner.abort();
            let _ = joiner.await;
        } else {
            return Result::Err(anyhow!("{path:?} is not being watched"));
        }
        Ok(())
    }

    /// todo: Allow for multiple paths to be watched by this watcher at the same time,
    /// we basically will setup a task poller for each directory and send the events to our channel
    pub async fn watch(&mut self, path: &Path) -> Result<()> {
        if self.watching.contains_key(&path.to_path_buf()) {
            return Result::Err(anyhow!("{path:?} is already being watched"));
        }
        let sender = self.sender.clone();
        let path_buf = path.to_path_buf();
        // a arc mutex might be more efficient, but MutexGuards are weird to work with
        let mut hasher = self.hasher.clone();
        let handle = tokio::spawn(async move {
            let original = Objects::from_directory(&path_buf, &mut hasher).await?;
            tokio::time::sleep(Duration::from_secs(POLL_SECONDS)).await;
            let state = Objects::from_directory(&path_buf, &mut hasher).await?;
            let diff = original.diff(&state);
            for (key, _) in diff.added {
                sender.send(ChangeEvent::Created(key.to_path_buf())).await?;
            }
            for (key, _) in diff.removed {
                sender.send(ChangeEvent::Deleted(key.to_path_buf())).await?;
            }
            for (key, _) in diff.modified {
                sender
                    .send(ChangeEvent::Modified(key.to_path_buf()))
                    .await?;
            }
            Ok(())
        });
        self.watching.insert(path.to_path_buf(), handle);
        Ok(())
    }
}
