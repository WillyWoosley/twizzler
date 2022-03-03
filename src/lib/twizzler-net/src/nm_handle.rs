use std::{future::Future, sync::Arc};

use twizzler::object::{ObjID, Object, ObjectInitFlags, Protections};
use twizzler_queue::{CallbackQueueReceiver, QueueBase, QueueError, QueueSender};

use crate::{
    buffer::{BufferBase, BufferController},
    client_rendezvous,
    rx_req::{RxCompletion, RxRequest},
    tx_req::{TxCompletion, TxRequest},
};

#[cfg(feature = "manager")]
use crate::server_rendezvous;

struct NmHandleObjects {
    tx_queue: Object<QueueBase<TxRequest, TxCompletion>>,
    rx_queue: Object<QueueBase<RxRequest, RxCompletion>>,
    #[allow(dead_code)]
    tx_buf: Object<BufferBase>,
    #[allow(dead_code)]
    rx_buf: Object<BufferBase>,
}

pub struct NmHandle {
    _objs: NmHandleObjects,
    handler: CallbackQueueReceiver<RxRequest, RxCompletion>,
    sender: QueueSender<TxRequest, TxCompletion>,
    tx_bc: BufferController,
    rx_bc: BufferController,
}

#[cfg(feature = "manager")]
pub struct NmHandleManager {
    _objs: NmHandleObjects,
    handler: CallbackQueueReceiver<TxRequest, TxCompletion>,
    sender: QueueSender<RxRequest, RxCompletion>,
    tx_bc: BufferController,
    rx_bc: BufferController,
}

impl NmHandle {
    pub async fn handle<'a, F, Fut>(self: &'a Arc<NmHandle>, f: F) -> Result<(), QueueError>
    where
        F: Fn(&'a Arc<NmHandle>, u32, RxRequest) -> Fut,
        Fut: Future<Output = RxCompletion>,
    {
        self.handler.handle(move |id, req| f(self, id, req)).await
    }

    pub async fn submit(&self, req: TxRequest) -> Result<TxCompletion, QueueError> {
        self.sender.submit_and_wait(req).await
    }

    pub fn tx_buffer_controller(&self) -> &BufferController {
        &self.tx_bc
    }

    pub fn rx_buffer_controller(&self) -> &BufferController {
        &self.rx_bc
    }

    pub fn allocatable_buffer_controller(&self) -> &BufferController {
        &self.tx_bc
    }
}

#[cfg(feature = "manager")]
impl NmHandleManager {
    pub async fn handle<'a, F, Fut>(self: &'a Arc<NmHandleManager>, f: F) -> Result<(), QueueError>
    where
        F: Fn(&'a Arc<NmHandleManager>, u32, TxRequest) -> Fut,
        Fut: Future<Output = TxCompletion>,
    {
        self.handler.handle(move |id, req| f(self, id, req)).await
    }

    pub async fn submit(&self, req: RxRequest) -> Result<RxCompletion, QueueError> {
        self.sender.submit_and_wait(req).await
    }

    pub fn tx_buffer_controller(&self) -> &BufferController {
        &self.tx_bc
    }

    pub fn rx_buffer_controller(&self) -> &BufferController {
        &self.rx_bc
    }

    pub fn allocatable_buffer_controller(&self) -> &BufferController {
        &self.rx_bc
    }
}

pub fn open_nm_handle() -> Option<NmHandle> {
    let id = std::env::var("NETOBJ").ok()?;
    let id = id
        .parse::<u128>()
        .expect(&format!("failed to parse object ID string {}", id));
    let id = ObjID::new(id);
    let objs = client_rendezvous(id);
    let objs = NmHandleObjects {
        tx_queue: Object::init_id(
            objs.tx_queue,
            Protections::READ | Protections::WRITE,
            ObjectInitFlags::empty(),
        )
        .ok()?,
        rx_queue: Object::init_id(
            objs.rx_queue,
            Protections::READ | Protections::WRITE,
            ObjectInitFlags::empty(),
        )
        .ok()?,
        tx_buf: Object::init_id(
            objs.tx_buf,
            Protections::READ | Protections::WRITE,
            ObjectInitFlags::empty(),
        )
        .ok()?,
        rx_buf: Object::init_id(objs.rx_buf, Protections::READ, ObjectInitFlags::empty()).ok()?,
    };
    let sender = QueueSender::new(objs.tx_queue.clone().into());
    let handler = CallbackQueueReceiver::new(objs.rx_queue.clone().into());
    let tx_bc = BufferController::new(false, true, objs.tx_buf.clone());
    let rx_bc = BufferController::new(false, false, objs.rx_buf.clone());
    let handle = NmHandle {
        _objs: objs,
        handler,
        sender,
        tx_bc,
        rx_bc,
    };
    Some(handle)
}

#[cfg(feature = "manager")]
pub fn server_open_nm_handle() -> Option<NmHandleManager> {
    let id = std::env::var("NETOBJ").ok()?;
    let id = id
        .parse::<u128>()
        .expect(&format!("failed to parse object ID string {}", id));
    let id = ObjID::new(id);
    let objs = server_rendezvous(id);
    let objs = NmHandleObjects {
        tx_queue: Object::init_id(
            objs.tx_queue,
            Protections::READ | Protections::WRITE,
            ObjectInitFlags::empty(),
        )
        .ok()?,
        rx_queue: Object::init_id(
            objs.rx_queue,
            Protections::READ | Protections::WRITE,
            ObjectInitFlags::empty(),
        )
        .ok()?,
        tx_buf: Object::init_id(objs.tx_buf, Protections::READ, ObjectInitFlags::empty()).ok()?,
        rx_buf: Object::init_id(
            objs.rx_buf,
            Protections::READ | Protections::WRITE,
            ObjectInitFlags::empty(),
        )
        .ok()?,
    };
    let sender = QueueSender::new(objs.rx_queue.clone().into());
    let handler = CallbackQueueReceiver::new(objs.tx_queue.clone().into());
    let tx_bc = BufferController::new(true, true, objs.tx_buf.clone());
    let rx_bc = BufferController::new(true, false, objs.rx_buf.clone());
    let handle = NmHandleManager {
        _objs: objs,
        handler,
        sender,
        tx_bc,
        rx_bc,
    };
    Some(handle)
}
