pub struct BehaviorExchange<'a> {
    context: &'a BehaviorContext,
    operation: BehaviorOperation<'a>,
    policy: BehaviorMethodPolicy,
    args: Vec<BehaviorValue>,
}

type BehaviorPendingAnswer<'b> =
    core::pin::Pin<Box<dyn core::future::Future<Output = BehaviorResult<BehaviorValue>> + 'b>>;
type BehaviorCacheChange<'a> = fn(
    &BehaviorExchange<'a>,
    &str,
    &std::sync::Arc<dyn BehaviorCacheAdapter>,
    &BehaviorValue,
) -> BehaviorResult<()>;

impl<'a> BehaviorExchange<'a> {
    const LARGEST_ANSWER: usize = 1_048_576;
    const MOST_ARGUMENTS: usize = 32;
    const READ: &'static str = "cache:read";
    const PUT: &'static str = "cache:put";
    const EVICT: &'static str = "cache:evict";

    pub async fn plain(
        context: &'a BehaviorContext,
        policy: BehaviorMethodPolicy,
        input: Vec<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let exchange = Self::open(context, policy, input)?;
        let value = exchange.send((false, false)).await?;
        exchange.finish(value)
    }

    pub async fn cached(
        context: &'a BehaviorContext,
        policy: BehaviorMethodPolicy,
        input: Vec<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let exchange = Self::open(context, policy, input)?;
        let key = exchange.read_key()?;
        let adapter = exchange.cache_adapter()?;
        let fresh = exchange.fresh(&key, &adapter)?;
        exchange.answer_or_send(fresh, &key, &adapter).await
    }

    pub async fn mutated(
        context: &'a BehaviorContext,
        policy: BehaviorMethodPolicy,
        input: Vec<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let exchange = Self::open(context, policy, input)?;
        let mutation = exchange.mutation()?;
        let key = exchange.target_key(&mutation)?;
        let adapter = exchange.cache_adapter()?;
        let value = exchange.send((false, false)).await?;
        let value = value.checked(exchange.mutation_limit(&mutation)?)?;
        exchange.operation.execution.check()?;
        exchange.change_cache(&mutation)(&exchange, &key, &adapter, &value)?;
        exchange.mark_revalidation(key);
        Ok(value)
    }

    fn open(
        context: &'a BehaviorContext,
        policy: BehaviorMethodPolicy,
        input: Vec<BehaviorValue>,
    ) -> BehaviorResult<Self> {
        let operation = context.operation()?;
        let args = behavior_arguments(input, &policy)?;
        Ok(Self {
            context,
            operation,
            policy,
            args,
        })
    }

    async fn answer_or_send(
        &self,
        fresh: Option<BehaviorValue>,
        key: &str,
        adapter: &std::sync::Arc<dyn BehaviorCacheAdapter>,
    ) -> BehaviorResult<BehaviorValue> {
        if let Some(value) = fresh {
            return self.finish(value);
        }
        let revalidate = self.take_revalidation(key);
        let value = self.send((true, revalidate)).await?;
        let value = self.store(key, adapter, value)?;
        self.finish(value)
    }

    async fn send(&self, reading: (bool, bool)) -> BehaviorResult<BehaviorValue> {
        let http = self.policy.http.as_ref().ok_or_else(behavior_invalid)?;
        let descriptor = Self::declares(http)
            .then(|| behavior_exchange_request(http, &self.args))
            .unwrap_or_else(|| Ok(BehaviorValue::object([] as [(&str, BehaviorValue); 0])))?;
        let mut request = behavior_http_request(http, descriptor)?;
        request.path = behavior_exchange_path(http, &self.args)?;
        let answer = self.deliver(http.client, &request, reading).await?;
        answer.checked(Self::LARGEST_ANSWER)
    }

