use anyhow::{Context, Error};
use async_channel::{bounded, Receiver, Sender};
use reqwest::{Client as ReqClient, Request, Response};
use tokio::sync::oneshot;
use tokio::time::Instant;

#[derive(Debug, Clone)]
struct ClientActor {
    inner: ReqClient,
    receiver: Receiver<ActorRequest>,
    init: Instant,
}

struct ActorRequest {
    request: Request,
    sender: oneshot::Sender<Result<Response, Error>>,
}

impl ClientActor {
    fn new(receiver: Receiver<ActorRequest>) -> Self {
        let inner = ReqClient::new();
        let init = Instant::now();
        ClientActor {
            inner,
            receiver,
            init,
        }
    }

    async fn execute(&self, request: ActorRequest) {
        println!("Request > Start at : {}", self.init.elapsed().as_secs());
        let response = self.inner.execute(request.request).await.context("");
        request.sender.send(response).unwrap();
        println!("Request > End at : {}", self.init.elapsed().as_secs());
    }
}

async fn start_client_actor(actor: ClientActor) {
    while let Ok(request) = actor.receiver.recv().await {
        tokio::spawn({
            let actor = actor.clone();
            async move {
                actor.execute(request).await;
            }
        });
    }
}

pub struct Client {
    sender: Sender<ActorRequest>,
}

impl Client {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(4);
        let actor = ClientActor::new(receiver);
        tokio::spawn(start_client_actor(actor));

        Self { sender }
    }

    pub async fn fetch(&self, request: Request) -> Result<Response, Error> {
        let (sender, recv) = oneshot::channel::<Result<Response, Error>>();
        let request = ActorRequest { request, sender };

        self.sender
            .send(request)
            .await
            .context("Could not send request to client")?;

        recv.await
            .context("Response Sender dropped unexpectedly")?
            .context("Actor task has been killed")
    }
}
