# moba

A game I'm working on in Rust. There is no real gameplay plan.
This is mostly a proof-of-concept/learning experience.

## Running
```bash
$ git clone git@github.com:MovingtoMars/moba.git
$ cd moba
$ cargo build --release
```

Two binaries are build, `client` and `server`.

In one terminal, run `./target/release/server`.

In another terminal, run `./target/release/client -u "username"`.

By default, the client connects to localhost.

### SDL2

You may get better performance using the SDL2 backend.

```bash
$ cargo build --release --features sdl2
```
