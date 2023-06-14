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
use std::io::BufRead;
use std::ops::DerefMut;
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    thread,
    thread::ThreadId,
};

#[derive(Clone)]
pub struct ThreadCommunication {
    locked_data: Arc<Mutex<PerThreadDataMap>>,
}

type PerThreadDataMap = HashMap<ThreadId, Arc<Mutex<PerThreadData>>>;
struct PerThreadData {
    sender: mpsc::Sender<ThreadMessage>,
    // client_name: Option<String>,
}

#[derive(Clone)]
pub enum ThreadMessage {
    ReadMessageRequest(ThreadId),
    ReadMessageResponse(Result<(), String>, String),
    // Refresh,
    // Abort,
}

impl ThreadCommunication {
    pub fn new() -> Self {
        let result = PerThreadDataMap::new();
        let result = ThreadCommunication {
            locked_data: Arc::new(Mutex::new(result)),
        };
        result
    }

    pub fn add_current_thread(&mut self, sender: mpsc::Sender<ThreadMessage>) {
        let mut lock = self.locked_data.lock().unwrap();
        let data = lock.deref_mut();

        let thread_data = PerThreadData {
            // client_name: None,
            sender: sender,
        };
        let thread_data = Arc::new(Mutex::new(thread_data));
        data.insert(thread::current().id(), thread_data);
    }

    pub fn remove_current_thread(&mut self) {
        let mut lock = self.locked_data.lock().unwrap();
        let data = lock.deref_mut();

        data.remove(&thread::current().id());
    }

    pub fn process_messages_from_other_threads<T: BufRead>(
        &self,
        receiver: &mut mpsc::Receiver<ThreadMessage>,
        client_state: &ClientState<T>,
    ) {
        loop {
            let message = match receiver.try_recv() {
                Ok(x) => x,
                Err(_) => break,
            };

            match message {
                ThreadMessage::ReadMessageResponse(_, _) => panic!("Unexpected message"),
                ThreadMessage::ReadMessageRequest(sender_id) => {
                    let mut lock = self.locked_data.lock().unwrap();
                    let mut data = lock.deref_mut();
                    let message =
                        ThreadMessage::ReadMessageResponse(client_state.get_status().clone(), client_state.get_name_for_logging());

                    Self::unicast(&mut data, sender_id, message)
                }
                // ThreadMessage::Refresh => todo!(),
                // ThreadMessage::Abort => todo!(),
            }
        }
    }

    pub fn read_messages(
        &self,
        receiver: &mpsc::Receiver<ThreadMessage>,
        include_names: bool,
    ) -> Vec<String> {
        let mut data: PerThreadDataMap;
        {
            // Clone the metadata about threads, so we can release the lock
            // We'll be working on stale data, but it's better than holding
            // the mutex for so long.
            let mut lock = self.locked_data.lock().unwrap();
            let original_data = lock.deref_mut();
            data = original_data.clone();
        }

        // Broadcast message to all other threads and collect results. During this
        // process communication with some threads may fail, so we gather their IDs
        // and remove them from map.
        let mut thread_ids_to_remove: Vec<ThreadId> = vec![];
        Self::broadcast(
            &mut data,
            &mut thread_ids_to_remove,
            ThreadMessage::ReadMessageRequest(thread::current().id()),
        );
        let result = Self::collect(&mut data, &mut thread_ids_to_remove, receiver)
            .into_iter()
            .filter_map(|message| match message {
                ThreadMessage::ReadMessageResponse(status, name) => match status {
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

        // Remove non-functional threads
        // TODO is this even neccessary? The threads could remove themselves
        if thread_ids_to_remove.len() != 0 {
            let mut lock = self.locked_data.lock().unwrap();
            let data = lock.deref_mut();
            for id in thread_ids_to_remove.iter() {
                data.remove(id);
            }
        }

        result
    }

    fn broadcast(
        data: &mut PerThreadDataMap,
        threads_to_remove: &mut Vec<ThreadId>,
        message: ThreadMessage,
    ) {
        let this_thread_id = thread::current().id();

        data.iter()
            .filter(|(id, _)| **id != this_thread_id)
            .for_each(|(id, data)| {
                let per_thread_data = data.lock().unwrap();
                let send_result = per_thread_data.sender.send(message.clone());
                if let Err(_err) = send_result {
                    threads_to_remove.push(*id);
                }
            });
    }

    fn collect(
        data: &mut PerThreadDataMap,
        threads_to_remove: &mut Vec<ThreadId>,
        receiver: &mpsc::Receiver<ThreadMessage>,
    ) -> Vec<ThreadMessage> {
        let this_thread_id = thread::current().id();

        data.iter()
            .filter(|(id, _)| **id != this_thread_id)
            .filter_map(|(id, _)| {
                let result = receiver.recv();
                if let Ok(message) = result {
                    Some(message)
                } else {
                    threads_to_remove.push(*id);
                    None
                }
            })
            .collect()
    }

    fn unicast(data: &mut PerThreadDataMap, recepient: ThreadId, message: ThreadMessage) {
        let per_thread_data = match data.get(&recepient) {
            Some(x) => x.lock().unwrap(),
            None => panic!("Could not find the sender"),
        };
        if let Err(_err) = per_thread_data.sender.send(message) {
            panic!("Could not send");
        }
    }
}
