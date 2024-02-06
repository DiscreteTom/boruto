# boruto -- 脖·人·转

![license](https://img.shields.io/github/license/DiscreteTom/boruto?style=flat-square)

Control your PC windows with your neck.

![demo](./img/demo.gif)

> [!WARNING]
> This project is useless and just for fun, unless you have a very stable neck.

## How to Use

This project consists of 3 parts:

- A WebSocket server written in Rust, which is responsible to move windows.
  - Use `cargo run` to start the server.
- A web page written in Vue3, which is responsible to select which windows are captured, and enable the control.
  - Use `cd web && yarn && yarn dev` to start the web server.
- An Android app to analyze your face in realtime (30+fps in my phone), and send the angle info to the server.
  - Use Android Studio to open the project (under `app/`) and run it on your phone.
