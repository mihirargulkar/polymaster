use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const KALSHI_WS_URL: &str = "wss://api.elections.kalshi.com/trade-api/ws/v2";
const PING_INTERVAL: Duration = Duration::from_secs(10);
const RECONNECT_BASE: Duration = Duration::from_secs(2);
const RECONNECT_MAX: Duration = Duration::from_secs(60);

/// A trade received from the Kalshi WebSocket
#[derive(Debug, Clone)]
pub struct WsTrade {
    pub trade_id: String,
    pub ticker: String,
    pub count: i32,
    pub yes_price: f64,
    pub no_price: f64,
    pub taker_side: String,
    pub created_time: String,
}

#[derive(Debug, Deserialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    #[serde(default)]
    msg: Option<WsTradeMsg>,
}

#[derive(Debug, Deserialize)]
struct WsTradeMsg {
    #[serde(default)]
    trades: Vec<WsTradeEntry>,
}

#[derive(Debug, Deserialize)]
struct WsTradeEntry {
    trade_id: Option<String>,
    ticker: Option<String>,
    count: Option<i32>,
    yes_price: Option<f64>,
    no_price: Option<f64>,
    taker_side: Option<String>,
    created_time: Option<String>,
}

/// Subscribe command for Kalshi WebSocket
fn subscribe_cmd() -> String {
    serde_json::json!({
        "id": 1,
        "cmd": "subscribe",
        "params": {
            "channels": ["trade"]
        }
    })
    .to_string()
}

/// Spawn a Kalshi WebSocket listener that sends trades to the returned channel.
/// The connection auto-reconnects with exponential backoff on failure.
pub fn spawn_kalshi_ws() -> mpsc::UnboundedReceiver<WsTrade> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut backoff = RECONNECT_BASE;

        loop {
            match connect_and_listen(&tx).await {
                Ok(()) => {
                    // Clean disconnect — reconnect immediately
                    eprintln!("[WS] Kalshi WebSocket disconnected, reconnecting...");
                    backoff = RECONNECT_BASE;
                }
                Err(e) => {
                    eprintln!("[WS] Kalshi WebSocket error: {}, reconnecting in {:?}...", e, backoff);
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(RECONNECT_MAX);
                }
            }
        }
    });

    rx
}

async fn connect_and_listen(
    tx: &mpsc::UnboundedSender<WsTrade>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ws_stream, _) = connect_async(KALSHI_WS_URL).await?;
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to trade channel
    write.send(Message::Text(subscribe_cmd())).await?;

    // Ping loop
    let ping_tx = tx.clone();
    let ping_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(PING_INTERVAL);
        loop {
            interval.tick().await;
            if ping_tx.is_closed() {
                break;
            }
        }
    });

    // Also spawn a ping sender on the write half
    // We need to use a channel to coordinate writes
    let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Message>();

    // Spawn writer task
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = write_rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Spawn ping task
    let ping_write_tx = write_tx.clone();
    let ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(PING_INTERVAL);
        loop {
            interval.tick().await;
            if ping_write_tx.send(Message::Ping(vec![])).is_err() {
                break;
            }
        }
    });

    // Read loop
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Try to parse as trade message
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    if ws_msg.msg_type.as_deref() == Some("trade") {
                        if let Some(trade_msg) = ws_msg.msg {
                            for entry in trade_msg.trades {
                                if let Some(trade) = parse_ws_trade(entry) {
                                    if tx.send(trade).is_err() {
                                        // Receiver dropped
                                        ping_handle.abort();
                                        ping_task.abort();
                                        writer_handle.abort();
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
                // Also try flat trade format (some WS messages are different shape)
                else if let Ok(entry) = serde_json::from_str::<WsTradeEntry>(&text) {
                    if let Some(trade) = parse_ws_trade(entry) {
                        let _ = tx.send(trade);
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = write_tx.send(Message::Pong(data));
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                ping_handle.abort();
                ping_task.abort();
                writer_handle.abort();
                return Err(Box::new(e));
            }
            _ => {}
        }
    }

    ping_handle.abort();
    ping_task.abort();
    writer_handle.abort();
    Ok(())
}

fn parse_ws_trade(entry: WsTradeEntry) -> Option<WsTrade> {
    Some(WsTrade {
        trade_id: entry.trade_id?,
        ticker: entry.ticker?,
        count: entry.count.unwrap_or(1),
        yes_price: entry.yes_price.unwrap_or(0.0),
        no_price: entry.no_price.unwrap_or(0.0),
        taker_side: entry.taker_side.unwrap_or_else(|| "yes".to_string()),
        created_time: entry.created_time.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    })
}
