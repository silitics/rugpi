use std::collections::VecDeque;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reportify::{bail, whatever, ErrorExt, Report, ResultExt, Whatever};
use rugpi_cli::style::Stylize;
use rugpi_cli::widgets::{Heading, Text, Widget};
use rugpi_cli::{StatusSegment, VisualHeight};
use russh::client::Handle;
use russh::keys::key::PrivateKeyWithHashAlg;
use russh::keys::{load_secret_key, ssh_key, PrivateKey};
use russh::ChannelMsg;
use russh_sftp::client::SftpSession;
use thiserror::Error;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};
use tokio::{fs, time};
use tracing::{error, info};

use crate::project::config::Architecture;

use super::workflow::TestSystemConfig;
use super::{RugpiTestResult, TestCtx};

pub struct Vm {
    #[expect(dead_code, reason = "not currently used")]
    child: Child,
    ssh_session: Mutex<Option<Handle<SshHandler>>>,
    sftp_session: Mutex<Option<SftpSession>>,
    #[expect(dead_code, reason = "not currently used")]
    vm_config: TestSystemConfig,
    private_key: Arc<PrivateKey>,
}

#[derive(Debug, Default)]
struct LineSplitter {
    buffer: Vec<u8>,
}

impl LineSplitter {
    pub fn continue_splitting<'splitter, 'bytes>(
        &'splitter mut self,
        bytes: &'bytes [u8],
    ) -> impl 'bytes + Iterator<Item = String>
    where
        'splitter: 'bytes,
    {
        bytes.iter().filter_map(|b| {
            if *b == b'\n' {
                let line = String::from_utf8_lossy(&self.buffer).into_owned();
                self.buffer.clear();
                Some(line)
            } else {
                self.buffer.push(*b);
                None
            }
        })
    }
}

impl Vm {
    pub async fn run_script(
        &self,
        ctx: &TestCtx,
        script: &str,
        stdin: Option<&Path>,
    ) -> Result<(), Report<ExecError>> {
        let Some(sftp_session) = &*self.sftp_session.lock().await else {
            bail!("no SFTP session");
        };
        let mut test_script = sftp_session
            .create("/tmp/rugpi-test-script.sh")
            .await
            .whatever("unable to create `rugpi-test-script.sh`")?;
        test_script
            .write_all(script.as_bytes())
            .await
            .whatever("unable to write to `rugpi-test-script.sh`")?;
        test_script
            .sync_all()
            .await
            .whatever("error syncing `rugpi-test-script.sh`")?;
        drop(test_script);

        self.call(
            ctx,
            "chmod +x /tmp/rugpi-test-script.sh\n/tmp/rugpi-test-script.sh",
            stdin,
        )
        .await
        .with_info(|_| "run script")
    }

