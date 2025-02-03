use std::process::Stdio;

use axum::extract::{DefaultBodyLimit, Multipart};
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Router, Server};
use clap::Parser;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
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
    // use crate::system::System;

    // tokio::task::spawn_blocking(|| {
    //     let system = System::initialize().unwrap();
    //     let default_entry = system.boot_flow().get_default(&system).unwrap();
    //     let active_entry = system.active_boot_entry().unwrap();
    //     Html(
    //         include_str!("../assets/index.html")
    //             .replace(
    //                 "HOT_PARTITIONS",
    //                 &system.boot_entries()[active_entry].name().to_uppercase(),
    //             )
    //             .replace(
    //                 "DEFAULT_PARTITIONS",
    //                 &system.boot_entries()[default_entry].name().to_uppercase(),
    //             ),
    //     )
    // })
    // .await
    // .unwrap()
    todo!()
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
                let mut command = Command::new("rugix-ctrl")
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
            tokio::task::spawn_blocking(|| run!(["rugix-ctrl", "state", "reset"]).unwrap())
                .await
                .unwrap();
        }
        Action::Commit => {
            tokio::task::spawn_blocking(|| run!(["rugix-ctrl", "system", "commit"]).unwrap())
                .await
                .unwrap();
        }
        Action::Reboot => {
            tokio::task::spawn_blocking(|| run!(["rugix-ctrl", "system", "reboot"]).unwrap())
                .await
                .unwrap();
        }
        Action::RebootSpare => {
            tokio::task::spawn_blocking(|| {
                run!(["rugix-ctrl", "system", "reboot", "--spare"]).unwrap()
            })
            .await
            .unwrap();
        }
        Action::Update => { /* do nothing */ }
    }
    render_index_html().await
}
