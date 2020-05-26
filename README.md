Simple demonstration of how I am probably using the library wrong.

After building with `cargo build`, run as follows:

```
target/debug/cbor-socket-example serve ./mysocket
```

In another terminal, run:

```
target/debug/cbor-socket-example connect ./mysocket
```

The client will send a message to the server, which will receive an
incoming connection. However, it will not actually deserialize the
message until the client closes its connection. The server is blocking
on its read of the request sent from the client. The client blocks
when it tries to read the response from the server.
