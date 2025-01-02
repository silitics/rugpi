use std::{path::Path, process::Stdio, sync::Arc, time::Duration};

use async_trait::async_trait;
use reportify::{bail, whatever, ErrorExt, Report, ResultExt, Whatever};
use russh::{
    client::Handle,
    keys::{key::PrivateKeyWithHashAlg, load_secret_key, ssh_key, PrivateKey},
    ChannelMsg,
};
use russh_sftp::client::SftpSession;
use thiserror::Error;
use tokio::{
    fs,
    io::{self, AsyncWriteExt},
    process::{Child, Command},
    sync::{oneshot, Mutex},
    time,
};
use tracing::{error, info};

use super::{workflow::TestSystemConfig, RugpiTestResult};

pub struct Vm {
    #[expect(dead_code, reason = "not currently used")]
    child: Child,
    ssh_session: Mutex<Option<Handle<SshHandler>>>,
    sftp_session: Mutex<Option<SftpSession>>,
    #[expect(dead_code, reason = "not currently used")]
    vm_config: TestSystemConfig,
    private_key: Arc<PrivateKey>,
}

impl Vm {
    pub async fn run_script(
        &self,
        script: &str,
        stdin: Option<&Path>,
    ) -> Result<(), Report<SshError>> {
        if self.call(script, stdin).await.with_info(|_| "run script")? != 0 {
            bail!("script failed");
        }
        Ok(())
    }

