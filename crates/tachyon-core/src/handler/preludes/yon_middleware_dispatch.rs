pub trait YonMiddleware {
    fn before() -> __YonAnswer {
        __YonAnswer::empty(204)
    }
    fn after() -> __YonAnswer {
        __YonAnswer::empty(204)
    }
}

impl YonMiddleware for Middleware {}

fn yon_dispatch(request: &__YonFrame) -> __YonAnswer {
    match request.parsed.get("operation").text().as_str() {
        "middleware.before" => yon_entry(Middleware::before, request),
        "middleware.after" => yon_entry(Middleware::after, request),
        _ => __YonAnswer::empty(405),
    }
}
