//! Order lifecycle: submit, track, cancel.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum OrderState {
    Pending,
    Open { order_id: String },
    Filled { order_id: String, avg_price: f64 },
    Cancelled { order_id: String },
    Failed(String),
}

#[derive(Debug, Default)]
pub struct OrderManager {
    orders: HashMap<String, OrderState>,
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
        }
    }

    pub fn track(&mut self, signal_id: uuid::Uuid, state: OrderState) {
        self.orders.insert(signal_id.to_string(), state);
    }

    pub fn get(&self, signal_id: &uuid::Uuid) -> Option<&OrderState> {
        self.orders.get(&signal_id.to_string())
    }
}