    async fn call(&self, command: &str, stdin: Option<&Path>) -> Result<u32, Report<SshError>> {
        let mut channel = if let Some(ssh_session) = &mut *self.ssh_session.lock().await {
            ssh_session.channel_open_session().await?
        } else {
            bail!("no SSH session");
        };
        channel.exec(true, command).await?;

        let mut code = None;
        let mut stdout = tokio::io::stdout();
        let mut stderr = tokio::io::stderr();

        let (eof_tx, mut eof_rx) = oneshot::channel();

        if let Some(stdin) = stdin {
            let mut stdin_writer = channel.make_writer();
            let mut stdin_file = tokio::fs::File::open(stdin)
                .await
                .whatever("error opening stdin file")?;
            tokio::spawn(async move {
                if let Err(err) = tokio::io::copy(&mut stdin_file, &mut stdin_writer).await {
                    error!("error writing stdin {:?}", err.report::<io::Error>());
                }
                let _ = eof_tx.send(());
            });
        };

        let mut eof_sent = false;

        loop {
            tokio::select! {
                _ = (&mut eof_rx), if !eof_sent => {
                    channel.eof().await?;
                    eof_sent = true;
                }
                msg = channel.wait() => {
                    let Some(msg) = msg else {
                        break;
                    };
                    match msg {
                        ChannelMsg::ExtendedData { ref data, .. } => {
                            stderr
                                .write_all(data)
                                .await
                                .whatever("unable to write SSH stderr to terminal")?;
                            stderr.flush().await.whatever("unable to flush stderr")?;
                        }
                        ChannelMsg::Data { ref data } => {
                            stdout
                                .write_all(data)
                                .await
                                .whatever("unable to write SSH stdout to terminal")?;
                            stdout.flush().await.whatever("unable to flush stdout")?;
                        }
                        ChannelMsg::ExitStatus { exit_status } => {
                            code = Some(exit_status);
                        }
                        _ => {}
                    }
                }
            };
        }
        if let Some(code) = code {
            Ok(code)
        } else {
            bail!("program did not exit properly")
        }
    }
    pub async fn wait_for_ssh(&self) -> Result<(), Report<SshError>> {
        if let Some(ssh_session) = &*self.ssh_session.lock().await {
            if !ssh_session.is_closed() {
                return Ok(());
            }
        }
        let config = Arc::new(russh::client::Config::default());
        let key =
            PrivateKeyWithHashAlg::new(self.private_key.clone(), Some(ssh_key::HashAlg::Sha512))
                .whatever("unable to construct SSH key for SSH authentication")?;
        time::timeout(Duration::from_secs(120), async {
            loop {
                info!("trying to connect to VM via SSH");
                if let Ok(Ok(mut ssh_session)) = time::timeout(
                    Duration::from_secs(5),
                    russh::client::connect(config.clone(), ("127.0.0.1", 2233), SshHandler),
                )
                .await
                {
                    if !ssh_session
                        .authenticate_publickey("root", key)
                        .await
                        .whatever("unable to authenticate via SSH")?
                    {
                        bail!("unable to authenticate with the provided private key");
                    }
                    let channel = ssh_session.channel_open_session().await.unwrap();
                    channel.request_subsystem(true, "sftp").await.unwrap();
                    let sftp = SftpSession::new(channel.into_stream()).await.unwrap();
                    info!(
                        "current SFTP path: {:?}",
                        sftp.canonicalize(".").await.unwrap()
                    );
                    *self.ssh_session.lock().await = Some(ssh_session);
                    *self.sftp_session.lock().await = Some(sftp);
                    break;
                } else {
                    time::sleep(Duration::from_secs(4)).await
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| SshError::NotConnected.report())
        .and_then(|result| result)
    }
}

pub async fn start(image_file: &str, config: &TestSystemConfig) -> RugpiTestResult<Vm> {
    let private_key = load_secret_key(&config.ssh.private_key, None)
        .whatever("unable to load private SSH key")
        .with_info(|_| format!("path: {:?}", config.ssh.private_key))?;
    fs::create_dir_all(".rugpi/")
        .await
        .whatever("unable to create .rugpi directory")?;
    if !Command::new("qemu-img")
        .args(&["create", "-f", "qcow2", "-F", "raw", "-o"])
        .arg(format!("backing_file=../{}", image_file))
        .args(&[
            ".rugpi/vm-image.img",
            config.disk_size.as_deref().unwrap_or("40G"),
        ])
        .spawn()
        .whatever("unable to create VM image")?
        .wait()
        .await
        .whatever("unable to create VM image")?
        .success()
    {
        bail!("unable to create VM image");
    }
    let mut command = Command::new("qemu-system-aarch64");
    command.args(&[
        "-machine",
        "virt",
        "-cpu",
        "cortex-a72",
        "-m",
        "2G",
        "-smp",
        "cpus=2",
    ]);
    command.arg("-drive");
    command.arg("file=.rugpi/vm-image.img,format=qcow2,if=virtio");
    command.args(&["-device", "virtio-net-pci,netdev=net0", "-netdev"]);
    command.arg("user,id=net0,hostfwd=tcp:127.0.0.1:2233-:22");
    command.args(&[
        "-device",
        "virtio-rng-pci",
        "-bios",
        "/usr/share/AAVMF/AAVMF_CODE.fd",
        "-nographic",
        "-serial",
        "mon:stdio",
    ]);
    command
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = command.spawn().whatever("unable to spawn Qemu")?;
    if let Some(stdout) = Some("build/vm-stdout.log") {
        let mut stdout_log = fs::File::create(stdout)
            .await
            .whatever("unable to create stdout log file")?;
        let mut stdout = child.stdout.take().expect("we used Stdio::piped");
        tokio::spawn(async move { io::copy(&mut stdout, &mut stdout_log).await });
    }
    if let Some(stderr) = Some("build/vm-stderr.log") {
        let mut stderr_log = fs::File::create(stderr)
            .await
            .whatever("unable to create stderr log file")?;
        let mut stderr = child.stderr.take().expect("we used Stdio::piped");
        tokio::spawn(async move { io::copy(&mut stderr, &mut stderr_log).await });
    }
    // We give Qemu some time to start before checking it's exit status.
    time::sleep(Duration::from_millis(500)).await;
    if let Ok(Some(status)) = child.try_wait() {
        Err(whatever!("unable to start qemu")
            .with_info(format!("status: {}", status.code().unwrap_or(1))))
    } else {
        Ok(Vm {
            child,
            ssh_session: Mutex::default(),
            sftp_session: Mutex::default(),
            vm_config: config.clone(),
            private_key: Arc::new(private_key),
        })
    }
}

struct SshHandler;

#[derive(Debug, Error)]
pub enum SshError {
    #[error("internal SSH error")]
    Ssh(#[from] russh::Error),
    #[error("internal SFTP error")]
    Sftp(#[from] russh_sftp::client::error::Error),
    #[error("SSH session is not connected")]
    NotConnected,
    #[error("other SSH connection error")]
    Other,
}

impl Whatever for SshError {
    fn new() -> Self {
        Self::Other
    }
}

#[async_trait]
impl russh::client::Handler for SshHandler {
    type Error = SshError;

    async fn check_server_key(&mut self, _: &ssh_key::PublicKey) -> Result<bool, Self::Error> {
        // We do not care about the identity of the server.
        Ok(true)
    }
}
