use std::process::Stdio;

use axum::{
    extract::{DefaultBodyLimit, Multipart},
    response::Html,
    routing::{get, post},
    Router, Server,
};
use clap::Parser;
#[cfg(not(debug_assertions))]
use rugpi_common::partitions::{get_hot_partitions, read_default_partitions};
use tokio::{io::AsyncWriteExt, process::Command};
#[cfg(not(debug_assertions))]
use xscript::{run, Run};

#[derive(Debug, Clone, Parser)]
pub struct Args {
    /// The address to bind to [default: 0.0.0.0:8088].
    #[clap(long)]
    pub address: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let address = args.address.as_deref().unwrap_or("0.0.0.0:8088");

    let app = Router::new()
        .route("/", get(get_index))
        .route("/", post(post_index))
        .layer(DefaultBodyLimit::disable());

    Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap()
}

#[cfg(not(debug_assertions))]
async fn render_index_html() -> Html<String> {
    use rugpi_common::partitions::Partitions;

    tokio::task::spawn_blocking(|| {
        let partitions = Partitions::load().unwrap();
        let hot_partitions = get_hot_partitions(&partitions).unwrap();
        let default_partitions = read_default_partitions().unwrap();
        Html(
            include_str!("../assets/index.html")
                .replace("HOT_PARTITIONS", &hot_partitions.as_str().to_uppercase())
                .replace(
                    "DEFAULT_PARTITIONS",
                    &default_partitions.as_str().to_uppercase(),
                ),
        )
    })
    .await
    .unwrap()
}

#[cfg(debug_assertions)]
async fn render_index_html() -> Html<String> {
    Html(include_str!("../assets/index.html").to_owned())
}

async fn get_index() -> Html<String> {
    render_index_html().await
}

async fn post_index(mut multipart: Multipart) -> Html<String> {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Action {
        Reset,
        Commit,
        Reboot,
        RebootSpare,
        Update,
    }

    let mut action = None;

    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_owned();
        match name.as_str() {
            "action" => {
                let value = field.text().await.unwrap();
                match value.as_str() {
                    "reset" => {
                        action = Some(Action::Reset);
                    }
                    "commit" => {
                        action = Some(Action::Commit);
                    }
                    "update" => {
                        action = Some(Action::Update);
                    }
                    "reboot" => {
                        action = Some(Action::Reboot);
                    }
                    "reboot-spare" => {
                        action = Some(Action::RebootSpare);
                    }
                    _ => {
                        panic!("invalid action `{value}`");
                    }
                }
            }
            "image" => {
                let mut command = Command::new("rugpi-ctrl")
                    .args(["update", "install", "--stream", "-"])
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap();
                let mut stdin = command.stdin.take().unwrap();
                while let Some(chunk) = field.chunk().await.unwrap() {
                    stdin.write_all(&chunk).await.unwrap();
                }
                command.wait().await.unwrap();
            }
            _ => {
                panic!("invalid field name `{name}`");
            }
        }
    }
    let action = action.expect("no action provided");
    #[cfg(debug_assertions)]
    let _ = action;
    #[cfg(not(debug_assertions))]
    match action {
        Action::Reset => {
            tokio::task::spawn_blocking(|| run!(["rugpi-ctrl", "state", "reset"]).unwrap())
                .await
                .unwrap();
        }
        Action::Commit => {
            tokio::task::spawn_blocking(|| run!(["rugpi-ctrl", "system", "commit"]).unwrap())
                .await
                .unwrap();
        }
        Action::Reboot => {
            tokio::task::spawn_blocking(|| run!(["rugpi-ctrl", "system", "reboot"]).unwrap())
                .await
                .unwrap();
        }
        Action::RebootSpare => {
            tokio::task::spawn_blocking(|| {
                run!(["rugpi-ctrl", "system", "reboot", "--spare"]).unwrap()
            })
            .await
            .unwrap();
        }
        Action::Update => { /* do nothing */ }
    }
    render_index_html().await
}
