use serde_json::json;
use tungstenite::{connect, Message, Utf8Bytes};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Deserialize, Serialize)]
struct DepthSnapshot {
    lastUpdateId: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

/// 获取币安交易所的深度快照数据
/// 
/// # 参数
/// 
/// * `symbol` - 交易对符号，例如 "BNBBTC"
/// * `limit` - 返回的深度级别，可选值：5, 10, 20, 50, 100, 500, 1000, 5000
/// 
/// # 返回值
/// 
/// 返回 Result，成功时包含 DepthSnapshot 结构体，失败时包含错误信息
fn get_depth_snapshot(symbol: &str, limit: u32) -> Result<DepthSnapshot, Box<dyn Error>> {
    let url = format!(
        "https://api.binance.com/api/v3/depth?symbol={}&limit={}",
        symbol, limit
    );
    
    println!("正在请求深度数据: {}", url);
    
    // 使用 reqwest 的阻塞客户端发送请求
    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send()?;
    
    if response.status().is_success() {
        let snapshot: DepthSnapshot = response.json()?;
        Ok(snapshot)
    } else {
        Err(format!("API 请求失败: {}", response.status()).into())
    }
}

fn main() {
    // 获取深度快照示例
    match get_depth_snapshot("BNBUSDT", 5000) {
        Ok(snapshot) => {
            println!("深度快照获取成功!");
            println!("最后更新 ID: {}", snapshot.lastUpdateId);
            println!("买单数量: {}", snapshot.bids.len());
            println!("卖单数量: {}", snapshot.asks.len());
            
            // 打印前5个买单
            println!("\n前5个买单:");
            for (i, bid) in snapshot.bids.iter().take(5).enumerate() {
                println!("{}. 价格: {}, 数量: {}", i+1, bid[0], bid[1]);
            }
            
            // 打印前5个卖单
            println!("\n前5个卖单:");
            for (i, ask) in snapshot.asks.iter().take(5).enumerate() {
                println!("{}. 价格: {}, 数量: {}", i+1, ask[0], ask[1]);
            }
        },
        Err(e) => {
            println!("获取深度快照失败: {}", e);
        }
    }

    // let subscribe = json!({
    //             "method": "SUBSCRIBE",
    //             "params": ["btcusdt@trade"],
    //             "id": 1
    //         }).to_string();
    //
    //
    // match connect("wss://stream.binance.com:9443/ws"){
    //     Ok((mut socket, response)) => {
    //         match response.status().as_u16() {
    //             101 => {
    //                 match socket.send(Message::Text(Utf8Bytes::from(subscribe))){
    //                     Ok(_) => {
    //                         loop {
    //                             match socket.read() {
    //                                 Ok(Message::Text(text)) => {
    //                                     println!("{}", text);
    //                                 },
    //                                 Err(_) => {}
    //                                 _ => {}
    //                             }
    //                         }
    //                     }
    //                     Err(_) => {}
    //                 }
    //
    //             },
    //             _ => {
    //                 println!("连接异常{:?}", response);
    //             }
    //         }
    //
    //
    //
    //     },
    //     Err(e) => {
    //         println!("连接异常: {}", e);
    //     }
    // };
}
