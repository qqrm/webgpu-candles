use crate::domain::market_data::{Candle, Symbol, TimeInterval};
use std::fmt::Debug;

/// Base trait for all domain events
pub trait DomainEvent: Debug + Clone {
    fn event_type(&self) -> &'static str;
    fn timestamp(&self) -> u64 {
        use crate::domain::logging::get_time_provider;
        get_time_provider().current_timestamp()
    }
}

/// Events related to market data
#[derive(Debug, Clone)]
pub enum MarketDataEvent {
    NewCandleReceived {
        symbol: Symbol,
        interval: TimeInterval,
        candle: Candle,
    },
    HistoricalDataLoaded {
        symbol: Symbol,
        interval: TimeInterval,
        candle_count: usize,
    },
    DataValidationFailed {
        symbol: Symbol,
        reason: String,
    },
    MarketDataConnectionEstablished {
        symbol: Symbol,
        interval: TimeInterval,
    },
    MarketDataConnectionLost {
        symbol: Symbol,
        reason: String,
    },
}

impl DomainEvent for MarketDataEvent {
    fn event_type(&self) -> &'static str {
        match self {
            MarketDataEvent::NewCandleReceived { .. } => "NewCandleReceived",
            MarketDataEvent::HistoricalDataLoaded { .. } => "HistoricalDataLoaded",
            MarketDataEvent::DataValidationFailed { .. } => "DataValidationFailed",
            MarketDataEvent::MarketDataConnectionEstablished { .. } => "MarketDataConnectionEstablished",
            MarketDataEvent::MarketDataConnectionLost { .. } => "MarketDataConnectionLost",
        }
    }
}

/// Events related to chart
#[derive(Debug, Clone)]
pub enum ChartEvent {
    ChartDataUpdated {
        chart_id: String,
        candle_count: usize,
    },
    ViewportChanged {
        chart_id: String,
        old_range: (f32, f32),
        new_range: (f32, f32),
    },
    ChartRenderingRequested {
        chart_id: String,
    },
}

impl DomainEvent for ChartEvent {
    fn event_type(&self) -> &'static str {
        match self {
            ChartEvent::ChartDataUpdated { .. } => "ChartDataUpdated",
            ChartEvent::ViewportChanged { .. } => "ViewportChanged",
            ChartEvent::ChartRenderingRequested { .. } => "ChartRenderingRequested",
        }
    }
}

/// Event dispatcher for publishing events
pub trait EventDispatcher {
    fn publish_market_data_event(&self, event: MarketDataEvent);
    fn publish_chart_event(&self, event: ChartEvent);
}

/// Simple in-memory event dispatcher
pub struct InMemoryEventDispatcher {
    market_data_handlers: Vec<Box<dyn Fn(&MarketDataEvent)>>,
    chart_handlers: Vec<Box<dyn Fn(&ChartEvent)>>,
}

impl InMemoryEventDispatcher {
    pub fn new() -> Self {
        Self {
            market_data_handlers: Vec::new(),
            chart_handlers: Vec::new(),
        }
    }

    pub fn subscribe_to_market_data_events<F>(&mut self, handler: F)
    where
        F: Fn(&MarketDataEvent) + 'static,
    {
        self.market_data_handlers.push(Box::new(handler));
    }

    pub fn subscribe_to_chart_events<F>(&mut self, handler: F)
    where
        F: Fn(&ChartEvent) + 'static,
    {
        self.chart_handlers.push(Box::new(handler));
    }
}

impl EventDispatcher for InMemoryEventDispatcher {
    fn publish_market_data_event(&self, event: MarketDataEvent) {
        for handler in &self.market_data_handlers {
            handler(&event);
        }
    }

    fn publish_chart_event(&self, event: ChartEvent) {
        for handler in &self.chart_handlers {
            handler(&event);
        }
    }
} 