use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{
    BuildOptions, DevServer, DevServerOptions, PreviewServer, PreviewServerOptions, WebCompiler,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

type Outcome<T = ()> = Result<T, Box<dyn Error>>;

const RUNTIME: &str = "/.tachyon/tac-client.js";
const STYLESHEET: &str = "/.tachyon/components.css";

fn write(path: &Path, contents: &str) -> Outcome {
    let parent = path.parent().ok_or("a source file has a parent folder")?;
    fs::create_dir_all(parent)?;
    fs::write(path, contents)?;
    Ok(())
}

fn write_pages(root: &Path) -> Outcome {
    write(&root.join("client/pages/tac.html"), "<offline-view />")?;
    write(
        &root.join("client/components/offline/view/tac.html"),
        "<main><h1 id=\"title\">Offline</h1></main>",
    )?;
    write(
        &root.join("client/components/offline/view/tac.css"),
        "#title { color: green }",
    )
}

fn write_controller(root: &Path) -> Outcome {
    write(
        &root.join("server/controllers/ItemsController.js"),
        r"@RequestRoute('/api/items/{id}')
@Public
export class ItemsController {
  @GetRoute
  get(@FromPath id) {
    return { id }
  }
}
",
    )
}

async fn exchange(port: u16, path: &str, headers: &str) -> Outcome<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
    let raw =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{headers}Connection: close\r\n\r\n");
    stream.write_all(raw.as_bytes()).await?;
    stream.flush().await?;
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), stream.read_to_end(&mut response)).await??;
    Ok(String::from_utf8_lossy(&response).into_owned())
}

fn header(response: &str, name: &str) -> Option<String> {
    let head = response.split("\r\n\r\n").next()?;
    head.lines().skip(1).find_map(|line| {
        let (field, value) = line.split_once(':')?;
        field
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_owned())
    })
}

fn status(response: &str) -> &str {
    response.split(' ').nth(1).unwrap_or_default()
}

fn assert_revalidated(response: &str, label: &str) {
    assert_eq!(status(response), "200", "{label}: {response}");
    assert_eq!(
        header(response, "cache-control").as_deref(),
        Some("no-cache"),
        "{label}"
    );
    let vary = header(response, "vary").unwrap_or_default();
    assert!(
        vary.is_empty() || vary.eq_ignore_ascii_case("accept-encoding"),
        "{label} varies only by encoding: {vary}"
    );
}

fn assert_not_stored(response: &str, label: &str) {
    assert_eq!(
        header(response, "cache-control").as_deref(),
        Some("no-store"),
        "{label}: {response}"
    );
}

async fn assert_listed_files_are_revalidated(port: u16) -> Outcome {
    for path in [RUNTIME, STYLESHEET, "/", "/index.html"] {
        assert_revalidated(&exchange(port, path, "").await?, path);
    }
    let compressed = exchange(port, RUNTIME, "Accept-Encoding: br\r\n").await?;
    assert_revalidated(&compressed, "a precompressed runtime");
    let tag = header(&compressed, "etag").ok_or("the runtime has an entity tag")?;
    let unchanged = exchange(
        port,
        RUNTIME,
        &format!("Accept-Encoding: br\r\nIf-None-Match: {tag}\r\n"),
    )
    .await?;
    assert_eq!(status(&unchanged), "304", "{unchanged}");
    assert_eq!(
        header(&unchanged, "cache-control").as_deref(),
        Some("no-cache")
    );
    Ok(())
}

async fn assert_private_or_unlisted_answers_are_not_stored(port: u16) -> Outcome {
    let refused = [
        (RUNTIME, "Authorization: Bearer test-only\r\n"),
        (RUNTIME, "Cookie: session=test-only\r\n"),
        ("/.tachyon/tac-client.js?build=2", ""),
        ("/tachyon-sw.js", ""),
        ("/.tachyon/build-state.json", ""),
        ("/missing.js", ""),
    ];
    for (path, headers) in refused {
        let label = format!("{path} {headers}");
        assert_not_stored(&exchange(port, path, headers).await?, &label);
    }
    Ok(())
}

async fn start_main_server(root: &Path, watch: bool) -> Outcome<(u16, oneshot::Sender<()>)> {
    let options = DevServerOptions {
        port: 0,
        watch,
        ..DevServerOptions::default()
    };
    let server = DevServer::bind(root, &options).await?;
    let port = server.address().port();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::spawn(server.run_until(async {
        let _stopped = stopped.await;
    }));
    Ok((port, stop))
}

async fn start_preview(bundle: &Path) -> Outcome<(u16, oneshot::Sender<()>)> {
    let options = PreviewServerOptions {
        port: 0,
        ..PreviewServerOptions::default()
    };
    let server = PreviewServer::bind(bundle, &options).await?;
    let port = server.address().port();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::spawn(server.run_until(async {
        let _stopped = stopped.await;
    }));
    Ok((port, stop))
}

#[tokio::test]
async fn the_main_server_lets_the_worker_keep_only_listed_public_build_files() -> Outcome {
    let root = tempfile::tempdir()?;
    write_pages(root.path())?;
    write_controller(root.path())?;
    let (port, stop) = start_main_server(root.path(), false).await?;
    assert_listed_files_are_revalidated(port).await?;
    assert_private_or_unlisted_answers_are_not_stored(port).await?;
    let api = exchange(port, "/api/items/7", "").await?;
    assert_eq!(status(&api), "200", "{api}");
    assert_not_stored(&api, "a controller answer");
    let _stopped = stop.send(());
    Ok(())
}

#[tokio::test]
async fn the_watching_development_server_stores_nothing() -> Outcome {
    let root = tempfile::tempdir()?;
    write_pages(root.path())?;
    let (port, stop) = start_main_server(root.path(), true).await?;
    for path in [RUNTIME, "/"] {
        assert_not_stored(&exchange(port, path, "").await?, path);
    }
    let _stopped = stop.send(());
    Ok(())
}

#[tokio::test]
async fn the_preview_server_follows_the_asset_table_of_the_current_bundle() -> Outcome {
    let root = tempfile::tempdir()?;
    write_pages(root.path())?;
    let built = WebCompiler::build_async(root.path(), &BuildOptions::default()).await?;
    let bundle = built.output_directory().to_path_buf();
    let (port, stop) = start_preview(&bundle).await?;
    assert_listed_files_are_revalidated(port).await?;
    assert_private_or_unlisted_answers_are_not_stored(port).await?;
    assert_not_stored(
        &exchange(port, "/later/", "").await?,
        "a page not built yet",
    );
    write(
        &root.path().join("client/pages/later/tac.html"),
        "<offline-view />",
    )?;
    WebCompiler::build_async(root.path(), &BuildOptions::default()).await?;
    assert_revalidated(
        &exchange(port, "/later/", "").await?,
        "a page added by a rebuild",
    );
    let _stopped = stop.send(());
    Ok(())
}
