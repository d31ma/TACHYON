use super::namespace_name::NamespaceName;
use crate::Failure;
use crate::storage::failure;

pub(super) struct FyloLocation<'url> {
    path: &'url str,
    namespace: Option<&'url str>,
}

impl<'url> FyloLocation<'url> {
    const SCHEME: &'static str = "fylo://";
    const NAMESPACE_KEY: &'static str = "namespace=";
    const FORM: &'static str = "FYLO datasource URL is fylo:///absolute/root, optionally with ?namespace=<name>.";

    pub(super) fn parsed(raw: &'url str) -> Result<Self, Failure> {
        let (address, query) = Self::split_query(raw);
        let path = Self::root_path(address).ok_or_else(|| failure(Self::FORM))?;
        let namespace = Self::namespace_of(query)?;
        Ok(Self { path, namespace })
    }

    pub(super) fn path(&self) -> &'url str {
        self.path
    }

    pub(super) fn namespace(&self) -> Option<&'url str> {
        self.namespace
    }

    fn split_query(raw: &str) -> (&str, Option<&str>) {
        match raw.split_once('?') {
            Some((address, query)) => (address, Some(query)),
            None => (raw, None),
        }
    }

    fn root_path(address: &str) -> Option<&str> {
        let path = address
            .strip_prefix(Self::SCHEME)
            .filter(|rest| rest.starts_with('/'))?;
        let windows_drive = [cfg!(windows), path.get(2..3) == Some(":")] == [true, true];
        path.get(usize::from(windows_drive)..)
    }

    fn namespace_of(query: Option<&str>) -> Result<Option<&str>, Failure> {
        query.map(Self::named_namespace).transpose()
    }

    fn named_namespace(query: &str) -> Result<&str, Failure> {
        let name = query
            .strip_prefix(Self::NAMESPACE_KEY)
            .ok_or_else(|| failure(Self::FORM))?;
        NamespaceName::checked(name)
    }
}

#[cfg(test)]
mod tests {
    use super::FyloLocation;

    #[test]
    fn a_url_without_a_query_names_only_the_root() {
        let location = FyloLocation::parsed("fylo:///srv/catalog").ok();
        assert_eq!(location.as_ref().map(FyloLocation::path), Some("/srv/catalog"));
        assert_eq!(location.and_then(|location| location.namespace()), None);
    }

    #[test]
    fn a_namespace_query_names_the_namespace() {
        let location = FyloLocation::parsed("fylo:///srv/catalog?namespace=slice-12").ok();
        assert_eq!(location.as_ref().map(FyloLocation::path), Some("/srv/catalog"));
        assert_eq!(location.and_then(|location| location.namespace()), Some("slice-12"));
    }

    #[test]
    fn another_query_key_or_a_bad_namespace_is_refused() {
        for url in [
            "fylo:///srv/catalog?ns=a1",
            "fylo:///srv/catalog?namespace=",
            "fylo:///srv/catalog?namespace=Slice",
            "fylo:///srv/catalog?namespace=a1&x=1",
            "fylo:///srv/catalog?namespace=con",
            "fylo://relative/catalog",
        ] {
            assert!(FyloLocation::parsed(url).is_err(), "{url}");
        }
    }
}
