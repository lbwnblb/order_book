use serde_json::json;
use tungstenite::{connect, Message, Utf8Bytes};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
struct DepthSnapshot {
    lastUpdateId: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

/// 订单薄结构体，包含买单和卖单
#[derive(Debug)]
struct OrderBook {
    last_update_id: u64,
    /// 买单映射 (价格 -> 数量)
    bids: BTreeMap<Decimal, Decimal>,
    /// 卖单映射 (价格 -> 数量)
    asks: BTreeMap<Decimal, Decimal>,
}

impl OrderBook {
    /// 从深度快照创建订单薄
    fn from_snapshot(snapshot: DepthSnapshot) -> Result<Self, Box<dyn Error>> {
        // 创建BTreeMap用于买单和卖单
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        
        // 处理买单，转换字符串为Decimal并插入到映射中
        for bid in snapshot.bids {
            let price = bid[0].parse::<Decimal>()?;
            let quantity = bid[1].parse::<Decimal>()?;
            if !quantity.is_zero() {
                bids.insert(price, quantity);
            }
        }
        
        // 处理卖单，转换字符串为Decimal并插入到映射中
        for ask in snapshot.asks {
            let price = ask[0].parse::<Decimal>()?;
            let quantity = ask[1].parse::<Decimal>()?;
            if !quantity.is_zero() {
                asks.insert(price, quantity);
            }
        }
        
        // 创建订单薄实例
        let order_book = OrderBook {
            last_update_id: snapshot.lastUpdateId,
            bids,
            asks,
        };
        
        Ok(order_book)
    }
    
    /// 获取买单列表（按价格降序排列）
    fn bids_list(&self) -> Vec<(Decimal, Decimal)> {
        let mut bids: Vec<(Decimal, Decimal)> = self.bids.iter()
            .map(|(price, quantity)| (*price, *quantity))
            .collect();
        
        // 按价格降序排列
        bids.sort_by(|a, b| b.0.cmp(&a.0));
        bids
    }
    
    /// 获取卖单列表（按价格升序排列）
    fn asks_list(&self) -> Vec<(Decimal, Decimal)> {
        // BTreeMap已经按键升序排列，所以不需要额外排序
        self.asks.iter()
            .map(|(price, quantity)| (*price, *quantity))
            .collect()
    }
    
    /// 打印订单薄信息
    fn print_summary(&self, limit: usize) {
        println!("订单薄信息:");
        println!("最后更新 ID: {}", self.last_update_id);
        println!("买单数量: {}", self.bids.len());
        println!("卖单数量: {}", self.asks.len());
        
        // 打印前N个买单（价格降序）
        println!("\n前{}个买单 (价格降序):", limit);
        for (i, (price, quantity)) in self.bids_list().iter().take(limit).enumerate() {
            println!("{}. 价格: {}, 数量: {}", i+1, price, quantity);
        }
        
        // 打印前N个卖单（价格升序）
        println!("\n前{}个卖单 (价格升序):", limit);
        for (i, (price, quantity)) in self.asks_list().iter().take(limit).enumerate() {
            println!("{}. 价格: {}, 数量: {}", i+1, price, quantity);
        }
    }
    
    /// 获取最高买价
    fn best_bid(&self) -> Option<(Decimal, Decimal)> {
        self.bids.iter()
            .max_by(|a, b| a.0.cmp(b.0))
            .map(|(price, quantity)| (*price, *quantity))
    }
    
    /// 获取最低卖价
    fn best_ask(&self) -> Option<(Decimal, Decimal)> {
        self.asks.iter()
            .min_by(|a, b| a.0.cmp(b.0))
            .map(|(price, quantity)| (*price, *quantity))
    }
    
    /// 获取买卖价差
    fn spread(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid_price, _)), Some((ask_price, _))) => Some(ask_price - bid_price),
            _ => None,
        }
    }
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
    match get_depth_snapshot("BNBUSDT", 10) {
        Ok(snapshot) => {
            println!("深度快照获取成功!");
            
            // 转换为订单薄结构
            match OrderBook::from_snapshot(snapshot) {
                Ok(order_book) => {
                   let bids_list = order_book.bids_list();
                   let asks_list = order_book.asks_list();
                    for bl in bids_list {
                        println!("买单: {}, 数量: {}", bl.0, bl.1)
                    }
                    for al in asks_list {
                        println!("卖单: {}, 数量: {}", al.0, al.1)
                    }
                },
                Err(e) => {
                    println!("创建订单薄失败: {}", e);
                }
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