    async fn call(
        &self,
        ctx: &TestCtx,
        command: &str,
        stdin: Option<&Path>,
    ) -> Result<(), Report<ExecError>> {
        let mut channel = if let Some(ssh_session) = &mut *self.ssh_session.lock().await {
            ssh_session
                .channel_open_session()
                .await
                .whatever("unable to open SSH channel")?
        } else {
            bail!("no SSH session");
        };
        channel
            .exec(true, command)
            .await
            .whatever("unable to execute command")?;

        let mut code = None;

        let (eof_tx, mut eof_rx) = oneshot::channel();

        if let Some(stdin) = stdin {
            let mut stdin_writer = channel.make_writer();
            let mut stdin_file = tokio::fs::File::open(stdin)
                .await
                .whatever("error opening stdin file")?;
            let stdin_length = stdin
                .metadata()
                .whatever("unable to load `stdin` metadata")?
                .size();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                let mut buffer = Vec::with_capacity(8096);
                let mut bytes_written = 0;
                while let Ok(read) = stdin_file.read_buf(&mut buffer).await {
                    if read == 0 {
                        break;
                    }
                    let _ = stdin_writer.write_all(&buffer[..read]).await;
                    buffer.clear();
                    bytes_written += read as u64;
                    let mut state = ctx.status.state.lock().unwrap();
                    state.step_progress = Some(super::StepProgress {
                        message: "sending `stdin-file`",
                        position: bytes_written,
                        length: stdin_length,
                    });
                }
                let mut state = ctx.status.state.lock().unwrap();
                state.step_progress = None;
                let _ = eof_tx.send(());
            });
        };

        let mut eof_sent = false;

        let mut stderr_splitter = LineSplitter::default();
        let mut stdout_splitter = LineSplitter::default();

        loop {
            tokio::select! {
                _ = (&mut eof_rx), if !eof_sent => {
                    let _ = channel.eof().await;
                    eof_sent = true;
                }
                msg = channel.wait() => {
                    let Some(msg) = msg else {
                        break;
                    };
                    match msg {
                        ChannelMsg::ExtendedData { ref data, .. } => {
                            for line in stderr_splitter.continue_splitting(data) {
                                ctx.status.push_log_line(line);
                            }
                            // stderr
                            //     .write_all(data)
                            //     .await
                            //     .whatever("unable to write SSH stderr to terminal")?;
                            // stderr.flush().await.whatever("unable to flush stderr")?;
                        }
                        ChannelMsg::Data { ref data } => {
                            for line in stdout_splitter.continue_splitting(data) {
                                ctx.status.push_log_line(line);
                            }
                            // stdout
                            //     .write_all(data)
                            //     .await
                            //     .whatever("unable to write SSH stdout to terminal")?;
                            // stdout.flush().await.whatever("unable to flush stdout")?;
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
            if code == 0 {
                Ok(())
            } else {
                Err(ExecError::Failed { code }.report())
            }
        } else {
            Err(ExecError::Disconnected.report())
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

pub async fn start(
    arch: Architecture,
    image_file: &str,
    config: &TestSystemConfig,
) -> RugpiTestResult<Vm> {
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
    let mut command = match arch {
        Architecture::Amd64 => {
            let mut command = Command::new("qemu-system-x86_64");
            command.args(&["-machine", "pc", "-m", "2G", "-smp", "cpus=2"]);
            command
        }
        Architecture::Arm64 => {
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
            command
        }
        _ => bail!("unsupported architecture {arch}"),
    };
    command.arg("-drive");
    command.arg("file=.rugpi/vm-image.img,format=qcow2,if=virtio");
    command.args(&["-device", "virtio-net-pci,netdev=net0", "-netdev"]);
    command.arg("user,id=net0,hostfwd=tcp:127.0.0.1:2233-:22");
    let efi_code = match arch {
        Architecture::Amd64 => "/usr/share/OVMF/OVMF_CODE.fd",
        Architecture::Arm64 => "/usr/share/AAVMF/AAVMF_CODE.fd",
        _ => bail!("unsupported architecture {arch}"),
    };
    command.args(&[
        "-device",
        "virtio-rng-pci",
        "-bios",
        efi_code,
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
        tokio::spawn(async move {
            let log = rugpi_cli::add_status(VmLog::default());
            let mut line_buffer = Vec::new();
            let mut buffer = Vec::with_capacity(8096);
            while let Ok(read) = stdout.read_buf(&mut buffer).await {
                if read == 0 {
                    break;
                }
                let _ = stdout_log.write_all(&buffer[..read]).await;
                for b in &buffer[..read] {
                    if *b == '\n' as u8 {
                        log.push_line(String::from_utf8_lossy(&line_buffer).into_owned());
                        line_buffer.clear();
                    } else {
                        line_buffer.push(*b);
                    }
                }
                buffer.clear();
            }
            // io::copy(&mut stdout, &mut stdout_log).await
        });
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
pub enum ExecError {
    #[error("SSH client disconnected while executing script")]
    Disconnected,
    #[error("script execution failed with non-zero return code {code}")]
    Failed { code: u32 },
    #[error("script execution failed")]
    Other,
}

impl Whatever for ExecError {
    fn new() -> Self {
        Self::Other
    }
}

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

#[derive(Debug, Default)]
struct VmLog {
    state: std::sync::Mutex<VmLogState>,
}

impl VmLog {
    fn push_line(&self, line: String) {
        let mut state = self.state.lock().unwrap();
        state.lines.push_back(line);
        while state.lines.len() > 15 {
            state.lines.pop_front();
        }
    }
}

#[derive(Debug, Default)]
struct VmLogState {
    lines: VecDeque<String>,
}

impl StatusSegment for VmLog {
    fn draw(&self, ctx: &mut rugpi_cli::DrawCtx) {
        Heading::new("VM Output").draw(ctx);
        let state = self.state.lock().unwrap();
        let show_lines = VisualHeight::from_usize(state.lines.len())
            .min(ctx.measure_remaining_height())
            .into_u64() as usize;
        let skip_lines = state.lines.len() - show_lines;
        Text::new(state.lines.iter().skip(skip_lines))
            .prefix("> ")
            .styled()
            .dark_gray()
            .draw(ctx);
    }
}
