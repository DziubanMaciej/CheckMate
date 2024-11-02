// The logic in this file serves for communications between different tasks within the server. For most of the time they can work independently,
// but they are some operations for which they have to cooperate. All tasks should periodically call process_task_message and handle possible
// messages from other tasks. Cases to handle
// 1. Reading statuses:
//   - one task broadcast a query for status to all other tasks
//   - all tasks respond with their statuses
//   - the one task collects all the responses
// 2. Refreshing clients
//   - one task broadcasts a refresh instruction to all other tasks. The instruction can be either conditional (by name) or unconditional.
//   - all tasks check whether they should actually refresh based on their client name
//   - if a task should refresh, it enqueues a refresh signal to send to its client
// 3. Task creation/destruction

use crate::client_state::ClientState;
use check_mate_common::ServerCommand;
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
    RefreshByName(String),
    RefreshAll,
    ListClientsRequest(Sender<TaskMessage>),
    ListClientsResponse(String),
    // Abort,
}

impl TaskCommunication {
    pub fn new() -> Self {
        let result = PerThreadDataMap::new();
        TaskCommunication {
            locked_data: Arc::new(Mutex::new(result)),
        }
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

    pub async fn process_task_message(&self, message: TaskMessage, client_state: &mut ClientState) {
        match message {
            TaskMessage::ReadMessageResponse(_, _) => panic!("Unexpected task message"),
            TaskMessage::ReadMessageRequest(sender) => {
                let message = TaskMessage::ReadMessageResponse(
                    client_state.get_status().clone(),
                    client_state.get_name_or_default(),
                );
                Self::unicast(sender, message).await;
            }
            TaskMessage::RefreshByName(ref name) => {
                if let Some(current_name) = client_state.get_name() {
                    if current_name == name {
                        client_state
                            .push_command_to_send(ServerCommand::Refresh)
                            .await;
                    }
                }
            }
            TaskMessage::RefreshAll => {
                client_state
                    .push_command_to_send(ServerCommand::Refresh)
                    .await;
            }
            TaskMessage::ListClientsRequest(sender) => {
                let message = TaskMessage::ListClientsResponse(
                    client_state.get_name_or_default(),
                );
                Self::unicast(sender, message).await;
            }
            TaskMessage::ListClientsResponse(_) => panic!("Unexpected task message"),
        }
    }

    pub async fn refresh_client_by_name(&self, task_id: usize, name: String) {
        let data = self.get_locked_data_snapshot().await;
        let message = TaskMessage::RefreshByName(name);
        Self::broadcast(task_id, &data, message).await;
    }

    pub async fn refresh_all_clients(&self, task_id: usize) {
        let data = self.get_locked_data_snapshot().await;
        let message = TaskMessage::RefreshAll;
        Self::broadcast(task_id, &data, message).await;
    }

    pub async fn read_messages(
        &self,
        task_id: usize,
        receiver: &mut Receiver<TaskMessage>,
        sender: &Sender<TaskMessage>,
        include_names: bool,
    ) -> Vec<String> {
        let mut data = self.get_locked_data_snapshot().await;

        // Broadcast message to all other task and collect their responses
        // in a vector. The vector could be smaller than our task list, since
        // some of them might have ended in the meantime. This is not a problem,
        // we just ignore all send/receive errors.
        Self::broadcast(
            task_id,
            &data,
            TaskMessage::ReadMessageRequest(sender.clone()),
        )
        .await;

        Self::collect(task_id, &mut data, receiver)
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
            .collect()
    }

    pub async fn list_clients(
        &self,
        task_id: usize,
        receiver: &mut Receiver<TaskMessage>,
        sender: &Sender<TaskMessage>,
    ) -> Vec<String> {
        let mut data = self.get_locked_data_snapshot().await;

        // Broadcast message to all other task and collect their responses
        // in a vector. The vector could be smaller than our task list, since
        // some of them might have ended in the meantime. This is not a problem,
        // we just ignore all send/receive errors.
        Self::broadcast(
            task_id,
            &data,
            TaskMessage::ListClientsRequest(sender.clone()),
        ).await;

        Self::collect(task_id, &mut data, receiver)
            .await
            .into_iter()
            .filter_map(|message| match message {
                TaskMessage::ListClientsResponse(name) => {
                    Some(name)
                },
                _ => panic!("Unexpected message received"),
            })
            .collect()
    }

    async fn broadcast(task_id: usize, data: &PerThreadDataMap, message: TaskMessage) {
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

    async fn get_locked_data_snapshot(&self) -> PerThreadDataMap {
        // Clone the metadata about threads, so we can release the lock
        // We'll be working on stale data, but it's better than holding
        // the mutex for a very long time.
        let mut lock = self.locked_data.lock().await;
        let original_data = lock.deref_mut();
        original_data.clone()
    }
}
