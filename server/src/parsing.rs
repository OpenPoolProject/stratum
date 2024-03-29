pub use crate::SessionList;
use crate::{Error, Result, EX_MAGIC_NUMBER};
use extended_primitives::Buffer;
use serde_json::{Map, Value};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    net::tcp::OwnedReadHalf,
};
use tracing::trace;

// pub async fn proxy_protocol(
//     //@todo try using non-owned Read Half we fucked it up w/ generics so might work now.
//     buffer_stream: &mut BufReader<OwnedReadHalf>,
//     expected_port: u16,
// ) -> Result<SocketAddr> {
//     let mut buf = String::new();
//
//     buffer_stream.read_line(&mut buf).await.unwrap();
//
//     //Buf will be of the format "PROXY TCP4 92.118.161.17 172.20.42.228 55867 8080\r\n"
//     //Trim the \r\n off
//     let buf = buf.trim();
//     //Might want to not be ascii whitespace and just normal here.
//     // let pieces = buf.split_ascii_whitespace();
//
//     let pieces: Vec<&str> = buf.split(' ').collect();
//
//     let attempted_port: u16 = pieces[5].parse().unwrap();
//
//     //Check that they were trying to connect to us.
//     if attempted_port != expected_port {
//         return Err(Error::StreamWrongPort);
//     }
//
//     Ok(format!("{}:{}", pieces[2], pieces[4]).parse()?)
// }

//@todo feature gate the EXMESSAGE and MAGIC stuff.
// pub async fn next_message<T>(stream: &mut T) -> Result<(String, MessageValue)>
// where
//     T: AsyncBufReadExt + Unpin,
// {
//     //I don't actually think this has to loop here.
//     loop {
//         let peak = stream.fill_buf().await?;
//
//         if peak.is_empty() {
//             return Err(Error::StreamClosed(String::from(
//                 "ExMessage peak was empty.",
//             )));
//         }
//
//         if peak[0] == EX_MAGIC_NUMBER {
//             let mut header_bytes = vec![0u8; 4];
//             stream.read_exact(&mut header_bytes).await?;
//             let mut header_buffer = Buffer::from(header_bytes);
//             let mut saved_header_buffer = header_buffer.clone();
//
//             let _magic_number = header_buffer.read_u8().map_err(|_| Error::BrokenExHeader)?;
//             let _cmd = header_buffer.read_u8().map_err(|_| Error::BrokenExHeader)?;
//             let length = header_buffer
//                 .read_u16()
//                 .map_err(|_| Error::BrokenExHeader)?;
//
//             let mut buf = vec![0u8; length as usize - 4];
//             stream.read_exact(&mut buf).await?;
//
//             let buffer = Buffer::from(buf);
//
//             //Add the new buffer body (buffer) to the header_bytes that we had previously saved.
//             saved_header_buffer.extend(buffer);
//
//             let ex_message = ExMessageGeneric::from_buffer(&mut saved_header_buffer)?;
//             return Ok((
//                 ex_message.cmd.to_string(),
//                 MessageValue::ExMessage(ex_message),
//             ));
//         }
//
//         //If we have reached here, then we did not breat the "Peak test" searching for the magic
//         //number of ExMessage.
//
//         //@todo let's break this into 2 separate functions eh?
//         let mut buf = String::new();
//         let num_bytes = stream.read_line(&mut buf).await?;
//
//         if num_bytes == 0 {
//             return Err(Error::StreamClosed(format!(
//                 "Some kind of issue with reading bytes {}",
//                 &buf
//             )));
//         }
//
//         if !buf.is_empty() {
//             //@smells
//             buf = buf.trim().to_owned();
//
//             trace!("Received Message: {}", &buf);
//
//             if buf.is_empty() {
//                 continue;
//             }
//
//             let msg: Map<String, Value> = match serde_json::from_str(&buf) {
//                 Ok(msg) => msg,
//                 Err(_) => continue,
//             };
//
//             let method = if msg.contains_key("method") {
//                 match msg.get("method") {
//                     Some(method) => method.as_str(),
//                     //@todo need better stratum erroring here.
//                     None => return Err(Error::MethodDoesntExist),
//                 }
//             } else if msg.contains_key("messsage") {
//                 match msg.get("message") {
//                     Some(method) => method.as_str(),
//                     None => return Err(Error::MethodDoesntExist),
//                 }
//             } else if msg.contains_key("result") {
//                 Some("result")
//             } else {
//                 // return Err(Error::MethodDoesntExist);
//                 Some("")
//             };
//
//             if let Some(method_string) = method {
//                 //Mark the sender as active as we received a message.
//                 //We only mark them as active if the message/method was valid
//                 // self.stats.lock().await.last_active = Utc::now().naive_utc();
//                 // @todo maybe expose a function on the connection for this btw.
//
//                 return Ok((method_string.to_owned(), MessageValue::StratumV1(msg)));
//             }
//             //@todo improper format
//             return Err(Error::MethodDoesntExist);
//         };
//     }
// }
