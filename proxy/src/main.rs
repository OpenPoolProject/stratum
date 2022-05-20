fn main() {
        let listening_host = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&listening_host).await?;
        let mut incoming = listener.incoming();

        info!("Listening on {}", listening_host);

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            //If Proxy === true, then we don't get this information from the stream....
            //We need to read a newline, and then parse the proxy info from that. AND Then
            //create a new miner from that information.
            //
            let mut buf = String::new();
            let (rh, wh) = stream.split();
            let mut buffer_stream = BufReader::new(rh);
            buffer_stream.read_line(&mut buf).await?;

            //Buf will be of the format "PROXY TCP4 92.118.161.17 172.20.42.228 55867 8080\r\n"
            //Trim the \r\n off
            let buf = buf.trim();
            //Might want to not be ascii whitespace and just normal here.
            // let pieces = buf.split_ascii_whitespace();

            let pieces: Vec<&str> = buf.split(' ').collect();
            let addr: SocketAddr = format!("{}:{}", pieces[2], pieces[5]).parse().unwrap();

            if addr.port() != self.config.port {
                continue;
            }

            task::spawn(async move {
                info!("Accepting stream from: {}", addr);

            //1. Make a connection to Stratum FROM the above information.
            //2. Pass in the first line of PROXy PROTOCOL
            //3. Loop, Read a line from the stream.
            //4. If strts with DISCONNECT then drop the connection.
            //5. othrwise forward the message upstream.
            //6. IF our connection to Stratum errors outs, instead of panicing on that thread.
            //7. We attempt to reconnect.

                info!("Closing stream from: {}", addr);
            });
        }
        Ok(())
}
