// Communications to be made:
// 1. readMessages:
//   - one thread sends question to all other threads
//   - all threads respond
//
// 2. refresh
//   - one thread sends signal to one thread
//   - it sends something to its client
//
// 3. abort
//   - one thread sends abort to all other threads
//   - they abort
//
// 4. thread creation
//   - add to list of all threads
//
// 5 thread destruction
//   - remove from list

use crate::client_state::ClientState;
use std::ops::DerefMut;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

#[derive(Clone)]
pub struct TaskCommunication {
    locked_data: Arc<Mutex<PerThreadDataMap>>,
}

type PerThreadDataMap = HashMap<usize, Arc<Mutex<PerThreadData>>>;
struct PerThreadData {
    sender: Sender<TaskMessage>,
}

#[derive(Clone)]
pub enum TaskMessage {
    ReadMessageRequest(Sender<TaskMessage>),
    ReadMessageResponse(Result<(), String>, String),
    // Refresh,
    // Abort,
}

impl TaskCommunication {
    pub fn new() -> Self {
        let result = PerThreadDataMap::new();
        let result = TaskCommunication {
            locked_data: Arc::new(Mutex::new(result)),
        };
        result
    }

    pub async fn register_task(&mut self, task_id: usize, sender: Sender<TaskMessage>) {
        let mut lock = self.locked_data.lock().await;
        let data = lock.deref_mut();

        let thread_data = PerThreadData { sender };
        let thread_data = Arc::new(Mutex::new(thread_data));
        data.insert(task_id, thread_data);
    }

    pub async fn unregister_task(&mut self, task_id: usize) {
        let mut lock = self.locked_data.lock().await;
        let data = lock.deref_mut();

        data.remove(&task_id);
    }

    pub async fn process_task_message(&self, message: TaskMessage, client_state: &ClientState) {
        match message {
            TaskMessage::ReadMessageResponse(_, _) => panic!("Unexpected task message"),
            TaskMessage::ReadMessageRequest(sender) => {
                    let message =
                    TaskMessage::ReadMessageResponse(client_state.get_status().clone(), client_state.get_name_for_logging());
                    Self::unicast(sender, message).await;
                }
                // TaskMessage::Refresh => todo!(),
                // TaskMessage::Abort => todo!(),
            }
    }

    pub async fn read_messages(
        &self,
        task_id: usize,
        receiver: &mut Receiver<TaskMessage>,
        sender: &Sender<TaskMessage>,
        include_names: bool,
    ) -> Vec<String> {
        let mut data: PerThreadDataMap;
        {
            // Clone the metadata about threads, so we can release the lock
            // We'll be working on stale data, but it's better than holding
            // the mutex for so long.
            let mut lock = self.locked_data.lock().await;
            let original_data = lock.deref_mut();
            data = original_data.clone();
        }

        // Broadcast message to all other task and collect their responses
        // in a vector. The vector could be smaller than our task list, since
        // some of them might have ended in the meantime. This is not a problem,
        // we just ignore all send/receive errors.
        Self::broadcast(
            task_id,
            &mut data,
            TaskMessage::ReadMessageRequest(sender.clone()),
        )
        .await;

        let result = Self::collect(task_id, &mut data, receiver)
            .await
            .into_iter()
            .filter_map(|message| match message {
                TaskMessage::ReadMessageResponse(status, name) => match status {
                    Ok(_) => None,
                    Err(mut status_string) => {
                        if include_names {
                            status_string = format!("{}: {}", name, status_string);
                        }
                        Some(status_string)
                    }
                },
                _ => panic!("Unexpected message received"),
            })
            .collect();

        result
    }

    async fn broadcast(task_id: usize, data: &mut PerThreadDataMap, message: TaskMessage) {
        for (_id, data) in data.iter().filter(|(id, _)| **id != task_id) {
            let per_thread_data = data.lock().await;
            let _send_result = per_thread_data.sender.send(message.clone()).await;
        }
    }

    async fn collect(
        task_id: usize,
        data: &mut PerThreadDataMap,
        receiver: &mut Receiver<TaskMessage>,
    ) -> Vec<TaskMessage> {
        let mut result: Vec<TaskMessage> = Vec::new();

        // TODO this will deadlock if we successfully broadcast, but one of the tasks doesn't respond and we don't
        // have the same number of responses. Is this even possible, though? How to handle this? We would need a
        //separate receiver for each pair of threads... Maybe use refcount of sender?
        let tasks_count = data.iter().filter(|(id, _)| **id != task_id).count();
        for _ in 0..tasks_count {
            let receive_result = receiver.recv().await;
            if let Some(message) = receive_result {
                result.push(message);
            }
        }
        result
    }

    async fn unicast(sender: Sender<TaskMessage>, message: TaskMessage) {
        if let Err(_err) = sender.send(message).await {
            panic!("Could not send");
        }
    }
}
