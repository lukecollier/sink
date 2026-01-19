use anyhow::*;
use async_trait::async_trait;
use futures::executor::block_on;
use notify::{
    RecommendedWatcher, Watcher as _,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    sync::{
        Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    task::JoinHandle,
};

use crate::{objects::Objects, path_is_child, path_is_parent, project::Project};

const POLL_SECONDS: u64 = 1;

#[derive(Debug)]
pub enum ChangeEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
}

// todo: Let's add a method for "currently watched"
#[async_trait]
pub trait Watcher {
    async fn watch<H: Hasher + Default + Send>(&mut self, path: &Path) -> Result<()>;
    async fn unwatch(&mut self, path: &Path) -> Result<()>;
    async fn recv(&mut self) -> Option<ChangeEvent>;
}

pub struct NotifyWatcher {
    receiver: Receiver<ChangeEvent>,
    watcher: RecommendedWatcher,
    projects: Arc<Mutex<HashMap<PathBuf, Project>>>,
}

// todo: NotifyWatcher need's to
impl NotifyWatcher {
    pub async fn paths_vec(&self) -> Vec<PathBuf> {
        self.projects
            .lock()
            .await
            .keys()
            .map(|path| path.clone())
            .collect::<Vec<_>>()
    }

    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<ChangeEvent>(1000);
        let project_ls: HashMap<PathBuf, Project> = HashMap::new();
        let projects = Arc::new(Mutex::new(project_ls));
        let projects_clone = projects.clone();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            block_on(async {
                let mut event = res.unwrap();
                let projects = projects_clone.lock().await;
                match event.kind {
                    notify::EventKind::Create(CreateKind::File) => {
                        for path in event.paths {
                            for (root, project) in projects.iter() {
                                if path.starts_with(root) {
                                    if let Some(relative_path) =
                                        project.exists(&path, path.is_dir())
                                    {
                                        tx.send(ChangeEvent::Created(relative_path.to_path_buf()))
                                            .await
                                            .unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    notify::EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::To)) => {
                        if let Some(path) = event.paths.pop() {
                            for (root, project) in projects.iter() {
                                if path.starts_with(root) {
                                    if let Some(relative_path) =
                                        project.exists(&path, path.is_dir())
                                    {
                                        tx.send(ChangeEvent::Deleted(relative_path.to_path_buf()))
                                            .await
                                            .unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                        if let Some(path) = event.paths.pop() {
                            for (root, project) in projects.iter() {
                                if path.starts_with(root) {
                                    if let Some(relative_path) =
                                        project.exists(&path, path.is_dir())
                                    {
                                        tx.send(ChangeEvent::Created(relative_path.to_path_buf()))
                                            .await
                                            .unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    notify::EventKind::Modify(ModifyKind::Data(_)) => {
                        for path in event.paths {
                            for (root, project) in projects.iter() {
                                if path.starts_with(root) {
                                    if let Some(relative_path) =
                                        project.exists(&path, path.is_dir())
                                    {
                                        tx.send(ChangeEvent::Modified(relative_path.to_path_buf()))
                                            .await
                                            .unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    notify::EventKind::Remove(RemoveKind::File) => {
                        for path in event.paths {
                            for (root, project) in projects.iter() {
                                if path.starts_with(root) {
                                    if let Some(relative_path) =
                                        project.exists(&path, path.is_dir())
                                    {
                                        tx.send(ChangeEvent::Deleted(relative_path.to_path_buf()))
                                            .await
                                            .unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                };
            })
        })
        .unwrap();
        Self {
            receiver: rx,
            watcher,
            projects: projects.clone(),
        }
    }
}

#[async_trait]
impl Watcher for NotifyWatcher {
    async fn watch<H: Hasher + Default + Send>(&mut self, path: &Path) -> Result<()> {
        let mut do_not_continue = false;
        for path_buf in self.paths_vec().await {
            if path_is_child(&path, &path_buf) {
                // the new path is a child we simply ignore the add.
                do_not_continue = true;
            } else if path_is_parent(&path, &path_buf) {
                // if our new watch is above any of our current watched paths, unwatch.
                self.unwatch(&path_buf).await?;
            }
        }
        if do_not_continue {
            return Ok(());
        }
        self.projects
            .lock()
            .await
            .insert(path.to_path_buf(), Project::new_global_or_default(path));
        self.watcher.watch(path, notify::RecursiveMode::Recursive)?;
        Ok(())
    }
    async fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.projects.lock().await.remove(path);
        self.watcher.unwatch(path)?;
        Ok(())
    }
    async fn recv(&mut self) -> Option<ChangeEvent> {
        self.receiver.recv().await
    }
}

pub struct AsyncWatcher {
    watching: HashMap<PathBuf, JoinHandle<Result<()>>>,
    sender: Sender<ChangeEvent>,
    receiver: Receiver<ChangeEvent>,
}

impl AsyncWatcher {
    pub async fn new() -> Result<Self> {
        let (sender, receiver) = channel(1000);
        Ok(Self {
            watching: HashMap::new(),
            sender,
            receiver,
        })
    }
    pub fn paths_vec(&self) -> Vec<PathBuf> {
        self.watching
            .keys()
            .map(|path| path.clone())
            .collect::<Vec<_>>()
    }
}

#[async_trait]
impl Watcher for AsyncWatcher {
    async fn recv(&mut self) -> Option<ChangeEvent> {
        self.receiver.recv().await
    }

    async fn unwatch(&mut self, path: &Path) -> Result<()> {
        if let Some(joiner) = self.watching.get_mut(path) {
            joiner.abort();
            let _ = joiner.await;
        } else {
            return Result::Err(anyhow!("{path:?} is not being watched"));
        }
        self.watching.remove(path);
        Ok(())
    }

    /// todo: Allow for multiple paths to be watched by this watcher at the same time,
    /// we basically will setup a task poller for each directory and send the events to our channel
    async fn watch<H: Hasher + Default + Send>(&mut self, path: &Path) -> Result<()> {
        if self.watching.contains_key(&path.to_path_buf()) {
            return Result::Err(anyhow!("{path:?} is already being watched"));
        }
        let mut do_not_continue = false;
        for path_buf in self.paths_vec() {
            if path_is_child(&path, &path_buf) {
                // the new path is a child we simply ignore the add.
                do_not_continue = true;
            } else if path_is_parent(&path, &path_buf) {
                // if our new watch is above any of our current watched paths, unwatch.
                self.unwatch(&path_buf).await?;
            }
        }
        if do_not_continue {
            return Result::Err(anyhow!("{path:?} is a child of another watched path"));
        }
        let sender = self.sender.clone();
        let path_buf = path.to_path_buf();
        // a arc mutex might be more efficient, but MutexGuards are weird to work with
        let handle = tokio::spawn(async move {
            let mut before = Objects::from_directory::<H>(&path_buf).await?;
            let mut after = Objects::from_directory::<H>(&path_buf).await?;
            let mut start_at_sys = SystemTime::now();
            loop {
                let update_start = Instant::now();
                start_at_sys = after.update::<H>(start_at_sys).await?;
                let update_end = Instant::now();
                println!("time to update: {:?}", update_end - update_start);
                tokio::time::sleep(Duration::from_secs(POLL_SECONDS)).await;
                let start_at = Instant::now();
                let diff = before.diff(&after);
                for (key, _) in &diff.added {
                    sender.send(ChangeEvent::Created(key.to_path_buf())).await?;
                }
                for (key, _) in &diff.removed {
                    sender.send(ChangeEvent::Deleted(key.to_path_buf())).await?;
                }
                for (key, _) in &diff.modified {
                    sender
                        .send(ChangeEvent::Modified(key.to_path_buf()))
                        .await?;
                }
                before.patch(diff)?;
                let end_at = Instant::now();
                println!("time taken to poll: {:?}", end_at - start_at);
            }
        });
        self.watching.insert(path.to_path_buf(), handle);
        Ok(())
    }
}