    fn deliver<'b>(
        &'b self,
        client: &str,
        request: &'b BehaviorHttpRequest,
        reading: (bool, bool),
    ) -> BehaviorPendingAnswer<'b> {
        self.context.0.configuration.http.get(client).map_or_else(
            || -> BehaviorPendingAnswer<'b> {
                Box::pin(BehaviorBrowserHttp::send(request, &self.operation.execution, reading))
            },
            |adapter| -> BehaviorPendingAnswer<'b> {
                let answer = self
                    .operation
                    .adapter(|| adapter.exchange(request, &self.operation.execution));
                Box::pin(core::future::ready(answer))
            },
        )
    }

    fn declares(http: &BehaviorHttpPolicy) -> bool {
        [
            !http.query.is_empty(),
            !http.headers.is_empty(),
            http.body.is_some(),
            !http.path_variables.is_empty(),
        ]
        .contains(&true)
    }

    fn finish(&self, value: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        self.operation.execution.check()?;
        Ok(value)
    }

    fn cache(&self) -> BehaviorResult<&BehaviorCachePolicy> {
        let cache = self.policy.cache.as_ref().ok_or_else(behavior_invalid)?;
        behavior_validate_cache(cache)?;
        Ok(cache)
    }

    fn cache_adapter(&self) -> BehaviorResult<std::sync::Arc<dyn BehaviorCacheAdapter>> {
        self.context.cache_adapter(self.cache()?.name)
    }

    fn read_key(&self) -> BehaviorResult<String> {
        self.cache()?;
        let indices = self.policy.key_args.as_ref().ok_or_else(behavior_invalid)?;
        (!behavior_http_method(self.policy.name))
            .then_some(())
            .ok_or_else(behavior_invalid)?;
        let selected = behavior_selected(indices, &self.args)?;
        self.context
            .authorize(&self.operation, &self.policy, selected.clone(), Self::READ)?;
        self.context.cache_key(&self.policy, selected)
    }

    fn fresh(
        &self,
        key: &str,
        adapter: &std::sync::Arc<dyn BehaviorCacheAdapter>,
    ) -> BehaviorResult<Option<BehaviorValue>> {
        let entry = self
            .operation
            .adapter(|| adapter.get(key, &self.operation.execution))?;
        behavior_fresh_entry(entry, self.cache()?)
    }

    fn store(
        &self,
        key: &str,
        adapter: &std::sync::Arc<dyn BehaviorCacheAdapter>,
        value: BehaviorValue,
    ) -> BehaviorResult<BehaviorValue> {
        let value = value.checked(self.cache()?.max_bytes)?;
        self.put(key, adapter, &value)?;
        Ok(value)
    }

    fn put(
        &self,
        key: &str,
        adapter: &std::sync::Arc<dyn BehaviorCacheAdapter>,
        value: &BehaviorValue,
    ) -> BehaviorResult<()> {
        let cache = self.cache()?;
        let write = BehaviorCacheWrite {
            execution: self.operation.execution.clone(),
            policy: cache.clone(),
            expires_at: behavior_now()?.saturating_add(cache.ttl_ms),
        };
        self.operation
            .adapter(|| adapter.set(key, value.clone(), &write))
    }

    fn delete(
        &self,
        key: &str,
        adapter: &std::sync::Arc<dyn BehaviorCacheAdapter>,
        _value: &BehaviorValue,
    ) -> BehaviorResult<()> {
        self.operation
            .adapter(|| adapter.delete(key, &self.operation.execution))
    }

    fn mutation(&self) -> BehaviorResult<BehaviorCacheMutationPolicy> {
        let mutation = self.policy.mutation.clone().ok_or_else(behavior_invalid)?;
        let refused = [
            self.policy.key_args.is_some(),
            ![Self::PUT, Self::EVICT].contains(&mutation.operation),
            !behavior_identifier(mutation.method),
            behavior_http_method(self.policy.name),
            mutation.arguments.len() > Self::MOST_ARGUMENTS,
        ]
        .contains(&true);
        (!refused).then_some(mutation).ok_or_else(behavior_invalid)
    }

    fn target_key(&self, mutation: &BehaviorCacheMutationPolicy) -> BehaviorResult<String> {
        self.cache()?;
        let target = behavior_mutation_target(&self.policy, mutation);
        let mapped = behavior_mutation_arguments(mutation, &self.args)?;
        let target_args = behavior_arguments(mapped, &target)?;
        let selected = behavior_selected(&mutation.key_args, &target_args)?;
        self.context
            .authorize(&self.operation, &target, selected.clone(), mutation.operation)?;
        self.context.cache_key(&target, selected)
    }

    fn mutation_limit(&self, mutation: &BehaviorCacheMutationPolicy) -> BehaviorResult<usize> {
        let limits = [Self::LARGEST_ANSWER, self.cache()?.max_bytes];
        Ok(limits[usize::from(mutation.operation == Self::PUT)])
    }

    fn change_cache(&self, mutation: &BehaviorCacheMutationPolicy) -> BehaviorCacheChange<'a> {
        let changes: [BehaviorCacheChange<'a>; 2] = [Self::delete, Self::put];
        changes[usize::from(mutation.operation == Self::PUT)]
    }

    fn take_revalidation(&self, key: &str) -> bool {
        self.context
            .0
            .revalidate
            .lock()
            .is_ok_and(|mut keys| keys.remove(key))
    }

    fn mark_revalidation(&self, key: String) {
        let _ = self
            .context
            .0
            .revalidate
            .lock()
            .map(|mut keys| keys.insert(key));
    }
}
