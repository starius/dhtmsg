# dhtmsg
Example of direct UDP connection using DHT to find each other

## Build

```
cargo build --release
```

if you are on Nix:

```
nix build
```

## Usage

You need two machines. Generate ID on each machine and exchange them. You can
just run the app, it will generate a random ID.

Let's assume you git IDs 11111111111111111111111111111111 and 22222222222222222222222222222222

Run on one machine:
```
dhtmsg --id 11111111111111111111111111111111 --peer 22222222222222222222222222222222
```

Run on another machine:
```
dhtmsg --id 22222222222222222222222222222222 --peer 11111111111111111111111111111111
```

If they connect, after some time you see messages like:
```
received hello from 1.2.3.4:56789: hello-ack from 11111111111111111111111111111111
```
