use super::{TopicBus, TopicDelivery, TopicFolder, TopicListeners};
use crate::handler::process::{HandlerCancellation, HandlerSupervisor};
use crate::handler::source::HandlerSource;
use cap_std::fs::Dir;
use serde_json::json;
use std::sync::Arc;
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct TopicDeliveries {
    listeners: TopicListeners,
    receiver: mpsc::Receiver<TopicDelivery>,
}

impl TopicDeliveries {
    pub fn connect<'a>(
        project_root: &std::path::Path,
        supervisor: &HandlerSupervisor,
        sources: impl Iterator<Item = &'a HandlerSource>,
    ) -> Result<Self, crate::Failure> {
        let project = Dir::open_ambient_dir(project_root, cap_std::ambient_authority())
            .map_err(|_| TopicFolder::failure("The project folder cannot be opened."))?;
        let folder = TopicFolder::prepare(&Arc::new(project), supervisor.application())?;
        Ok(Self::with_folder(folder, supervisor, sources))
    }

    pub(crate) fn with_folder<'a>(
        folder: TopicFolder,
        supervisor: &HandlerSupervisor,
        sources: impl Iterator<Item = &'a HandlerSource>,
    ) -> Self {
        let (bus, receiver) = TopicBus::open(folder);
        supervisor.connect_topics(bus);
        Self {
            listeners: TopicListeners::from_sources(sources),
            receiver,
        }
    }

    pub async fn run(self, supervisor: Arc<HandlerSupervisor>, cancellation: CancellationToken) {
        self.deliver_until_cancelled(&supervisor, &cancellation)
            .await;
    }

    async fn deliver_until_cancelled(
        mut self,
        supervisor: &HandlerSupervisor,
        cancellation: &CancellationToken,
    ) {
        while let Some(delivery) = Self::next_delivery(&mut self.receiver, cancellation).await {
            self.deliver_to_targets(supervisor, &delivery, cancellation)
                .await;
        }
    }

    async fn next_delivery(
        receiver: &mut mpsc::Receiver<TopicDelivery>,
        cancellation: &CancellationToken,
    ) -> Option<TopicDelivery> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            next = receiver.recv() => next,
        }
    }

    async fn deliver_to_targets(
        &self,
        supervisor: &HandlerSupervisor,
        delivery: &TopicDelivery,
        cancellation: &CancellationToken,
    ) {
        let targets = self
            .listeners
            .targets(delivery)
            .into_iter()
            .take_while(|_| !cancellation.is_cancelled());
        for (source, keys) in targets {
            Self::deliver_to(supervisor, &source, delivery, &keys).await;
        }
    }

    async fn deliver_to(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        delivery: &TopicDelivery,
        listeners: &[String],
    ) {
        let request = Self::delivery_request(source, delivery, listeners);
        let cancellation = HandlerCancellation::default();
        match supervisor.invoke(source, &request, &cancellation).await {
            Ok(response) if (200..400).contains(&response.status) => {}
            Ok(response) => eprintln!(
                "Topic listener '{}' reported status {}",
                source.relative_path(),
                response.status
            ),
            Err(_) => eprintln!("Topic listener '{}' failed.", source.relative_path()),
        }
    }

    fn delivery_request(
        source: &HandlerSource,
        delivery: &TopicDelivery,
        listeners: &[String],
    ) -> HandlerRequest {
        let mut request = HandlerRequest::route(
            crate::ttid::generate(),
            format!("/{}", source.relative_path()),
            HttpMethod::Post,
        );
        request.operation = String::from(TopicBus::OPERATION);
        let body = json!({
            "topic": delivery.topic,
            "payload": delivery.payload,
            "hops": delivery.hops,
            "listeners": listeners,
        });
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: body.to_string(),
        });
        request
    }
}
