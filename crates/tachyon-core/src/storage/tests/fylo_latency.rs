use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{ManagedStorage, execute, fylo_store};

struct LatencyRun {
    store: ManagedStorage,
    ids: Vec<String>,
}

impl LatencyRun {
    const SEEDED: usize = 200;
    const WARMUP: usize = 20;
    const ROUNDS: usize = 300;

    fn driver() -> &'static str {
        if cfg!(feature = "storage-fylo-in-process") {
            "in-process"
        } else {
            "subprocess"
        }
    }

    fn document(command: &str, fields: Value) -> Value {
        let mut request = json!({"operation":"document","command":command,"collection":"bench"});
        let Value::Object(fields) = fields else {
            panic!("fields");
        };
        for (key, value) in fields {
            request[&key] = value;
        }
        request
    }

    fn queue(command: &str, fields: Value) -> Value {
        let mut request = json!({"operation":"queue","command":command,"topic":"bench.jobs"});
        let Value::Object(fields) = fields else {
            panic!("fields");
        };
        for (key, value) in fields {
            request[&key] = value;
        }
        request
    }

    async fn seeded(root: &std::path::Path) -> Self {
        let store = fylo_store(root).await;
        execute(&store, Self::document("createCollection", json!({}))).await;
        let mut ids = Vec::with_capacity(Self::SEEDED);
        for index in 0..Self::SEEDED {
            let inserted = Self::document("insert", json!({"value":{"kind":"seed","n":index}}));
            let saved = execute(&store, inserted).await;
            ids.push(saved["id"].as_str().unwrap_or_default().to_owned());
        }
        Self { store, ids }
    }

    async fn timed(&self, command: Value) -> (Duration, Value) {
        let started = Instant::now();
        let answer = execute(&self.store, command).await;
        (started.elapsed(), answer)
    }

    async fn measure<Make>(&self, operation: &str, make: Make)
    where
        Make: Fn(usize) -> Value,
    {
        for round in 0..Self::WARMUP {
            self.timed(make(round)).await;
        }
        let mut samples = Vec::with_capacity(Self::ROUNDS);
        for round in 0..Self::ROUNDS {
            samples.push(self.timed(make(Self::WARMUP + round)).await.0);
        }
        Self::report(operation, &mut samples);
    }

    async fn measure_claim_and_ack(&self) {
        for round in 0..Self::WARMUP + Self::ROUNDS {
            let publish = Self::queue("publish", json!({"payload":{"round":round}}));
            execute(&self.store, publish).await;
        }
        let claim = Self::queue("claim", json!({"group":"workers","maxMessages":1}));
        let mut claims = Vec::with_capacity(Self::ROUNDS);
        let mut acks = Vec::with_capacity(Self::ROUNDS);
        for round in 0..Self::WARMUP + Self::ROUNDS {
            let (claimed_in, claimed) = self.timed(claim.clone()).await;
            let delivery = &claimed[0];
            let settle =
                json!({"group":"workers","id":delivery["id"],"receipt":delivery["receipt"]});
            let (acked_in, _) = self.timed(Self::queue("ack", settle)).await;
            if round >= Self::WARMUP {
                claims.push(claimed_in);
                acks.push(acked_in);
            }
        }
        Self::report("queue claim", &mut claims);
        Self::report("queue ack", &mut acks);
    }

    fn report(operation: &str, samples: &mut [Duration]) {
        Self::report_for(Self::driver(), operation, samples);
    }

    fn report_for(driver: &str, operation: &str, samples: &mut [Duration]) {
        samples.sort_unstable();
        let percentile = |share: usize| samples[(samples.len() * share).div_ceil(100) - 1];
        println!(
            "fylo-latency driver={driver} op=\"{operation}\" n={} p50_us={} p95_us={}",
            samples.len(),
            percentile(50).as_micros(),
            percentile(95).as_micros(),
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Benchmark: needs the fylo binary for the subprocess driver; run with --run-ignored only and --no-capture"]
async fn fylo_operation_latency() {
    let directory = tempfile::tempdir().expect("root");
    let root = std::fs::canonicalize(directory.path()).expect("canonical root");
    let run = LatencyRun::seeded(&root).await;
    let id = |round: usize| run.ids[round % LatencyRun::SEEDED].clone();
    run.measure("document insert", |round| {
        LatencyRun::document("insert", json!({"value":{"kind":"bench","n":round}}))
    })
    .await;
    run.measure("document findById", |round| {
        LatencyRun::document("findById", json!({"id":id(round)}))
    })
    .await;
    run.measure("document patch", |round| {
        LatencyRun::document("patch", json!({"id":id(round),"value":{"touched":round}}))
    })
    .await;
    run.measure("document scroll 10", |_| {
        LatencyRun::document("scroll", json!({"size":10}))
    })
    .await;
    run.measure("document scroll where", |round| {
        let n = round % LatencyRun::SEEDED;
        LatencyRun::document("scroll", json!({"size":10,"where":{"n":n}}))
    })
    .await;
    run.measure("queue publish", |round| {
        LatencyRun::queue("publish", json!({"payload":{"round":round}}))
    })
    .await;
    run.measure_claim_and_ack().await;
    let inserted: Vec<String> = {
        let mut ids = Vec::with_capacity(LatencyRun::WARMUP + LatencyRun::ROUNDS);
        for round in 0..LatencyRun::WARMUP + LatencyRun::ROUNDS {
            let insert = LatencyRun::document("insert", json!({"value":{"doomed":round}}));
            let saved = execute(&run.store, insert).await;
            ids.push(saved["id"].as_str().unwrap_or_default().to_owned());
        }
        ids
    };
    run.measure("document deleteById", |round| {
        LatencyRun::document("deleteById", json!({"id":inserted[round]}))
    })
    .await;
    run.store.close().await;
}

#[cfg(feature = "storage-fylo-in-process")]
struct EngineLatency {
    root: fylo::Root,
    ids: Vec<String>,
}

#[cfg(feature = "storage-fylo-in-process")]
impl EngineLatency {
    fn document(fields: &Value) -> fylo::Document {
        fields.as_object().cloned().unwrap_or_default()
    }

    fn seeded(path: &std::path::Path) -> Self {
        let root = fylo::Root::open(path, fylo::OpenOptions::default()).expect("open root");
        root.create_collection("bench").expect("collection");
        let ids = (0..LatencyRun::SEEDED)
            .map(|index| {
                let document = Self::document(&json!({"kind":"seed","n":index}));
                root.collection("bench").put(document).expect("seed")
            })
            .collect();
        Self { root, ids }
    }

    fn measure<Call>(&self, operation: &str, call: Call)
    where
        Call: Fn(&fylo::Root, usize),
    {
        for round in 0..LatencyRun::WARMUP {
            call(&self.root, round);
        }
        let mut samples = Vec::with_capacity(LatencyRun::ROUNDS);
        for round in 0..LatencyRun::ROUNDS {
            let started = Instant::now();
            call(&self.root, LatencyRun::WARMUP + round);
            samples.push(started.elapsed());
        }
        LatencyRun::report_for("engine-only", operation, &mut samples);
    }

    fn query(fields: &Value) -> fylo::Query {
        fylo::Query::from_json(&json!({ "$ops": [fields] })).expect("query")
    }

    fn measure_claim_and_ack(&self) {
        let queue = self.root.queue("bench.jobs");
        let total = LatencyRun::WARMUP + LatencyRun::ROUNDS;
        for round in 0..total {
            let options = fylo::PublishOptions::default();
            queue
                .publish(json!({"round":round}), &options)
                .expect("publish");
        }
        let mut claims = Vec::with_capacity(LatencyRun::ROUNDS);
        let mut acks = Vec::with_capacity(LatencyRun::ROUNDS);
        for _ in 0..total {
            let options = fylo::ClaimOptions {
                max_messages: 1,
                ..fylo::ClaimOptions::default()
            };
            let started = Instant::now();
            let claimed = queue.claim("workers", options).expect("claim");
            let claimed_in = started.elapsed();
            let delivery = serde_json::to_value(&claimed[0]).expect("delivery");
            let id = delivery["id"].as_str().unwrap_or_default();
            let receipt = delivery["receipt"].as_str().unwrap_or_default();
            let started = Instant::now();
            queue.ack("workers", id, receipt).expect("ack");
            let acked_in = started.elapsed();
            claims.push(claimed_in);
            acks.push(acked_in);
        }
        let mut claims = claims.split_off(LatencyRun::WARMUP);
        let mut acks = acks.split_off(LatencyRun::WARMUP);
        LatencyRun::report_for("engine-only", "queue claim", &mut claims);
        LatencyRun::report_for("engine-only", "queue ack", &mut acks);
    }
}

#[cfg(feature = "storage-fylo-in-process")]
#[test]
#[ignore = "Benchmark: the fylo crate's own time for the calls TACHYON's commands make; run with --run-ignored only and --no-capture"]
fn fylo_engine_latency() {
    let directory = tempfile::tempdir().expect("root");
    let path = std::fs::canonicalize(directory.path()).expect("canonical root");
    let run = EngineLatency::seeded(&path);
    let id = |round: usize| run.ids[round % LatencyRun::SEEDED].as_str();
    run.measure("document insert", |root, round| {
        let document = EngineLatency::document(&json!({"kind":"bench","n":round}));
        root.collection("bench").put(document).expect("put");
    });
    run.measure("document findById", |root, round| {
        root.collection("bench").latest(id(round)).expect("latest");
    });
    run.measure("document patch", |root, round| {
        let changes = EngineLatency::document(&json!({"touched":round}));
        let collection = root.collection("bench");
        collection.patch(id(round), &changes).expect("patch");
        collection.get(id(round)).expect("get");
    });
    run.measure("document scroll 10", |root, _| {
        let everything = EngineLatency::query(&json!({}));
        root.collection("bench").find(&everything).expect("find");
    });
    run.measure("document scroll where", |root, round| {
        let n = round % LatencyRun::SEEDED;
        let matching = EngineLatency::query(&json!({"n":{"$eq":n}}));
        root.collection("bench").find(&matching).expect("find");
    });
    run.measure("queue publish", |root, round| {
        let options = fylo::PublishOptions::default();
        let queue = root.queue("bench.jobs");
        queue
            .publish(json!({"round":round}), &options)
            .expect("publish");
    });
    run.measure_claim_and_ack();
    let total = LatencyRun::WARMUP + LatencyRun::ROUNDS;
    let doomed: Vec<String> = (0..total)
        .map(|round| {
            let document = EngineLatency::document(&json!({"doomed":round}));
            run.root.collection("bench").put(document).expect("put")
        })
        .collect();
    run.measure("document deleteById", |root, round| {
        root.collection("bench")
            .delete(&doomed[round])
            .expect("delete");
    });
    run.root.close().expect("close");
}
