use tachyon_contracts::HttpMethod;

pub(crate) struct RouteMethod;

impl RouteMethod {
    const ALL: [HttpMethod; 7] = [
        HttpMethod::Delete,
        HttpMethod::Get,
        HttpMethod::Head,
        HttpMethod::Options,
        HttpMethod::Patch,
        HttpMethod::Post,
        HttpMethod::Put,
    ];

    pub(crate) fn parse(name: &str) -> Option<HttpMethod> {
        Self::ALL
            .into_iter()
            .find(|method| Self::name(*method) == name)
    }

    pub(crate) const fn name(method: HttpMethod) -> &'static str {
        Self::text(method)
    }

    const fn text(method: HttpMethod) -> &'static str {
        match method {
            HttpMethod::Delete => "DELETE",
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
        }
    }
}
