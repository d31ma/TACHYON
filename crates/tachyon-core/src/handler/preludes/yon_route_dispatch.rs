pub trait YonRoutes {
    fn GET() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn POST() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn PUT() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn PATCH() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn DELETE() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn OPTIONS() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn HEAD() -> __YonAnswer {
        __YonAnswer::empty(405)
    }
    fn __yon_deliver() -> __YonAnswer {
        __YonAnswer::empty(404)
    }
}

impl YonRoutes for Handler {}

fn yon_dispatch(request: &__YonFrame) -> __YonAnswer {
    if request.parsed().get("operation").text() == "event.deliver" {
        return yon_entry(Handler::__yon_deliver, request);
    }
    match request.method().as_str() {
        "GET" => yon_entry(Handler::GET, request),
        "POST" => yon_entry(Handler::POST, request),
        "PUT" => yon_entry(Handler::PUT, request),
        "PATCH" => yon_entry(Handler::PATCH, request),
        "DELETE" => yon_entry(Handler::DELETE, request),
        "OPTIONS" => yon_entry(Handler::OPTIONS, request),
        "HEAD" => yon_entry(Handler::HEAD, request),
        _ => __YonAnswer::empty(405),
    }
}
