use tokio::task::JoinSet;

pub struct TaskManager<T> {
    join_set: JoinSet<T>,
    max_concurrent: usize,
}
impl<T: 'static> TaskManager<T> {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            join_set: JoinSet::new(),
            max_concurrent,
        }
    }
    pub async fn spawn_or_wait<F, Fut>(&mut self, task: F) -> Option<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // If we're at capacity, wait for one task to complete
        if self.join_set.len() >= self.max_concurrent {
            // Wait for any task to complete and return its result
            return self.join_set.join_next().await.and_then(|res| res.ok());
        }

        // We have capacity, spawn the new task
        let future = task();
        self.join_set.spawn(future);

        // No completed task to return yet
        None
    }

    // Helper method to wait for all tasks to complete
    pub async fn join_all(&mut self) -> Vec<T> {
        let mut results = Vec::new();
        while let Some(result) = self.join_set.join_next().await {
            if let Ok(value) = result {
                results.push(value);
            }
        }
        results
    }

    // Helper method to get the number of running tasks
    pub fn len(&self) -> usize {
        self.join_set.len()
    }

    // Helper method to check if there are any running tasks
    pub fn is_empty(&self) -> bool {
        self.join_set.is_empty()
    }
}
