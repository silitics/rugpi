use std::{fs, ops::Deref};

use axum::{
    extract::{DefaultBodyLimit, Multipart},
    response::Html,
    routing::{get, post},
    Router, Server,
};
use clap::Parser;
#[cfg(not(debug_assertions))]
use rugpi_common::partitions::{get_default_partitions, get_hot_partitions};
use tokio::io::AsyncWriteExt;
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
    tokio::task::spawn_blocking(|| {
        let hot_partitions = get_hot_partitions().unwrap();
        let default_partitions = get_default_partitions().unwrap();
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

    #[cfg(debug_assertions)]
    const DOWNLOAD_DIR: &str = "tmp";
    #[cfg(debug_assertions)]
    const IMAGE_FILE: &str = "tmp/image.img";
    #[cfg(not(debug_assertions))]
    const DOWNLOAD_DIR: &str = "/var/rugpi/admin/tmp";
    #[cfg(not(debug_assertions))]
    const IMAGE_FILE: &str = "/var/rugpi/admin/tmp/image.img";

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
                fs::create_dir_all(DOWNLOAD_DIR).unwrap();
                let mut image_file =
                    tokio::io::BufWriter::new(tokio::fs::File::create(IMAGE_FILE).await.unwrap());
                while let Some(chunk) = field.chunk().await.unwrap() {
                    image_file.write(chunk.deref()).await.unwrap();
                }
                image_file.flush().await.unwrap();
                drop(image_file);
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
        Action::Update => {
            tokio::task::spawn_blocking(|| {
                run!(["rugpi-ctrl", "update", "install", IMAGE_FILE]).unwrap()
            })
            .await
            .unwrap();
        }
    }
    render_index_html().await
}
