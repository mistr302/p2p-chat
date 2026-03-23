use p2pchat_types::api::{UiClientRequest, WriteEvent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

async fn read_write_event(
    sock_read: &mut (impl AsyncReadExt + Unpin),
) -> anyhow::Result<WriteEvent> {
    let len = sock_read.read_u64().await?;
    let mut buf = vec![0u8; len as usize];
    sock_read.read_exact(&mut buf).await?;
    let event: WriteEvent = postcard::from_bytes(&buf)?;
    Ok(event)
}

async fn send_request(
    sock_write: &mut (impl AsyncWriteExt + Unpin),
    request: &UiClientRequest,
) -> anyhow::Result<()> {
    let serialized = postcard::to_allocvec(request)?;
    sock_write.write_u64(serialized.len() as u64).await?;
    sock_write.write_all(&serialized).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stream = UnixStream::connect("/tmp/p2p-chat.sock").await?;
    let (mut sock_read, mut sock_write) = stream.into_split();

    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<UiClientRequest>();

    loop {
        tokio::select! {
            result = read_write_event(&mut sock_read) => {
                let event = result?;
                // TODO: handle incoming WriteEvent
                println!("{event:?}");
            }
            Some(request) = request_rx.recv() => {
                send_request(&mut sock_write, &request).await?;
            }
        }
    }
}
