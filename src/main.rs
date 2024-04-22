use std::ffi::{OsStr, OsString};

use clap::Parser;
use compio::{
    buf::{IntoInner, IoBuf},
    fs::named_pipe::ClientOptions,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    signal::ctrl_c,
    BufResult,
};
use futures_util::FutureExt;
use ipnet::IpNet;

/// Simple program to forward a named pipe to TCP.
#[derive(Debug, Parser)]
#[command(version, about, author)]
struct Args {
    /// Path of named pipe, start with `\\.\pipe\`.
    #[arg(long)]
    pipe: OsString,
    /// Listen host.
    #[arg(short = 'c', long)]
    host: String,
    /// Listen port.
    #[arg(short, long)]
    port: u16,
    /// Listen IP range.
    #[arg(short, long)]
    filter: Option<IpNet>,
}

#[compio::main]
async fn main() {
    let args = Args::parse();

    let server = TcpListener::bind((args.host, args.port))
        .await
        .expect("cannot bind address");
    loop {
        futures_util::select! {
            _ = serve(&server, &args.pipe, &args.filter).fuse() => {},
            _ = ctrl_c().fuse() => break,
        }
    }
}

async fn copy_io(mut src: impl AsyncRead, mut target: impl AsyncWrite) {
    let mut buffer = Box::new([0u8; 4096]);
    loop {
        let len;
        (len, buffer) = src.read(buffer).await.expect("cannot read source");
        if len == 0 {
            break;
        }
        let BufResult(res, slice) = target.write_all(buffer.slice(..len)).await;
        match res {
            Err(e) if e.kind() == std::io::ErrorKind::WriteZero => break,
            _ => res.expect("cannot write target"),
        }
        buffer = slice.into_inner();
        target.flush().await.expect("cannot flush target");
    }
}

async fn serve(server: &TcpListener, path: &OsStr, accept_range: &Option<IpNet>) {
    let (client, addr) = server.accept().await.expect("fail to accept client");
    let accept = if let Some(accept_range) = accept_range {
        accept_range.contains(&addr.ip())
    } else {
        true
    };
    if accept {
        let pipe_client = ClientOptions::new()
            .open(path)
            .await
            .expect("cannot open ssh-agent named pipe");
        compio::runtime::spawn(async move {
            let read_task = copy_io(&client, &pipe_client);
            let write_task = copy_io(&pipe_client, &client);

            futures_util::join!(read_task, write_task);
            client.close().await.expect("cannot shutdown client");
        })
        .detach();
    }
}
