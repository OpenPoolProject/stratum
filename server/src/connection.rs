use crate::{frame::Request, session::SendInformation, Error, Frame, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_util::sync::CancellationToken;
use tracing::trace;

#[derive(Debug)]
pub struct Connection {
    writer: OwnedWriteHalf,
    reader: BufReader<OwnedReadHalf>,
    cancel_token: CancellationToken,

    // The buffer for reading frames.
    buffer: BytesMut,
    pub(crate) address: SocketAddr,
}

impl Connection {
    pub(crate) fn new(socket: TcpStream, cancel_token: CancellationToken) -> Result<Self> {
        let addr = socket.peer_addr()?;

        let (read_half, write_half) = socket.into_split();

        Ok(Connection {
            address: addr,
            writer: write_half,
            reader: BufReader::new(read_half),
            cancel_token,
            buffer: BytesMut::new(),
        })
    }

    pub(crate) fn init(self) -> (ConnectionReader, UnboundedSender<SendInformation>) {
        let reader = ConnectionReader {
            reader: self.reader,
        };

        let (tx, rx): (
            UnboundedSender<SendInformation>,
            UnboundedReceiver<SendInformation>,
        ) = unbounded_channel();

        let cancel_token = self.cancel_token.child_token();
        tokio::spawn(async move { write_message(cancel_token, rx, self.writer).await });

        (reader, tx)
    }

    //@todo polish this up and support both v1 and v2.
    pub(crate) async fn proxy_protocol(&mut self) -> Result<SocketAddr> {
        let mut buf = String::new();

        // Check for Proxy Protocol.
        self.reader.read_line(&mut buf).await?;

        //Buf will be of the format "PROXY TCP4 92.118.161.17 172.20.42.228 55867 8080\r\n"
        //Trim the \r\n off
        let buf = buf.trim();
        //Might want to not be ascii whitespace and just normal here.
        // let pieces = buf.split_ascii_whitespace();

        let pieces: Vec<&str> = buf.split(' ').collect();

        Ok(format!("{}:{}", pieces[2], pieces[4]).parse()?)
    }
}

async fn write_message(
    cancel_token: CancellationToken,
    mut rx: UnboundedReceiver<SendInformation>,
    mut writer: OwnedWriteHalf,
) -> Result<()> {
    while !cancel_token.is_cancelled() {
        tokio::select! {
            Some(msg) = rx.recv() => {
                match msg {
                    SendInformation::Json(json) => {
                        writer.write_all(json.as_bytes()).await?;
                        writer.write_all(b"\n").await?;
                    }
                    SendInformation::Text(text) => {
                        writer.write_all(text.as_bytes()).await?;
                    }
                    SendInformation::Raw(buffer) => {
                        writer.write_all(&buffer).await?;
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                //Return Err
                    return Ok(());
            }
            else => {
            //Return Err
                return Ok(());
            }
        }
    }

    Ok(())
}

//@todo inhouse a buffer here, but for now this works I suppose.
pub struct ConnectionReader {
    reader: BufReader<OwnedReadHalf>,
}

impl ConnectionReader {
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            let mut buf = String::new();
            if 0 == self.reader.read_line(&mut buf).await? {
                if self.reader.buffer().is_empty() {
                    return Ok(None);
                }
                return Err(Error::PeerResetConnection);
            }

            if !buf.is_empty() {
                //@smells
                buf = buf.trim().to_owned();

                trace!("Received Message: {}", &buf);

                if buf.is_empty() {
                    continue;
                }

                let msg: Request = serde_json::from_str(&buf)?;

                return Ok(Some(Frame::V1(msg)));
            }
        }
    }
}
