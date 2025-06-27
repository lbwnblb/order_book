use serde_json::json;
use tungstenite::{connect, Message, Utf8Bytes};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

/// 深度更新事件结构体，对应币安WebSocket深度更新消息
#[derive(Debug, Deserialize, Serialize)]
struct DepthUpdate {
    e: String,             // 事件类型
    E: u64,                // 事件时间
    s: String,             // 交易对
    U: u64,                // 从上次推送至今新增的第一个update Id
    u: u64,                // 从上次推送至今新增的最后一个update Id
    b: Vec<[String; 3]>,   // 变动的买单深度 [价格, 数量, 忽略]
    a: Vec<[String; 3]>,   // 变动的卖单深度 [价格, 数量, 忽略]
}

/// 深度快照结构体，对应币安REST API深度快照
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
    
    /// 应用深度更新到订单薄
    fn apply_depth_update(&mut self, update: &DepthUpdate) -> Result<(), Box<dyn Error>> {
        // 检查更新ID是否连续
        if update.U <= self.last_update_id + 1 && update.u >= self.last_update_id + 1 {
            // 更新买单
            for bid in &update.b {
                let price = bid[0].parse::<Decimal>()?;
                let quantity = bid[1].parse::<Decimal>()?;
                
                if quantity.is_zero() {
                    // 数量为0表示删除此价格的订单
                    self.bids.remove(&price);
                } else {
                    // 更新或添加此价格的订单
                    self.bids.insert(price, quantity);
                }
            }
            
            // 更新卖单
            for ask in &update.a {
                let price = ask[0].parse::<Decimal>()?;
                let quantity = ask[1].parse::<Decimal>()?;
                
                if quantity.is_zero() {
                    // 数量为0表示删除此价格的订单
                    self.asks.remove(&price);
                } else {
                    // 更新或添加此价格的订单
                    self.asks.insert(price, quantity);
                }
            }
            
            // 更新最后更新ID
            self.last_update_id = update.u;
            Ok(())
        } else {
            Err("深度更新ID不连续，需要重新获取快照".into())
        }
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

    // WebSocket深度更新示例（注释掉的代码）
    let subscribe = json!({
        "method": "SUBSCRIBE",
        "params": ["bnbusdt@depth@100ms"],
        "id": 1
    }).to_string();

    match connect("wss://stream.binance.com:9443/ws") {
        Ok((mut socket, response)) => {
            if response.status().as_u16() == 101 {
                // 订阅深度更新
                if let Ok(_) = socket.send(Message::Text(Utf8Bytes::from(subscribe))) {
                    loop {
                        match socket.read(){
                            Ok(Message::Text(msg)) => {
                               println!("收到消息: {}", msg);
                               // match serde_json::from_str::<DepthUpdate>(&msg) {
                               //     Ok(update) => {
                               //         println!("收到深度更新: 交易对 {}, 更新ID: {} - {}", update.s, update.U, update.u);
                               //         // 这里可以处理更新数据
                               //     },
                               //     Err(e) => {
                               //         println!("解析深度更新失败: {}", e);
                               //     }
                               // }
                            }
                            Err(e) => {
                                println!("读取WebSocket消息失败: {}", e);
                            }
                            _ => {}
                        };
                    }

                }
            } else {}
        },
        Err(e) => {
            println!("WebSocket连接失败: {}", e);
        }
    };
}
