use crate::protocol::{Action, Reply};
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{
  body::{Bytes, Incoming},
  server::conn::http1,
  upgrade::Upgraded,
  Request, Response,
};
use hyper_tungstenite::HyperWebsocket;
use hyper_util::rt::TokioIo;
use serde_json;
use std::net::SocketAddr;
use tokio::{
  net::TcpListener,
  sync::{mpsc, watch},
};
use tokio_tungstenite::{
  tungstenite::{Error, Message, Result},
  WebSocketStream,
};

/// Handle a HTTP or WebSocket request.
async fn handle_request(
  mut request: Request<Incoming>,
  peer: SocketAddr,
  action_tx: mpsc::Sender<Action>,
  reply_rx: watch::Receiver<Reply>,
) -> Result<Response<Full<Bytes>>, Error> {
  // Check if the request is a websocket upgrade request.
  if hyper_tungstenite::is_upgrade_request(&request) {
    let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;

    // Spawn a task to handle the websocket connection.
    tokio::spawn(async move {
      if let Err(e) = serve_websocket(websocket, peer, action_tx, reply_rx).await {
        eprintln!("Error in websocket connection: {e:?}");
      }
    });

    // Return the response so the spawned future can continue.
    Ok(response)
  } else {
    // Handle regular HTTP requests here.
    Ok(Response::new(Full::<Bytes>::from("Hello HTTP!")))
  }
}

async fn reply(ws: &mut WebSocketStream<TokioIo<Upgraded>>, reply: Reply) -> Result<(), ()> {
  match ws
    .send(Message::Text(match serde_json::to_string(&reply) {
      Ok(s) => s,
      Err(e) => {
        eprintln!("Error serializing reply: {e:?}");
        return Err(());
      }
    }))
    .await
  {
    Ok(_) => (),
    Err(e) => {
      eprintln!("Error sending reply: {e:?}");
      return Err(());
    }
  }
  Ok(())
}

async fn serve_websocket(
  ws: HyperWebsocket,
  peer: SocketAddr,
  action_tx: mpsc::Sender<Action>,
  mut reply_rx: watch::Receiver<Reply>,
) -> Result<()> {
  let mut ws = ws.await?;
  // let mut init_failed = false;

  println!("WebSocket Connected: {peer}");

  // send initial states
  if let Err(e) = action_tx.send(Action::Refresh).await {
    eprintln!("Error sending refresh action: {e:?}");
    println!("WebSocket Disconnected: {}", peer);
    return Ok(());
  }

  loop {
    tokio::select! {
      changed = reply_rx.changed() => {
        if let Err(e) = changed {
          eprintln!("Error receiving reply: {e:?}");
          break;
        }
        let rep = reply_rx.borrow_and_update().clone();
        if let Err(e) = reply(&mut ws, rep).await {
          eprintln!("Error sending reply: {e:?}");
          break;
        }
      }
      msg = ws.next() => {
        match msg {
          None => break,
          Some(msg) => match msg {
            Err(e) => {
              eprintln!("Error receiving message: {e:?}");
              break;
            }
            Ok(msg) => match msg {
              Message::Text(text) => {
                let action: Action = serde_json::from_str(&text).unwrap();
                action_tx.send(action).await.unwrap();
              }
              // other websocket message types are ignored
              _ => (),
            }
          }
        }
      }
    }
  }

  println!("WebSocket Disconnected: {}", peer);
  Ok(())
}

pub async fn start_server(
  addr: String,
  action_tx: mpsc::Sender<Action>,
  reply_rx: watch::Receiver<Reply>,
) {
  let listener = TcpListener::bind(&addr).await.expect("Can't listen");
  let mut http = http1::Builder::new();
  http.keep_alive(true);

  println!("Listening on: {addr}");

  loop {
    let (stream, peer) = listener.accept().await.expect("failed to accept");
    let action_tx = action_tx.clone();
    let reply_rx = reply_rx.clone();
    let connection = http
      .serve_connection(
        TokioIo::new(stream),
        hyper::service::service_fn(move |req| {
          handle_request(req, peer, action_tx.clone(), reply_rx.clone())
        }),
      )
      .with_upgrades();

    tokio::spawn(async move {
      if let Err(err) = connection.await {
        println!("Error serving HTTP connection: {err:?}");
      }
    });
  }
}
